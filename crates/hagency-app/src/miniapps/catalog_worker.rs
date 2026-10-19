//! One serial account-owned catalog worker. Never block the native event loop
//! on GitHub or share pending results between Matrix accounts.
use makepad_widgets::SignalToUI;
use rinx_miniapp_catalog::{Client, Consent, Snapshot, VerifiedBundle};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

pub enum Command {
    Refresh,
    Install(Consent),
    Open(Consent),
    Remove(String),
    Record(String),
}
pub enum Outcome {
    Updated,
    Opened(VerifiedBundle),
}
pub struct Reply {
    pub job: u64,
    pub snapshot: Snapshot,
    pub result: Result<Outcome, String>,
}
pub struct Worker {
    send: Sender<(u64, Command)>,
    pub receive: Receiver<Reply>,
}
impl Worker {
    pub fn new(root: PathBuf) -> Self {
        let (send, commands) = mpsc::channel();
        let (replies, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let platform = std::env::consts::OS;
            let mut client = match Client::production(root, platform) {
                Ok(client) => client,
                Err(error) => {
                    let _ = replies.send(Reply {
                        job: 0,
                        snapshot: Snapshot::default(),
                        result: Err(error),
                    });
                    SignalToUI::set_ui_signal();
                    return;
                }
            };
            let _ = replies.send(Reply {
                job: 0,
                snapshot: client.snapshot(),
                result: Ok(Outcome::Updated),
            });
            SignalToUI::set_ui_signal();
            while let Ok((job, command)) = commands.recv() {
                let result = match command {
                    Command::Refresh => {
                        client.refresh();
                        Ok(Outcome::Updated)
                    }
                    Command::Install(consent) => client.install(&consent).map(|_| Outcome::Updated),
                    Command::Open(consent) => client.open(&consent).map(Outcome::Opened),
                    Command::Remove(id) => client.remove(&id).map(|_| Outcome::Updated),
                    Command::Record(id) => client.record_open(&id).map(|_| Outcome::Updated),
                };
                if replies
                    .send(Reply {
                        job,
                        snapshot: client.snapshot(),
                        result,
                    })
                    .is_err()
                {
                    break;
                }
                SignalToUI::set_ui_signal();
            }
        });
        Self { send, receive }
    }
    pub fn request(&self, job: u64, command: Command) -> Result<(), String> {
        self.send
            .send((job, command))
            .map_err(|_| "App Hub worker stopped; reopen Mini apps".into())
    }
}
