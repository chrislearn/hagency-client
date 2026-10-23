//! Main-thread Sparkle integration for validated product and isolated test bundles.
mod config;
use config::{BuildTrust, Config};
use super::{Phase, Snapshot, UpdateAction, UpdateState};
use makepad_widgets::*;
use std::{
    ffi::{CStr, c_char, c_void},
    marker::PhantomData,
    rc::Rc,
    sync::OnceLock,
};

unsafe extern "C" {
    fn ha_sparkle_config() -> *mut c_char;
    fn ha_sparkle_create(legacy_auto_check: i32, wake: extern "C" fn()) -> *mut c_void;
    fn ha_sparkle_migration_needed() -> bool;
    fn ha_sparkle_set_auto_check(handle: *mut c_void, value: bool) -> bool;
    fn ha_sparkle_can_check(handle: *mut c_void) -> bool;
    fn ha_sparkle_next_event(handle: *mut c_void) -> *mut c_char;
    fn ha_sparkle_action(handle: *mut c_void, action: u32, cycle: u64) -> bool;
    fn ha_sparkle_free_string(value: *mut c_char);
    fn ha_sparkle_destroy(handle: *mut c_void);
}

static CONFIG: OnceLock<Config> = OnceLock::new();
static CONFIG_ERROR: OnceLock<&'static str> = OnceLock::new();
extern "C" fn wake_ui() {
    SignalToUI::set_ui_signal();
}
// Kept until process exit, including App destruction before native termination.
static EXIT_FENCE: std::sync::Mutex<Option<hagency::update_guard::InstallGuard>> =
    std::sync::Mutex::new(None);

fn owned_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the bridge returns an owned, NUL-terminated UTF-8 allocation.
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ha_sparkle_free_string(ptr) };
    Some(value)
}

fn validate(config: &Config) -> anyhow::Result<()> {
    anyhow::ensure!(
        config.automatic_updates_allowed == Some(false)
            && config.automatically_update == Some(false),
        "Prototype must not enable unattended installation"
    );
    anyhow::ensure!(
        cfg!(debug_assertions),
        "Sparkle prototype requires a debug build"
    );
    anyhow::ensure!(
        config.root.is_absolute() && config.root.canonicalize()? == config.root,
        "Prototype root must be canonical"
    );
    anyhow::ensure!(
        std::fs::read(config.root.join("prototype-marker"))? == b"hagency-sparkle-prototype-v1\n",
        "Missing prototype marker"
    );
    anyhow::ensure!(
        config.bundle == config.root.join("installed/Hagency Sparkle Prototype.app"),
        "Refusing a bundle outside the harness installation"
    );
    config.validate_executable(&std::env::current_exe()?)?;
    let url: url::Url = config.feed.parse()?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none(),
        "Prototype feed must be explicit IPv4 loopback"
    );
    Ok(())
}

/// Configure the isolated profile before Makepad or background runtimes start.
///
/// # Safety
/// Call once on the main thread, before creating any threads (environment mutation).
pub unsafe fn configure() -> anyhow::Result<bool> {
    let Some(raw) = owned_string(unsafe { ha_sparkle_config() }) else {
        return Ok(false);
    };
    let config: Config = serde_json::from_str(&raw)?;
    if !config.prototype() {
        let trust = BuildTrust {
            packaged_release: cfg!(packaging_build)
                && !cfg!(debug_assertions)
                && cfg!(feature = "standalone")
                && !cfg!(test),
            public_key: option_env!("HAGENCY_SPARKLE_PUBLIC_KEY").unwrap_or_default(),
            team_id: option_env!("HAGENCY_SPARKLE_TEAM_ID").unwrap_or_default(),
            arch: std::env::consts::ARCH,
            version: env!("CARGO_PKG_VERSION"),
        };
        if let Err(error) = config
            .validate_product(&trust)
            .and_then(|()| config.validate_executable(&std::env::current_exe()?))
        {
            eprintln!("Sparkle product configuration refused: {error:#}");
            let _ = CONFIG_ERROR.set("Update signing is not configured for this build.");
            return Ok(false);
        }
        anyhow::ensure!(CONFIG.set(config).is_ok(), "Sparkle configured twice");
        return Ok(true);
    }
    validate(&config)?;
    let data = config.root.join("data");
    std::fs::create_dir_all(&data)?;
    // SAFETY: upheld by the entry point, before app_main and any worker creation.
    unsafe {
        std::env::set_var("HAGENCY_APP_DATA_DIR", &data);
    }
    anyhow::ensure!(CONFIG.set(config).is_ok(), "Prototype configured twice");
    record(
        "launch",
        serde_json::json!({"version": CONFIG.get().unwrap().version, "executable": std::env::current_exe()?, "data": data}),
    );
    Ok(true)
}

