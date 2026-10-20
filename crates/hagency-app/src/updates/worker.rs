use super::{Phase, Snapshot, UpdateAction, UpdateState};
use hagency_updater::{Client, Manifest, Preferences, Staged, stable_version};
use makepad_widgets::{Cx, SignalToUI};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(super) const PUBLIC_KEY: &str = match option_env!("HAGENCY_UPDATER_PUBLIC_KEY") {
    Some(value) => value,
    None => "",
};
enum Command {
    Check,
    Download,
    Verify,
}
pub(super) struct Worker {
    root: PathBuf,
    snapshot: Arc<Mutex<Snapshot>>,
    preferences: Arc<Mutex<Preferences>>,
    commands: mpsc::Sender<Command>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn update(snapshot: &Mutex<Snapshot>, change: impl FnOnce(&mut Snapshot)) {
    change(&mut snapshot.lock().unwrap());
    SignalToUI::set_ui_signal();
}
impl Worker {
    pub(super) fn start() -> anyhow::Result<Self> {
        let client = Client::new(PUBLIC_KEY)?;
        let current = stable_version(env!("CARGO_PKG_VERSION"))?;
        let root = crate::app_data_dir().join("updates");
        let loaded = Preferences::load(&root);
        let preferences_valid = loaded.is_ok();
        let preferences = Arc::new(Mutex::new(loaded.unwrap_or(Preferences {
            auto_check: false,
            auto_download: false,
            last_check: 0,
        })));
        let prefs = *preferences.lock().unwrap();
        let snapshot = Arc::new(Mutex::new(Snapshot {
            enabled: true,
            phase: if preferences_valid {
                Phase::Idle
            } else {
                Phase::Failed
            },
            error: "Update preferences could not be read. Check for updates or save your preferences again.",
            auto_check: prefs.auto_check,
            auto_download: prefs.auto_download,
            ..Default::default()
        }));
        let (commands, receiver) = mpsc::channel();
        let state = snapshot.clone();
        let settings = preferences.clone();
        let directory = root.clone();
        std::thread::spawn(move || {
            // Restored packages are verified again on Install and in the helper.
            let mut candidate: Option<(Manifest, Vec<u8>, String)> = None;
            match Staged::load(&directory, &client.key, &current) {
                Ok(Some(staged)) => update(&state, |s| {
                    s.phase = Phase::Ready;
                    s.version = staged.manifest.version.to_string();
                    s.notes = staged.manifest.notes;
                }),
                Ok(None) => {}
                Err(_) => update(&state, |s| {
                    s.phase = Phase::Failed;
                    s.error =
                        "The saved update is invalid. Check for updates to download it again.";
                }),
            }
            if std::fs::read(directory.join("install-result"))
                .ok()
                .as_deref()
                == Some(b"failed")
            {
                update(&state, |s| {
                    s.phase = Phase::Failed;
                    s.error = "The update was canceled or could not be installed. Check for updates to try again.";
                });
            }
            let _ = std::fs::remove_file(directory.join("install-result"));
            let started = Instant::now();
            loop {
                let command = match receiver.recv_timeout(Duration::from_secs(30)) {
                    Ok(command) => command,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let prefs = *settings.lock().unwrap();
                        if started.elapsed() < Duration::from_secs(30)
                            || !prefs.due(now())
                            || matches!(
                                state.lock().unwrap().phase,
                                Phase::Waiting | Phase::Preparing | Phase::Ready
                            )
                        {
                            continue;
                        }
                        Command::Check
                    }
                };
                match command {
                    Command::Check => {
                        update(&state, |s| {
                            s.phase = Phase::Checking;
                        });
                        let mut prefs = settings.lock().unwrap();
                        prefs.last_check = now();
                        let _ = prefs.save(&directory);
                        drop(prefs);
                        match client.check(&current) {
                            Ok(Some(found)) => {
                                update(&state, |s| {
                                    s.phase = Phase::Available;
                                    s.version = found.0.version.to_string();
                                    s.notes = found.0.notes.clone();
                                });
                                candidate = Some(found);
                                if settings.lock().unwrap().auto_download {
                                    download(
                                        &client,
                                        &directory,
                                        candidate.as_ref().unwrap(),
                                        &state,
                                    );
                                }
                            }
                            Ok(None) => update(&state, |s| {
                                s.phase = Phase::Current;
                                s.version.clear();
                                s.notes.clear();
                            }),
                            Err(_) => update(&state, |s| {
                                s.phase = Phase::Failed;
                                s.error = "Could not check for updates. Check your connection and try again.";
                            }),
                        }
                    }
                    Command::Download => {
                        if let Some(candidate) = &candidate {
                            download(&client, &directory, candidate, &state);
                        }
                    }
                    Command::Verify => {
                        let result =
                            Staged::load(&directory, &client.key, &current).and_then(|value| {
                                let staged =
                                    value.ok_or_else(|| anyhow::anyhow!("No staged update"))?;
                                staged.verified_file(&client.key).map(|_| ())
                            });
                        update(&state, |s| {
                            if result.is_ok() {
                                s.phase = Phase::Waiting;
                            } else {
                                s.phase = Phase::Failed;
                                s.error = "The saved update is invalid. Check for updates to download it again.";
                            }
                        });
                    }
                }
            }
        });
        Ok(Self {
            root,
            snapshot,
            preferences,
            commands,
        })
    }
    pub(super) fn root(&self) -> &Path {
        &self.root
    }
    pub(super) fn waiting(&self) -> bool {
        self.snapshot.lock().unwrap().phase == Phase::Waiting
    }
    pub(super) fn preparing(&self) {
        update(&self.snapshot, |s| s.phase = Phase::Preparing);
    }
    pub(super) fn fail(&self, error: &'static str) {
        update(&self.snapshot, |s| {
            s.phase = Phase::Failed;
            s.error = error;
        });
    }
    pub(super) fn action(&self, action: UpdateAction) {
        let phase = self.snapshot.lock().unwrap().phase;
        match action {
            UpdateAction::Check
                if matches!(
                    phase,
                    Phase::Idle | Phase::Current | Phase::Failed | Phase::Available | Phase::Ready
                ) =>
            {
                update(&self.snapshot, |s| s.phase = Phase::Checking);
                let _ = self.commands.send(Command::Check);
            }
            UpdateAction::Download if matches!(phase, Phase::Available) => {
                let _ = self.commands.send(Command::Download);
            }
            UpdateAction::Install if phase == Phase::Ready => {
                update(&self.snapshot, |s| s.phase = Phase::Verifying);
                let _ = self.commands.send(Command::Verify);
            }
            UpdateAction::Later if phase == Phase::Waiting => {
                update(&self.snapshot, |s| s.phase = Phase::Ready)
            }
            UpdateAction::AutoCheck(value) | UpdateAction::AutoDownload(value) => {
                let mut prefs = self.preferences.lock().unwrap();
                let mut next = *prefs;
                if matches!(action, UpdateAction::AutoCheck(_)) {
                    next.auto_check = value;
                } else {
                    next.auto_download = value;
                }
                if next.save(&self.root).is_ok() {
                    *prefs = next;
                    update(&self.snapshot, |s| {
                        s.auto_check = next.auto_check;
                        s.auto_download = next.auto_download;
                    });
                } else {
                    self.fail("Could not save update preferences. Please try again.");
                }
            }
            _ => {}
        }
    }
    pub(super) fn publish(&self, cx: &mut Cx) {
        let snapshot = self.snapshot.lock().unwrap().clone();
        cx.global::<UpdateState>().0 = snapshot;
    }
}
fn download(
    client: &Client,
    root: &Path,
    candidate: &(Manifest, Vec<u8>, String),
    snapshot: &Mutex<Snapshot>,
) {
    update(snapshot, |s| {
        s.phase = Phase::Downloading;
        s.progress = 0;
    });
    let result = client.download(
        root,
        &candidate.0,
        &candidate.1,
        &candidate.2,
        |received, total| {
            let percent = received.saturating_mul(100) / total;
            let mut state = snapshot.lock().unwrap();
            if state.progress != percent {
                state.progress = percent;
                drop(state);
                SignalToUI::set_ui_signal();
            }
        },
    );
    update(snapshot, |s| {
        if result.is_ok() {
            s.phase = Phase::Ready;
        } else {
            s.phase = Phase::Failed;
            s.error = "Could not download or verify the update. Check for updates to retry.";
        }
    });
}
