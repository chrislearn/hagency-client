//! Coalesce replaceable content without crossing ordered control events.
use crate::home::room_screen::TimelineUpdate;
use crossbeam_channel::{SendError, TryRecvError};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

struct State {
    queue: VecDeque<TimelineUpdate>,
    senders: usize,
    receiving: bool,
    forward: Option<Arc<Mutex<State>>>,
}

pub struct Sender(Arc<Mutex<State>>);
pub struct Receiver(Arc<Mutex<State>>);

pub fn channel() -> (Sender, Receiver) {
    let state = Arc::new(Mutex::new(State {
        queue: VecDeque::new(),
        senders: 1,
        receiving: true,
        forward: None,
    }));
    (Sender(state.clone()), Receiver(state))
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        self.0.lock().unwrap().senders += 1;
        Self(self.0.clone())
    }
}
impl Drop for Sender {
    fn drop(&mut self) {
        self.0.lock().unwrap().senders -= 1;
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        let mut state = self.0.lock().unwrap();
        state.receiving = false;
        state.queue.clear();
    }
}

fn content(update: &TimelineUpdate) -> bool {
    matches!(
        update,
        TimelineUpdate::NewItems { .. } | TimelineUpdate::LocalEchoProgress { .. }
    )
}

fn operation(update: &TimelineUpdate) -> bool {
    matches!(
        update,
        TimelineUpdate::MessageEdited { .. }
            | TimelineUpdate::PinResult { .. }
            | TimelineUpdate::SendFailedBeforeBeingQueued { .. }
            | TimelineUpdate::FileUploadStarted { .. }
            | TimelineUpdate::FileUploadQueuing { .. }
            | TimelineUpdate::FileUploadProgress { .. }
            | TimelineUpdate::FileUploadError { .. }
            | TimelineUpdate::FileUploadComplete { .. }
            | TimelineUpdate::AttachmentDownloadFinished(..)
            | TimelineUpdate::AttachmentDownloadReset(..)
    )
}

