//! Desktop release notifications. Downloads open only after an explicit user action.
use super::{Phase, Snapshot, UpdateAction, UpdateState};
use hagency_updater::{
    Preferences,
    releases::{DesktopPlatform, Release, ReleaseClient},
    stable_version,
};
use makepad_widgets::{Cx, SignalToUI};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

enum Command {
    Check,
    Download,
    AutoCheck(bool),
}

pub(super) struct Worker {
    snapshot: Arc<Mutex<Snapshot>>,
    commands: mpsc::Sender<Command>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn update(state: &Mutex<Snapshot>, change: impl FnOnce(&mut Snapshot)) {
    change(&mut state.lock().unwrap());
    SignalToUI::set_ui_signal();
}

impl Worker {
    pub(super) fn start() -> anyhow::Result<Self> {
        let current = stable_version(env!("CARGO_PKG_VERSION"))?;
        let architecture = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => DesktopPlatform::MacArm64,
            ("macos", "x86_64") => DesktopPlatform::MacAmd64,
            ("linux", "aarch64") => DesktopPlatform::LinuxArm64,
            ("linux", "x86_64") => DesktopPlatform::LinuxAmd64,
            _ => anyhow::bail!("Unsupported desktop platform"),
        };
        let client = ReleaseClient::new()?;
        Ok(Self::start_with(
            crate::app_data_dir()
                .join("updates")
                .join(std::env::consts::OS),
            move || client.check(&current, architecture),
            |url| {
                robius_open::Uri::new(url)
                    .open()
                    .map_err(|_| anyhow::anyhow!("Could not open download"))
            },
        ))
    }

