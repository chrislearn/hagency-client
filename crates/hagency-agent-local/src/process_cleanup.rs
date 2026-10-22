//! Retain a provider child even when its spawning/request future is canceled.
//! `kill_on_drop` alone does not acknowledge that the original child exited.
use std::{
    ops::{Deref, DerefMut},
    sync::{LazyLock, Mutex},
};
use tokio::{process::Child, sync::watch};

type Receipt = watch::Receiver<Option<bool>>;
static RETIRED: LazyLock<Mutex<Vec<Receipt>>> = LazyLock::new(|| Mutex::new(Vec::new()));

pub struct RetainedChild(Option<Child>);
impl From<Child> for RetainedChild {
    fn from(child: Child) -> Self {
        Self(Some(child))
    }
}
impl Deref for RetainedChild {
    type Target = Child;
    fn deref(&self) -> &Child {
        self.0.as_ref().unwrap()
    }
}
impl DerefMut for RetainedChild {
    fn deref_mut(&mut self) -> &mut Child {
        self.0.as_mut().unwrap()
    }
}
impl Drop for RetainedChild {
    fn drop(&mut self) {
        let Some(mut child) = self.0.take() else {
            return;
        };
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        let (sender, receiver) = watch::channel(None);
        {
            let mut retained = RETIRED.lock().unwrap_or_else(|error| error.into_inner());
            retained.retain(|receipt| *receipt.borrow() != Some(true));
            retained.push(receiver);
        }
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let operation = runtime.spawn(async move {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    return true;
                }
                child.kill().await.is_ok()
            });
            runtime.spawn(async move {
                sender.send_replace(Some(operation.await.unwrap_or(false)));
            });
        } else {
            sender.send_replace(Some(false));
        }
    }
}

/// Call after admission is fenced and every original worker/owner is joined.
/// Failures remain retained, including loss of the original monitor's receipt.
pub async fn finish_all() -> bool {
    let receipts = RETIRED.lock().map(|receipts| receipts.clone()).map_err(|_| ());
    let Ok(receipts) = receipts else {
        return false;
    };
    let mut stopped = true;
    for mut receipt in receipts {
        loop {
            if let Some(result) = *receipt.borrow_and_update() {
                stopped &= result;
                break;
            }
            if receipt.changed().await.is_err() {
                stopped = false;
                break;
            }
        }
    }
    stopped
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn canceled_owner_retains_and_reaps_the_original_child() {
        let child = tokio::process::Command::new("/bin/sleep")
            .arg("60")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        let child = RetainedChild::from(child);
        drop(child); // request/initialization unwound before its explicit stop
        assert!(finish_all().await);
        let alive = std::process::Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(!alive.success());
        assert!(finish_all().await);
    }
}