pub fn prototype() -> bool {
    CONFIG.get().is_some_and(Config::prototype)
}

pub(super) fn configuration_error() -> Option<&'static str> {
    CONFIG_ERROR.get().copied()
}

pub fn active() -> bool {
    CONFIG.get().is_some()
}

pub(crate) fn configure_resources(vm: &mut ScriptVm) {
    if let Some(config) = CONFIG.get() {
        // The relaunched process must not read removable-volume development paths.
        let resources = config.bundle.join("Contents/Resources");
        vm.with_cx_mut(|cx| cx.package_root = Some(resources.to_string_lossy().into_owned()));
        record("packaged_resources", serde_json::json!({"root": resources}));
    }
}

fn record(kind: &str, detail: serde_json::Value) {
    if let Some(config) = CONFIG.get().filter(|config| config.prototype()) {
        use std::io::Write;
        let event = serde_json::json!({"kind": kind, "pid": std::process::id(), "version": config.version, "detail": detail});
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(config.root.join("events.jsonl"))
        {
            let _ = writeln!(file, "{event}");
            let _ = file.sync_all();
        }
    }
}

pub(super) struct Runtime {
    native: *mut c_void,
    _main_thread: PhantomData<Rc<()>>,
    state: Snapshot,
    cycle: u64,
    requested: bool,
    guard: Option<hagency::update_guard::InstallGuard>,
    pending: Option<hagency::update_guard::PendingInstall>,
    preparation: Option<super::preparation::Preparation>,
    account: Option<(u64, Option<String>)>,
    dismissed: bool,
    pub(super) needs_preparation: bool,
    allow_exit: bool,
    terminal_error: bool,
    cancel_requested: bool,
    held_test_task: Option<hagency::update_guard::TaskGuard>,
}

impl Runtime {
    pub(super) fn start() -> anyhow::Result<Self> {
        anyhow::ensure!(active(), "No validated Sparkle configuration");
        let legacy_auto_check = if unsafe { ha_sparkle_migration_needed() } {
            i32::from(
                hagency_updater::Preferences::load(&crate::app_data_dir().join("updates/macos"))?
                    .auto_check,
            )
        } else {
            -1
        };
        // SAFETY: Integration starts/polls/actions/drops this non-Send owner on the UI thread.
        let native = unsafe { ha_sparkle_create(legacy_auto_check, wake_ui) };
        anyhow::ensure!(
            !native.is_null(),
            "Could not initialize Sparkle preferences or updater"
        );
        record(
            "bridge_started",
            serde_json::json!({"main_thread": true, "idle_poll": prototype() && !CONFIG.get().unwrap().signal_only}),
        );
        Ok(Self {
            native,
            _main_thread: PhantomData,
            state: Snapshot {
                enabled: true,
                auto_check: false,
                auto_download: false,
                phase: Phase::Idle,
                notes: if prototype() {
                    "Isolated Sparkle prototype. No production account or application is updated."
                        .into()
                } else {
                    String::new()
                },
                ..Default::default()
            },
            cycle: 0,
            requested: false,
            guard: None,
            pending: None,
            preparation: None,
            account: None,
            dismissed: false,
            needs_preparation: false,
            allow_exit: false,
            terminal_error: false,
            cancel_requested: false,
            held_test_task: None,
        })
    }