    fn start_with(
        root: PathBuf,
        mut check: impl FnMut() -> anyhow::Result<Option<Release>> + Send + 'static,
        open: impl Fn(&str) -> anyhow::Result<()> + Send + 'static,
    ) -> Self {
        let loaded = Preferences::load(&root);
        let valid = loaded.is_ok();
        let mut preferences = loaded.unwrap_or(Preferences {
            auto_check: false,
            auto_download: false,
            last_check: 0,
        });
        preferences.auto_download = false;
        let snapshot = Arc::new(Mutex::new(Snapshot {
            enabled: true,
            auto_check: preferences.auto_check,
            auto_download: false,
            phase: if valid { Phase::Idle } else { Phase::Failed },
            error: "Update preferences could not be read. Check for updates or save your preferences again.",
            ..Default::default()
        }));
        let state = snapshot.clone();
        let (commands, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut candidate: Option<Release> = None;
            // A fresh check restores the reminder on each launch; later checks are daily.
            let mut initial_check = true;
            loop {
                let command = match receiver.recv_timeout(Duration::from_secs(30)) {
                    Ok(command) => command,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if !preferences.auto_check || (!initial_check && !preferences.due(now())) {
                            continue;
                        }
                        Command::Check
                    }
                };
                match command {
                    Command::Check => {
                        initial_check = false;
                        update(&state, |s| s.phase = Phase::Checking);
                        preferences.last_check = now();
                        let saved = preferences.save(&root).is_ok();
                        match check() {
                            Ok(release) => {
                                candidate = release;
                                update(&state, |s| {
                                    s.phase = if candidate.is_some() {
                                        Phase::Available
                                    } else {
                                        Phase::Current
                                    };
                                    s.version = candidate
                                        .as_ref()
                                        .map(|r| r.version.to_string())
                                        .unwrap_or_default();
                                    s.notes = candidate
                                        .as_ref()
                                        .map(|r| r.notes.clone())
                                        .unwrap_or_default();
                                });
                            }
                            Err(_) => update(&state, |s| {
                                s.phase = Phase::Failed;
                                s.error = "Could not check for updates. Check your connection and try again.";
                            }),
                        }
                        if !saved {
                            update(&state, |s| {
                                s.phase = Phase::Failed;
                                s.error = "Could not save update preferences. Please try again.";
                            });
                        }
                    }
                    Command::Download => {
                        let available = state.lock().unwrap().phase == Phase::Available;
                        if available
                            && let Some(release) = &candidate
                            && open(&release.download_url).is_err()
                        {
                            update(&state, |s| {
                                s.phase = Phase::Failed;
                                s.error = "Could not open the update download. Please try again.";
                            });
                        }
                    }
                    Command::AutoCheck(value) => {
                        let mut next = preferences;
                        next.auto_check = value;
                        if next.save(&root).is_ok() {
                            preferences = next;
                            update(&state, |s| s.auto_check = value);
                        } else {
                            update(&state, |s| {
                                s.phase = Phase::Failed;
                                s.error = "Could not save update preferences. Please try again.";
                            });
                        }
                    }
                }
            }
        });
        Self { snapshot, commands }
    }

    pub(super) fn action(&self, action: UpdateAction) {
        let command = match action {
            UpdateAction::Check => Command::Check,
            UpdateAction::Download => Command::Download,
            UpdateAction::AutoCheck(value) => Command::AutoCheck(value),
            _ => return,
        };
        let _ = self.commands.send(command);
    }

    pub(super) fn publish(&self, cx: &mut Cx) {
        cx.global::<UpdateState>().0 = self.snapshot.lock().unwrap().clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for(worker: &Worker, condition: impl Fn(&Snapshot) -> bool) {
        let until = std::time::Instant::now() + Duration::from_secs(3);
        while !condition(&worker.snapshot.lock().unwrap()) {
            assert!(
                std::time::Instant::now() < until,
                "Update worker did not reach expected state"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn desktop_updates_only_open_download_on_request_and_persist_check_preferences() {
        let root = tempfile::tempdir().unwrap();
        let (opened, requests) = mpsc::channel();
        let worker = Worker::start_with(
            root.path().into(),
            || {
                Ok(Some(Release {
            version: "0.7.0".parse().unwrap(), notes: "Notes".into(),
            download_url: "https://github.com/hagency-org/hagency-client/releases/download/v0.7.0/hagency-macos-arm64.dmg".into(),
        }))
            },
            move |url| {
                opened.send(url.to_owned())?;
                Ok(())
            },
        );
        worker.action(UpdateAction::Check);
        wait_for(&worker, |s| s.phase == Phase::Available);
        assert!(requests.try_recv().is_err());
        assert!(!worker.snapshot.lock().unwrap().auto_download);
        worker.action(UpdateAction::Install);
        worker.action(UpdateAction::AutoCheck(false));
        wait_for(&worker, |s| !s.auto_check);
        assert!(requests.try_recv().is_err());
        assert!(!Preferences::load(root.path()).unwrap().auto_check);
        worker.action(UpdateAction::Download);
        assert!(
            requests
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                .ends_with("arm64.dmg")
        );
        assert_eq!(worker.snapshot.lock().unwrap().phase, Phase::Available);
    }

    #[test]
    fn desktop_update_failures_are_visible_and_checks_recover() {
        let root = tempfile::tempdir().unwrap();
        let mut checks = 0;
        let worker = Worker::start_with(
            root.path().into(),
            move || {
                checks += 1;
                if checks == 1 {
                    anyhow::bail!("fixture transport failure");
                }
                Ok(None)
            },
            |_| panic!("A check must never open a download"),
        );
        worker.action(UpdateAction::Check);
        wait_for(&worker, |s| s.phase == Phase::Failed);
        worker.action(UpdateAction::Check);
        wait_for(&worker, |s| s.phase == Phase::Current);
        assert!(worker.snapshot.lock().unwrap().version.is_empty());
    }

    #[test]
    fn desktop_download_open_failure_does_not_claim_installation_or_download_success() {
        let root = tempfile::tempdir().unwrap();
        let worker = Worker::start_with(
            root.path().into(),
            || {
                Ok(Some(Release {
            version: "0.7.0".parse().unwrap(), notes: String::new(),
            download_url: "https://github.com/hagency-org/hagency-client/releases/download/v0.7.0/hagency-macos-arm64.dmg".into(),
        }))
            },
            |_| anyhow::bail!("fixture browser refusal"),
        );
        worker.action(UpdateAction::Check);
        wait_for(&worker, |s| s.phase == Phase::Available);
        worker.action(UpdateAction::Download);
        wait_for(&worker, |s| s.phase == Phase::Failed);
        assert_eq!(
            worker.snapshot.lock().unwrap().error,
            "Could not open the update download. Please try again."
        );
    }
}
