//! Native Windows update UI and lifecycle. Hosted modules never self-update.
use makepad_widgets::*;

#[cfg(target_os = "windows")]
static EXIT_FENCE: std::sync::Mutex<Option<hagency::update_guard::InstallGuard>> =
    std::sync::Mutex::new(None);

mod ui;
pub use ui::script_mod;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod worker;

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
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub enabled: bool,
    pub auto_check: bool,
    pub auto_download: bool,
    pub phase: Phase,
    pub version: String,
    pub notes: String,
    pub progress: u64,
    pub error: &'static str,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_check: true,
            auto_download: true,
            phase: Phase::Disabled,
            version: String::new(),
            notes: String::new(),
            progress: 0,
            error: "Updates are available in official Windows installers.",
        }
    }
}
impl Snapshot {
    pub fn status(&self) -> &'static str {
        match self.phase {
            Phase::Disabled | Phase::Failed => self.error,
            Phase::Idle => "Check for a new version of Hagency.",
            Phase::Current => "Hagency is up to date.",
            Phase::Checking => "Checking for updates…",
            Phase::Available => "A new version is available.",
            Phase::Downloading => "Downloading update…",
            Phase::Ready => "Update downloaded. Restart when you are ready.",
            Phase::Verifying => "Verifying the downloaded update…",
            Phase::Waiting => {
                "Waiting for Agent tasks to finish. Installation will start when Hagency is idle."
            }
            Phase::Preparing => "Saving your work and preparing to restart…",
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
    #[cfg(target_os = "windows")]
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