    fn command(&mut self, action: u32) -> bool {
        // SAFETY: handle and copied blocks are owned by this main-thread Runtime.
        unsafe { ha_sparkle_action(self.native, action, self.cycle) }
    }

    pub(super) fn needs_poll(&self) -> bool {
        (prototype() && !CONFIG.get().unwrap().signal_only)
            || self.pending.is_some()
            || self.needs_preparation
            || self
                .preparation
                .as_ref()
                .is_some_and(|preparation| preparation.result().is_none())
    }

    pub(super) fn action(&mut self, action: UpdateAction) {
        match action {
            UpdateAction::Check
                if matches!(
                    self.state.phase,
                    Phase::Idle | Phase::Current | Phase::Failed
                ) =>
            {
                if self.preparation.is_some() {
                    return;
                }
                if self.command(1) {
                    self.terminal_error = false;
                    self.state.phase = Phase::Checking;
                }
            }
            UpdateAction::Download if self.state.phase == Phase::Available => {
                self.pending = hagency::update_guard::PROCESS.begin_update();
                if self.pending.is_none() {
                    self.fail("Agent cleanup is unconfirmed. Installation has been postponed.");
                    return;
                }
                self.requested = true;
                self.dismissed = false;
                self.cancel_requested = false;
                self.state.phase = Phase::Waiting;
                record(
                    "user_authorized_update",
                    serde_json::json!({"cycle": self.cycle}),
                );
            }
            UpdateAction::Later if self.state.phase == Phase::Waiting && self.guard.is_none() => {
                self.requested = false;
                self.pending.take();
                self.state.phase = Phase::Available;
                record("waiting_canceled", serde_json::json!({}));
            }
            UpdateAction::AutoCheck(value) => {
                if !unsafe { ha_sparkle_set_auto_check(self.native, value) } {
                    self.fail("Could not save update preferences. Please try again.");
                }
            }
            UpdateAction::Later
                if matches!(
                    self.state.phase,
                    Phase::Checking
                        | Phase::Available
                        | Phase::Downloading
                        | Phase::Verifying
                        | Phase::Ready
                ) =>
            {
                self.cancel_requested = true;
                self.requested = false;
                self.needs_preparation = false;
                if self.preparation.is_some() {
                    self.fail("The update was canceled after shutdown started. Restart Hagency before updating.");
                } else {
                    self.state.phase = Phase::Canceling;
                    let _ = self.command(4); // During extraction, wait for Ready then Skip.
                }
            }
            _ => {}
        }
    }

    fn fail(&mut self, message: &'static str) {
        self.terminal_error = true;
        self.requested = false;
        self.needs_preparation = false;
        self.state.phase = Phase::Failed;
        self.state.restart_required = self.preparation.is_some();
        self.state.error = message;
        // Do not release admission until Sparkle acknowledges dismissing its installation.
        let _ = self.command(4);
    }

