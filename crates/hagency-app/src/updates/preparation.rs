//! A UI deadline observes cleanup; it never cancels or replaces the operation.
use hagency::shutdown::{Result, Task};
use std::time::{Duration, Instant};
use std::cell::Cell;

pub(super) struct Preparation {
    task: Task,
    started: Instant,
    expired: Cell<bool>,
}
impl Preparation {
    pub(super) fn start() -> Self {
        let runtime = crate::sliding_sync::backend_runtime();
        let _entered = runtime.enter();
        Self::new(Task::start(async {
            crate::hagency::backend::Service::stop_current_checked().await?;
            crate::sliding_sync::stop_sync_service_for_update().await?;
            crate::account_session::close_for_update().await;
            Ok(())
        }))
    }
    fn new(task: Task) -> Self {
        Self {
            task,
            started: Instant::now(),
            expired: Cell::new(false),
        }
    }
    pub(super) fn result(&self) -> Option<Result> {
        self.task.result()
    }
    pub(super) fn overdue(&self) -> bool {
        if self.result().is_none() && self.started.elapsed() >= Duration::from_secs(15) {
            self.expired.set(true);
        }
        self.expired.get()
    }
    pub(super) fn can_install(&self, same_account: bool, saved: bool) -> bool {
        !self.expired.get() && same_account && saved && self.result() == Some(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn deadline_keeps_original_cleanup_and_does_not_make_a_receipt() {
        let (release, held) = tokio::sync::oneshot::channel();
        let task = Task::start(async move {
            held.await.unwrap();
            Ok(())
        });
        let mut preparation = Preparation::new(task.clone());
        preparation.started -= Duration::from_secs(16);
        assert!(preparation.overdue());
        assert_eq!(preparation.result(), None);
        release.send(()).unwrap();
        task.wait().await.unwrap();
        assert_eq!(preparation.result(), Some(Ok(())));
        assert!(
            !preparation.can_install(true, true),
            "late completion must never auto-install"
        );
    }
    #[tokio::test]
    async fn worker_panic_cannot_authorize_installation() {
        let task = Task::start(async { panic!("cleanup failed") });
        let preparation = Preparation::new(task.clone());
        assert!(task.wait().await.is_err());
        assert_eq!(
            preparation.result(),
            Some(Err(hagency::shutdown::Error::Worker))
        );
        assert!(!preparation.can_install(true, true));
    }
    #[tokio::test]
    async fn saved_state_cannot_replace_a_shutdown_receipt_or_current_account() {
        let (release, held) = tokio::sync::oneshot::channel();
        let task = Task::start(async move {
            held.await.unwrap();
            Ok(())
        });
        let preparation = Preparation::new(task.clone());
        assert!(!preparation.can_install(true, true));
        release.send(()).unwrap();
        task.wait().await.unwrap();
        assert!(!preparation.can_install(false, true));
        assert!(!preparation.can_install(true, false));
        assert!(preparation.can_install(true, true));
    }
}
