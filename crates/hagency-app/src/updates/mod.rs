//! Native update notifications and the Windows installation lifecycle.
use makepad_widgets::*;

#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", not(target_env = "ohos"))
))]
fn sparkle_configuration_failed() -> bool {
    #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
    {
        sparkle::configuration_error().is_some()
    }
    #[cfg(not(all(target_os = "macos", feature = "macos-sparkle")))]
    false
}

fn sparkle_active() -> bool {
    #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
    {
        sparkle::active()
    }
    #[cfg(not(all(target_os = "macos", feature = "macos-sparkle")))]
    false
}

#[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
pub mod sparkle;
#[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
mod preparation;

#[cfg(target_os = "windows")]
static EXIT_FENCE: std::sync::Mutex<Option<hagency::update_guard::InstallGuard>> =
    std::sync::Mutex::new(None);

mod ui;
mod titlebar;
pub(crate) use titlebar::titlebar_notice_visible;
pub fn script_mod(vm: &mut ScriptVm) {
    ui::script_mod(vm);
    titlebar::script_mod(vm);
}
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod worker;
#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", not(target_env = "ohos"))
))]
mod desktop;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Disabled,
    Idle,
    Current,
    Checking,
    Available,
    Downloading,
    Ready,
    Verifying,
    Waiting,
    Preparing,
    Canceling,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub enabled: bool,
    /// Explicit, isolated development preview; never authorizes update actions.
    pub preview: bool,
    /// Cleanup has retired this process; a fresh launch is required to retry.
    pub restart_required: bool,
    pub auto_check: bool,
    pub auto_download: bool,
    pub phase: Phase,
    pub version: String,
    pub notes: String,
    pub progress: u64,
    pub can_check: bool,
    pub error: &'static str,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            preview: false,
            restart_required: false,
            auto_check: true,
            auto_download: true,
            phase: Phase::Disabled,
            version: String::new(),
            notes: String::new(),
            progress: 0,
            can_check: true,
            error: "Updates are available in official desktop installers.",
        }
    }
}
impl Snapshot {
    pub fn status(&self) -> &'static str {
        if self.preview {
            return "Update reminder preview. Downloads and installation are disabled.";
        }
        match self.phase {
            Phase::Disabled | Phase::Failed => self.error,
            Phase::Idle => "Check for a new version of Hagency.",
            Phase::Current => "Hagency is up to date.",
            Phase::Checking => "Checking for updates…",
            Phase::Available if cfg!(target_os = "macos") && !sparkle_active() => {
                "A new version is available. Download the DMG and replace Hagency when you are ready."
            }
            Phase::Available if cfg!(target_os = "linux") => {
                "A new version is available. Open the download page to install it manually."
            }
            Phase::Available => "A new version is available.",
            Phase::Downloading => "Downloading update…",
            Phase::Ready => "Update downloaded. Restart when you are ready.",
            Phase::Verifying => "Verifying the downloaded update…",
            Phase::Waiting => {
                "Waiting for Agent tasks to finish. Installation will start when Hagency is idle."
            }
            Phase::Preparing => "Saving your work and preparing to restart…",
            Phase::Canceling => "Canceling update…",
        }
    }
}

#[derive(Default)]
pub struct UpdateState(pub Snapshot);

#[derive(Clone, Copy, Debug)]
pub enum UpdateAction {
    Check,
    Download,
    Install,
    Later,
    AutoCheck(bool),
    AutoDownload(bool),
}

#[derive(Default)]
pub struct Integration {
    initialized: bool,
    exit_requested: bool,
    #[cfg(target_os = "windows")]
    worker: Option<worker::Worker>,
    #[cfg(any(
        target_os = "macos",
        all(target_os = "linux", not(target_env = "ohos"))
    ))]
    desktop_worker: Option<desktop::Worker>,
    #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
    sparkle: Option<sparkle::Runtime>,
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        all(target_os = "linux", not(target_env = "ohos"))
    ))]
    timer: Timer,
    #[cfg(target_os = "windows")]
    install_guard: Option<hagency::update_guard::InstallGuard>,
    #[cfg(target_os = "windows")]
    preparation: Option<(std::sync::mpsc::Receiver<()>, std::time::Instant, bool)>,
    #[cfg(target_os = "windows")]
    helper: Option<windows::Helper>,
}