    pub(super) fn poll(&mut self, cx: &mut Cx) {
        // Explicit harness commands exercise the same actions as the Makepad buttons.
        // The production identity cannot initialize CONFIG or this runtime.
        let command_path = CONFIG.get().unwrap().root.join("action.json");
        if prototype()
            && let Ok(bytes) = std::fs::read(&command_path)
        {
            let _ = std::fs::remove_file(&command_path);
            if let Ok(command) = serde_json::from_slice::<serde_json::Value>(&bytes)
                && command["pid"].as_u64() == Some(u64::from(std::process::id()))
                && command["cycle"].as_u64() == Some(self.cycle)
            {
                match command["action"].as_str() {
                    Some("check") => cx.action(UpdateAction::Check),
                    Some("update") => cx.action(UpdateAction::Download),
                    Some("later") => cx.action(UpdateAction::Later),
                    Some("auto_check_on") => cx.action(UpdateAction::AutoCheck(true)),
                    Some("auto_check_off") => cx.action(UpdateAction::AutoCheck(false)),
                    Some("request_quit") => {
                        let _ = self.command(6);
                    }
                    Some("hold_task") => {
                        self.held_test_task = hagency::update_guard::PROCESS.enter_task();
                        record("test_task_held", serde_json::json!({}));
                    }
                    Some("release_task") => {
                        self.held_test_task.take();
                        record("test_task_released", serde_json::json!({}));
                    }
                    _ => {}
                }
            }
        }
        while let Some(raw) = owned_string(unsafe { ha_sparkle_next_event(self.native) }) {
            let event: serde_json::Value = serde_json::from_str(&raw).expect("bridge event JSON");
            record("sparkle", event.clone());
            let cycle = event["cycle"].as_u64().unwrap_or(self.cycle);
            if cycle > self.cycle
                && self.preparation.is_none()
                && self.guard.is_none()
                && self.pending.is_none()
            {
                self.terminal_error = false;
                self.cancel_requested = false;
            }
            self.cycle = cycle;
            match event["kind"].as_str().unwrap_or("") {
                "checking" => {
                    self.state.phase = Phase::Checking;
                    self.state.progress = 0;
                    self.state.version.clear();
                    self.state.notes.clear();
                }
                "available" => {
                    self.state.notes = event["notes"].as_str().unwrap_or_default().into();
                    self.state.phase = Phase::Available;
                    self.state.version = event["version"].as_str().unwrap_or_default().into();
                }
                "preferences" => {
                    self.state.auto_check = event["auto_check"].as_bool().unwrap_or(false)
                }
                "notes" => self.state.notes = event["text"].as_str().unwrap_or_default().into(),
                "current" => self.state.phase = Phase::Current,
                "cycle_finished" if self.state.phase == Phase::Checking => {
                    self.state.phase = Phase::Idle
                }
                "downloading" => self.state.phase = Phase::Downloading,
                "progress" => {
                    let expected = event["expected"].as_u64().unwrap_or(0);
                    self.state.progress = event["received"]
                        .as_u64()
                        .unwrap_or(0)
                        .saturating_mul(100)
                        .checked_div(expected)
                        .unwrap_or(0)
                        .min(100);
                }
                "extracting" if !self.cancel_requested => self.state.phase = Phase::Verifying,
                "ready" => {
                    if self.terminal_error
                        || self.cancel_requested
                        || self.guard.is_none()
                        || !self.requested
                    {
                        let _ = self.command(4);
                    } else {
                        self.state.phase = Phase::Ready;
                        self.needs_preparation = true;
                    }
                }
                "error" if self.cancel_requested => {}
                "error" => self
                    .fail("Could not download or verify the update. Check for updates to retry."),
                "dismissed" if !self.allow_exit => {
                    self.dismissed = true;
                    if self.cancel_requested {
                        record(
                            "installation_canceled",
                            serde_json::json!({"cycle": self.cycle}),
                        );
                        self.cancel_requested = false;
                    }
                    if self.preparation.is_none() {
                        self.guard.take();
                        self.pending.take();
                        record("admission_released", serde_json::json!({}));
                    }
                    self.requested = false;
                    if !self.terminal_error && self.state.phase != Phase::Current {
                        self.state.phase = Phase::Idle;
                    }
                }
                _ => {}
            }
        }
        if self.requested
            && self.guard.is_none()
            && self.state.phase == Phase::Waiting
            && let Some(guard) = self
                .pending
                .as_mut()
                .and_then(|pending| pending.try_install())
        {
            self.guard = Some(guard);
            self.pending.take();
            record("admission_fenced", serde_json::json!({}));
            if !self.command(2) {
                self.fail("Could not start the update installer. Please try again.");
            }
        }
        if let Some(preparation) = &self.preparation {
            match preparation.result() {
                Some(Ok(())) if !self.terminal_error && !self.allow_exit => {
                    self.needs_preparation = true
                }
                Some(Err(error)) if !self.terminal_error => {
                    record(
                        "preparation_failed",
                        serde_json::json!({"stage": error.to_string()}),
                    );
                    hagency::update_guard::PROCESS.block_install();
                    self.fail("Shutdown could not be confirmed. Installation was postponed.");
                }
                None if preparation.overdue() && !self.terminal_error => {
                    record(
                        "preparation_timed_out",
                        serde_json::json!({"original_cleanup_retained": true}),
                    );
                    self.fail("Shutdown is still pending. Installation was postponed; waiting for the original cleanup.");
                }
                _ => {}
            }
        }
        self.state.can_check =
            self.preparation.is_none() && unsafe { ha_sparkle_can_check(self.native) };
        if cx.global::<UpdateState>().0 != self.state {
            cx.global::<UpdateState>().0 = self.state.clone();
            cx.redraw_all();
        }
    }

