//! Retain the original cleanup across UI deadlines, canceled waiters and retries.
use std::{future::Future, sync::Arc};
use tokio::{sync::watch, task::JoinHandle};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("shutdown_worker_failed")]
    Worker,
    #[error("agent_shutdown_unconfirmed")]
    Agent,
    #[error("provider_shutdown_unconfirmed")]
    Provider,
    #[error("owner_revocation_not_saved")]
    Revocation,
    #[error("assistant_shutdown_unconfirmed")]
    Assistant,
    #[error("matrix_shutdown_unconfirmed")]
    Matrix,
}
pub type Result = std::result::Result<(), Error>;

#[derive(Clone)]
pub struct Task(Arc<watch::Receiver<Option<Result>>>);
impl Task {
    pub fn start(work: impl Future<Output = Result> + Send + 'static) -> Self {
        let (sender, receiver) = watch::channel(None);
        // The monitor retains and joins the original operation; dropping a waiter
        // cannot abort it or turn a missing result into success.
        let operation = tokio::spawn(work);
        tokio::spawn(async move {
            let result = operation.await.unwrap_or(Err(Error::Worker));
            sender.send_replace(Some(result));
        });
        Self(Arc::new(receiver))
    }
    pub fn result(&self) -> Option<Result> {
        *self.0.borrow()
    }
    pub async fn wait(&self) -> Result {
        let mut receiver = (*self.0).clone();
        loop {
            if let Some(result) = *receiver.borrow_and_update() {
                return result;
            }
            if receiver.changed().await.is_err() {
                return Err(Error::Worker);
            }
        }
    }
}

/// Retain cleanup after its original owner leaves the current-account slot.
/// Only a positive receipt can be pruned; an error is process-lifetime evidence.
#[derive(Default)]
pub struct Registry(Vec<Task>);
impl Registry {
    pub fn track(&mut self, task: Task) {
        self.0.push(task);
    }
    pub fn outstanding(&mut self) -> Vec<Task> {
        self.0.retain(|task| task.result() != Some(Ok(())));
        self.0.clone()
    }
}

/// Store this behind a manager mutex before removing any registry entries.
/// Awaiting a handle by reference is cancellation safe; completed failures stick.
#[derive(Default)]
pub(crate) struct Joins {
    pending: Vec<JoinHandle<()>>,
    failed: bool,
}
impl Joins {
    pub fn push(&mut self, task: JoinHandle<()>) {
        self.pending.push(task);
    }
    pub async fn finish(&mut self) -> Result {
        while let Some(task) = self.pending.last_mut() {
            let result = task.await;
            self.pending.pop();
            self.failed |= result.is_err();
        }
        if self.failed {
            Err(Error::Agent)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn removing_current_owner_cannot_lose_pending_or_failed_retirement() {
        let (release, held) = tokio::sync::oneshot::channel();
        let task = Task::start(async move {
            held.await.unwrap();
            Err(Error::Agent)
        });
        let mut retired = Registry::default();
        retired.track(task.clone());
        drop(task);
        let original = retired.outstanding().pop().unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), original.wait())
                .await
                .is_err()
        );
        assert_eq!(retired.outstanding().len(), 1);
        release.send(()).unwrap();
        assert_eq!(original.wait().await, Err(Error::Agent));
        assert_eq!(
            retired.outstanding().pop().unwrap().wait().await,
            Err(Error::Agent)
        );
    }
    #[tokio::test]
    async fn timeout_and_canceled_waiter_rejoin_the_original_cleanup() {
        let (release, held) = tokio::sync::oneshot::channel();
        let task = Task::start(async move {
            held.await.map_err(|_| Error::Worker)?;
            Ok(())
        });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), task.wait())
                .await
                .is_err()
        );
        assert_eq!(task.result(), None);
        release.send(()).unwrap();
        assert_eq!(task.wait().await, Ok(()));
        assert_eq!(task.wait().await, Ok(()));
    }
    #[tokio::test]
    async fn panic_is_a_retained_failure_not_an_empty_success() {
        let task = Task::start(async { panic!("cleanup panic") });
        assert_eq!(task.wait().await, Err(Error::Worker));
        assert_eq!(task.wait().await, Err(Error::Worker));
    }
    #[tokio::test]
    async fn canceled_drain_keeps_the_original_join_handle() {
        let (release, held) = tokio::sync::oneshot::channel();
        let mut joins = Joins::default();
        joins.push(tokio::spawn(async {
            let _ = held.await;
        }));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), joins.finish())
                .await
                .is_err()
        );
        assert_eq!(joins.pending.len(), 1);
        release.send(()).unwrap();
        assert_eq!(joins.finish().await, Ok(()));
        joins.push(tokio::spawn(async { panic!("worker panic") }));
        assert_eq!(joins.finish().await, Err(Error::Agent));
        assert_eq!(joins.finish().await, Err(Error::Agent));
    }
}