impl Integration {
    /// Returns true only after task admission is fenced and the owner has joined.
    pub fn handle_event(&mut self, cx: &mut Cx, event: &Event) -> bool {
        if self.exit_requested {
            return false;
        }
        if !self.initialized {
            self.initialized = true;
            #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
            if sparkle::active() && !cfg!(test) && cfg!(feature = "standalone") {
                match sparkle::Runtime::start() {
                    Ok(runtime) => {
                        self.sparkle = Some(runtime);
                        if self
                            .sparkle
                            .as_ref()
                            .is_some_and(|runtime| runtime.needs_poll())
                        {
                            self.timer = cx.start_interval(0.25);
                        }
                    }
                    Err(error) => {
                        error!("Sparkle initialization failed: {error:#}");
                        cx.global::<UpdateState>().0.error =
                            "Could not initialize updates. Please restart Hagency to try again.";
                    }
                }
            }
            #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
            if let Some(error) = sparkle::configuration_error() {
                cx.global::<UpdateState>().0.error = error;
            }
            #[cfg(debug_assertions)]
            if std::env::var("HAGENCY_PREVIEW_UPDATE").as_deref() == Ok("1")
                && std::env::var_os("HAGENCY_APP_DATA_DIR").is_some_and(|path| !path.is_empty())
            {
                cx.global::<UpdateState>().0 = Snapshot {
                    enabled: true,
                    preview: true,
                    phase: Phase::Available,
                    version: crate::i18n::tr("Preview version").to_owned(),
                    auto_check: false,
                    auto_download: false,
                    ..Default::default()
                };
                return false;
            }
            #[cfg(target_os = "windows")]
            if !cfg!(test) && cfg!(feature = "standalone") && cfg!(packaging_build) {
                match worker::Worker::start() {
                    Ok(worker) => {
                        self.worker = Some(worker);
                        self.timer = cx.start_interval(1.0);
                    }
                    Err(_) => {
                        cx.global::<UpdateState>().0.error =
                            "Update signing is not configured for this build."
                    }
                }
            }
            #[cfg(any(
                target_os = "macos",
                all(target_os = "linux", not(target_env = "ohos"))
            ))]
            if !cfg!(test)
                && cfg!(feature = "standalone")
                && !sparkle_active()
                && !sparkle_configuration_failed()
            {
                match desktop::Worker::start() {
                    Ok(worker) => {
                        self.desktop_worker = Some(worker);
                        self.timer = cx.start_interval(1.0);
                    }
                    Err(_) => {
                        cx.global::<UpdateState>().0.error =
                            "Update checks are unavailable in this build."
                    }
                }
            }
        }
        #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
        if let Some(runtime) = &mut self.sparkle {
            if let Event::Actions(actions) = event {
                for action in actions {
                    if let Some(action) = action.downcast_ref::<UpdateAction>() {
                        runtime.action(*action);
                    }
                }
            }
            if self.timer.is_event(event).is_some()
                || matches!(event, Event::Signal | Event::Actions(_))
            {
                runtime.poll(cx);
            }
            if runtime.needs_poll() && self.timer.is_empty() {
                self.timer = cx.start_interval(0.25);
            } else if !runtime.needs_poll() && !self.timer.is_empty() {
                cx.stop_timer(self.timer);
                self.timer = Timer::empty();
            }
        }
        #[cfg(any(
            target_os = "macos",
            all(target_os = "linux", not(target_env = "ohos"))
        ))]
        if let Some(worker) = &self.desktop_worker {
            if let Event::Actions(actions) = event {
                for action in actions {
                    if let Some(action) = action.downcast_ref::<UpdateAction>() {
                        worker.action(*action);
                    }
                }
            }
            if self.timer.is_event(event).is_some()
                || matches!(event, Event::Signal | Event::Actions(_))
            {
                worker.publish(cx);
            }
        }
        #[cfg(target_os = "windows")]
        if let Some(worker) = &self.worker {
            if let Event::Actions(actions) = event {
                for action in actions {
                    if let Some(action) = action.downcast_ref::<UpdateAction>() {
                        worker.action(*action);
                    }
                }
            }
            if self.timer.is_event(event).is_some()
                || matches!(event, Event::Signal | Event::Actions(_))
            {
                worker.publish(cx);
                if self.preparation.is_none()
                    && let Some(helper) = self.helper.as_mut()
                {
                    if helper.failed() {
                        self.helper.take();
                        self.install_guard.take();
                        worker.fail("Could not start the update installer. Please try again.");
                    } else if helper.ready() {
                        let (sender, receiver) = std::sync::mpsc::channel();
                        self.preparation = Some((receiver, std::time::Instant::now(), false));
                        std::thread::spawn(move || {
                            let result = crate::sliding_sync::block_on_async_with_timeout(
                                None,
                                crate::hagency::backend::Service::stop_current(),
                            );
                            if result.is_ok() {
                                let _ = sender.send(());
                            }
                        });
                    }
                }
                if let Some((receiver, started, canceled)) = &mut self.preparation {
                    if !*canceled && started.elapsed() > std::time::Duration::from_secs(15) {
                        *canceled = true;
                        self.helper.take();
                        worker.fail("Hagency could not stop in time. Please try again.");
                    }
                    match receiver.try_recv() {
                        Ok(()) => {
                            let aborted = *canceled;
                            self.preparation = None;
                            if aborted {
                                self.helper.take();
                                self.install_guard.take();
                                worker.fail("Hagency could not stop in time. Please try again.");
                            } else if self.helper.as_mut().is_some_and(windows::Helper::ready) {
                                // The helper has a handle to this exact process and a
                                // verified, write-locked installer. It waits for exit.
                                self.exit_requested = true;
                                worker.publish(cx);
                                return true;
                            } else {
                                self.helper.take();
                                self.install_guard.take();
                                worker.fail(
                                    "Could not start the update installer. Please try again.",
                                );
                            }
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            self.preparation = None;
                            self.helper.take();
                            self.install_guard.take();
                            worker.fail("Hagency could not prepare the update. Please try again.");
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {}
                    }
                }
                worker.publish(cx);
            }
        }
        let _ = (cx, event);
        false
    }

    pub fn sparkle_prototype_active(&self) -> bool {
        #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
        {
            sparkle::prototype()
        }
        #[cfg(not(all(target_os = "macos", feature = "macos-sparkle")))]
        false
    }

    #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
    pub fn sparkle_blocks_quit(&self) -> bool {
        self.sparkle.as_ref().is_some_and(|r| r.blocks_quit())
    }

    #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
    pub fn sparkle_needs_preparation(&self) -> bool {
        self.sparkle.as_ref().is_some_and(|r| r.needs_preparation)
    }

    #[cfg(all(target_os = "macos", feature = "macos-sparkle"))]
    pub fn sparkle_prepared(&mut self, ok: bool) {
        if let Some(r) = &mut self.sparkle {
            r.prepared(ok);
        }
    }

    /// Called after the App has successfully persisted its UI and draft state.
    #[cfg(target_os = "windows")]
    pub fn prepare_if_idle(&mut self) -> bool {
        let Some(worker) = &self.worker else {
            return false;
        };
        if self.helper.is_some() || self.preparation.is_some() || !worker.waiting() {
            return false;
        }
        let Some(guard) = hagency::update_guard::PROCESS.try_install() else {
            return false;
        };
        self.install_guard = Some(guard);
        true
    }

    #[cfg(target_os = "windows")]
    pub fn begin_shutdown(&mut self) {
        let Some(worker) = &self.worker else {
            return;
        };
        match windows::Helper::spawn(worker.root()) {
            Ok(helper) => self.helper = Some(helper),
            Err(_) => {
                self.install_guard.take();
                worker.fail("Could not start the update installer. Please try again.");
                return;
            }
        }
        worker.preparing();
    }

    #[cfg(target_os = "windows")]
    pub fn cancel_preparation(&mut self) {
        self.exit_requested = false;
        self.helper.take();
        self.install_guard.take();
        if let Some(worker) = &self.worker {
            worker.fail("Could not save your work. The update was postponed.");
        }
    }

    #[cfg(target_os = "windows")]
    pub fn confirm_exit(&mut self) {
        if let Some(helper) = self.helper.as_mut() {
            helper.record_commit();
            // App is destroyed before main returns. Keep admission closed even
            // during that final window while the process runtime is still live.
            *EXIT_FENCE.lock().unwrap() = self.install_guard.take();
        }
    }
}

/// The copied executable handles installation before claiming the GUI mutex.
#[cfg(target_os = "windows")]
pub fn run_helper_if_requested() -> Option<anyhow::Result<()>> {
    windows::run_if_requested()
}

#[cfg(target_os = "windows")]
pub fn commit_after_shutdown() {
    windows::commit_after_shutdown();
}

fn download_label() -> &'static str {
    if sparkle_active() {
        "Restart and Install"
    } else if cfg!(target_os = "linux") {
        "Open Download Page"
    } else {
        "Download Update"
    }
}