    pub(super) fn blocks_quit(&self) -> bool {
        let blocked = if self.allow_exit {
            false
        } else if let Some(preparation) = &self.preparation {
            // Only a positive receipt AND dismissal permit ordinary quit. Keep
            // admission fenced after retirement, including a UI deadline.
            !(self.terminal_error && self.dismissed && preparation.result() == Some(Ok(())))
        } else {
            self.pending.is_some() || self.guard.is_some()
        };
        if blocked {
            record("quit_blocked", serde_json::json!({"cycle": self.cycle}));
        }
        blocked
    }

    pub(super) fn prepared(&mut self, ok: bool) {
        if !self.needs_preparation {
            return;
        }
        self.needs_preparation = false;
        if !ok {
            self.fail("Could not save your work. The update was postponed.");
            return;
        }
        if self.preparation.is_none() {
            if crate::octos_service::ready_for_update().is_err() {
                self.fail(
                    "The assistant cannot confirm shutdown. Restart Hagency before updating.",
                );
                return;
            }
            let (epoch, user) = account();
            // Account-job quiescence closes precisely one scope generation.
            self.account = Some((epoch.wrapping_add(1), user));
            self.state.phase = Phase::Preparing;
            self.preparation = Some(super::preparation::Preparation::start());
            record(
                "preparation_started",
                serde_json::json!({"owner": true, "matrix": true}),
            );
            return;
        }
        // A second save is requested only AFTER the retained real cleanup reports
        // success. A raw UI boolean can never substitute for this receipt.
        if self.preparation.as_ref().unwrap().result() != Some(Ok(())) || self.terminal_error {
            return;
        }
        if !self
            .preparation
            .as_ref()
            .unwrap()
            .can_install(self.account.as_ref() == Some(&account()), ok)
        {
            self.fail("Your account changed during preparation. Installation was postponed.");
            return;
        }
        record(
            "preparation_succeeded",
            serde_json::json!({"owner_stopped": true, "matrix_stopped": true, "state_saved": true}),
        );
        self.state.phase = Phase::Preparing;
        self.allow_exit = true;
        *EXIT_FENCE.lock().unwrap() = self.guard.take();
        if !self.command(3) {
            self.allow_exit = false;
            self.guard = EXIT_FENCE.lock().unwrap().take();
            self.fail("Could not start the update installer. Please restart Hagency to try again.");
        }
    }
}

fn account() -> (u64, Option<String>) {
    (
        crate::account_session::epoch(),
        crate::sliding_sync::current_user_id().map(|id| id.to_string()),
    )
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if self.preparation.is_some() && self.guard.is_some() {
            *EXIT_FENCE.lock().unwrap() = self.guard.take();
        }
        // SAFETY: Runtime is non-Send; all ObjC references are released on its main thread.
        unsafe {
            ha_sparkle_destroy(self.native);
        }
    }
}