impl Sender {
    pub(crate) fn identity(&self) -> usize { Arc::as_ptr(&self.0) as usize }
    /// Retain operation outcomes while dropping a hidden window's snapshots.
    pub(crate) fn discard_snapshots(&self) {
        self.0.lock().unwrap().queue.retain(operation);
    }
    /// Only mutation/transfer outcomes follow a same-account context switch.
    /// Positional snapshots, pagination results and search targets must stay fenced.
    pub(crate) fn forward_operations_to(&self, next: &Self) {
        let pending = {
            let mut state = self.0.lock().unwrap();
            state.forward = Some(next.0.clone());
            state.queue.drain(..).filter(operation).collect::<Vec<_>>()
        };
        for update in pending {
            let _ = next.send(update);
        }
    }
    pub fn send(&self, update: TimelineUpdate) -> Result<(), SendError<()>> {
        Self::enqueue(&self.0, update)
    }
    fn enqueue(
        destination: &Arc<Mutex<State>>,
        update: TimelineUpdate,
    ) -> Result<(), SendError<()>> {
        let mut destination = destination.clone();
        loop {
            let mut state = destination.lock().unwrap();
            if let Some(next) = state.forward.clone() {
                if !operation(&update) {
                    return Err(SendError(()));
                }
                drop(state);
                destination = next;
                continue;
            }
            if !state.receiving {
                return Err(SendError(()));
            }
            if content(&update) && state.queue.back().is_some_and(content) {
                let previous = state.queue.pop_back().unwrap();
                if matches!(previous, TimelineUpdate::LocalEchoProgress { .. })
                    && matches!(update, TimelineUpdate::LocalEchoProgress { .. })
                {
                    state.queue.push_back(update);
                    return Ok(());
                }
                // Once intermediate snapshots disappear, invalidate their combined content.
                // Preserve append notification even when the final update is upload progress.
                let was_append = matches!(
                    previous,
                    TimelineUpdate::NewItems {
                        is_append: true,
                        ..
                    }
                );
                let clear_cache = matches!(
                    previous,
                    TimelineUpdate::NewItems {
                        clear_cache: true,
                        ..
                    }
                ) || matches!(
                    update,
                    TimelineUpdate::NewItems {
                        clear_cache: true,
                        ..
                    }
                );
                let update = match update {
                    TimelineUpdate::NewItems {
                        new_items,
                        is_append,
                        ..
                    } => TimelineUpdate::NewItems {
                        changed_indices: 0..new_items.len(),
                        new_items,
                        clear_cache,
                        is_append: was_append || is_append,
                    },
                    TimelineUpdate::LocalEchoProgress { new_items } => TimelineUpdate::NewItems {
                        changed_indices: 0..new_items.len(),
                        new_items,
                        clear_cache,
                        is_append: was_append,
                    },
                    _ => unreachable!(),
                };
                state.queue.push_back(update);
            } else {
                state.queue.push_back(update);
            }
            return Ok(());
        }
    }
}
impl Receiver {
    pub(crate) fn identity(&self) -> usize { Arc::as_ptr(&self.0) as usize }
    pub fn try_recv(&self) -> Result<TimelineUpdate, TryRecvError> {
        let mut state = self.0.lock().unwrap();
        state.queue.pop_front().ok_or(if state.senders == 0 {
            TryRecvError::Disconnected
        } else {
            TryRecvError::Empty
        })
    }
    pub fn is_empty(&self) -> bool {
        self.0.lock().unwrap().queue.is_empty()
    }
    #[cfg(test)]
    fn len(&self) -> usize {
        self.0.lock().unwrap().queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imbl::Vector;
    fn snapshot(append: bool) -> TimelineUpdate {
        TimelineUpdate::NewItems {
            new_items: Vector::new(),
            changed_indices: 0..0,
            clear_cache: false,
            is_append: append,
        }
    }
    #[test]
    fn content_bursts_keep_one_snapshot_and_append_semantics() {
        let (tx, rx) = channel();
        tx.send(snapshot(true)).unwrap();
        for _ in 0..100_000 {
            tx.send(snapshot(false)).unwrap();
        }
        tx.send(TimelineUpdate::LocalEchoProgress {
            new_items: Vector::new(),
        })
        .unwrap();
        assert_eq!(rx.len(), 1);
        assert!(matches!(
            rx.try_recv(),
            Ok(TimelineUpdate::NewItems {
                clear_cache: false,
                is_append: true,
                ..
            })
        ));
    }
    #[test]
    fn control_events_remain_ordered_between_content() {
        let (tx, rx) = channel();
        tx.send(snapshot(false)).unwrap();
        tx.send(TimelineUpdate::PaginationRunning(
            crate::sliding_sync::PaginationDirection::Backwards,
        ))
        .unwrap();
        tx.send(snapshot(true)).unwrap();
        assert_eq!(rx.len(), 3);
        assert!(matches!(
            rx.try_recv(),
            Ok(TimelineUpdate::NewItems {
                is_append: false,
                ..
            })
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(TimelineUpdate::PaginationRunning(_))
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(TimelineUpdate::NewItems {
                is_append: true,
                ..
            })
        ));
    }
    #[test]
    fn disconnect_drops_retained_content_and_rejects_late_producers() {
        let (tx, rx) = channel();
        let late = tx.clone();
        tx.send(snapshot(false)).unwrap();
        drop(rx);
        assert!(late.send(snapshot(false)).is_err());
        assert!(tx.0.lock().unwrap().queue.is_empty());
    }
    #[test]
    fn context_replacement_forwards_transfer_outcomes_but_rejects_stale_positions() {
        let (old, old_rx) = channel();
        let (next, next_rx) = channel();
        let uri = "mxc://example.org/download".into();
        old.send(TimelineUpdate::AttachmentDownloadFinished(uri, Ok(())))
            .unwrap();
        old.send(snapshot(false)).unwrap();
        old.forward_operations_to(&next);
        drop(old_rx);
        assert!(old.send(snapshot(true)).is_err());
        assert!(matches!(
            next_rx.try_recv(),
            Ok(TimelineUpdate::AttachmentDownloadFinished(_, Ok(())))
        ));
        let (latest, latest_rx) = channel();
        next.forward_operations_to(&latest);
        drop(next_rx);
        old.send(TimelineUpdate::AttachmentDownloadReset(
            "mxc://example.org/download".into(),
        ))
        .unwrap();
        assert!(matches!(
            latest_rx.try_recv(),
            Ok(TimelineUpdate::AttachmentDownloadReset(_))
        ));
    }
    #[test]
    fn progress_coalescing_keeps_the_progress_only_fast_path() {
        let (tx, rx) = channel();
        for _ in 0..1000 {
            tx.send(TimelineUpdate::LocalEchoProgress {
                new_items: Vector::new(),
            })
            .unwrap();
        }
        assert_eq!(rx.len(), 1);
        assert!(matches!(
            rx.try_recv(),
            Ok(TimelineUpdate::LocalEchoProgress { .. })
        ));
    }
}
