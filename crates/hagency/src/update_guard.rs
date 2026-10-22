//! Process-local admission fence: finish existing Agent work before updating.
use std::sync::{Arc, LazyLock, Mutex};

#[derive(Default)]
struct State {
    tasks: usize,
    continuations: usize,
    updating: bool,
    unsafe_cleanup: bool,
}

#[derive(Clone, Default)]
pub struct UpdateGate(Arc<Mutex<State>>);

pub struct TaskGuard {
    gate: UpdateGate,
    continuation: bool,
}
pub struct InstallGuard(UpdateGate);
pub struct PendingInstall {
    gate: UpdateGate,
    owned: bool,
}

impl UpdateGate {
    pub fn enter_task(&self) -> Option<TaskGuard> {
        let mut state = self.0.lock().ok()?;
        if state.updating || state.unsafe_cleanup {
            return None;
        }
        state.tasks += 1;
        Some(TaskGuard {
            gate: self.clone(),
            continuation: false,
        })
    }
    /// Only status/approval operations for already admitted work may use this.
    /// Once original work drains, new observers cannot prolong an update forever.
    pub fn continue_task(&self) -> Option<TaskGuard> {
        let mut state = self.0.lock().ok()?;
        if state.unsafe_cleanup || (state.updating && state.tasks == 0) {
            return None;
        }
        state.continuations += 1;
        Some(TaskGuard {
            gate: self.clone(),
            continuation: true,
        })
    }
    /// A failed attempt leaves running tasks and admission unchanged.
    pub fn try_install(&self) -> Option<InstallGuard> {
        let mut state = self.0.lock().ok()?;
        if state.updating || state.unsafe_cleanup || state.tasks != 0 || state.continuations != 0 {
            return None;
        }
        state.updating = true;
        Some(InstallGuard(self.clone()))
    }
    /// Fence new claims now, but never cancel a task already admitted.
    pub fn begin_update(&self) -> Option<PendingInstall> {
        let mut state = self.0.lock().ok()?;
        if state.updating || state.unsafe_cleanup {
            return None;
        }
        state.updating = true;
        Some(PendingInstall {
            gate: self.clone(),
            owned: true,
        })
    }
    pub fn block_install(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.unsafe_cleanup = true;
        }
    }
    pub fn cleanup_unconfirmed(&self) -> bool {
        self.0.lock().map_or(true, |state| state.unsafe_cleanup)
    }
}
impl PendingInstall {
    pub fn try_install(&mut self) -> Option<InstallGuard> {
        let state = self.gate.0.lock().ok()?;
        if !self.owned || state.unsafe_cleanup || state.tasks != 0 || state.continuations != 0 {
            return None;
        }
        self.owned = false;
        Some(InstallGuard(self.gate.clone()))
    }
}
impl Drop for PendingInstall {
    fn drop(&mut self) {
        if self.owned
            && let Ok(mut state) = self.gate.0.lock()
        {
            state.updating = false;
        }
    }
}
impl Drop for TaskGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.gate.0.lock() {
            if self.continuation {
                state.continuations -= 1;
            } else {
                state.tasks -= 1;
            }
        }
    }
}
impl Drop for InstallGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.0.lock() {
            state.updating = false;
        }
    }
}

pub static PROCESS: LazyLock<UpdateGate> = LazyLock::new(UpdateGate::default);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approval_can_finish_original_work_but_observers_cannot_extend_it_forever() {
        let gate = UpdateGate::default();
        let task = gate.enter_task().unwrap();
        let mut pending = gate.begin_update().unwrap();
        let approval = gate
            .continue_task()
            .expect("owner can approve the running task");
        assert!(gate.enter_task().is_none());
        assert!(pending.try_install().is_none());
        drop(task);
        assert!(
            gate.continue_task().is_none(),
            "late polls cannot keep update waiting"
        );
        assert!(
            pending.try_install().is_none(),
            "join the already admitted approval"
        );
        drop(approval);
        let installation = pending.try_install().unwrap();
        assert!(gate.continue_task().is_none());
        drop(installation);
    }
    #[test]
    fn pending_update_stops_new_claims_without_canceling_running_tasks() {
        let gate = UpdateGate::default();
        let running = gate.enter_task().unwrap();
        let mut pending = gate.begin_update().unwrap();
        assert!(gate.enter_task().is_none());
        assert!(gate.begin_update().is_none());
        assert!(pending.try_install().is_none());
        drop(running);
        let install = pending.try_install().unwrap();
        drop(pending); // ownership transferred, so this must not reopen admission
        assert!(gate.enter_task().is_none());
        drop(install);
        assert!(gate.enter_task().is_some());
    }
    #[test]
    fn canceling_wait_resumes_admission_and_keeps_the_original_task() {
        let gate = UpdateGate::default();
        let running = gate.enter_task().unwrap();
        let pending = gate.begin_update().unwrap();
        drop(pending);
        assert!(gate.enter_task().is_some());
        assert!(gate.try_install().is_none());
        drop(running);
        assert!(gate.try_install().is_some());
    }
    #[test]
    fn unconfirmed_cleanup_never_becomes_idle_install_permission() {
        let gate = UpdateGate::default();
        let pending = gate.begin_update().unwrap();
        gate.block_install();
        drop(pending);
        assert!(gate.cleanup_unconfirmed());
        assert!(gate.try_install().is_none());
        assert!(gate.begin_update().is_none());
        assert!(gate.enter_task().is_none());
    }
    #[test]
    fn update_waits_for_tasks_and_fences_new_admission() {
        let gate = UpdateGate::default();
        let task = gate.enter_task().unwrap();
        assert!(gate.try_install().is_none());
        let another = gate.enter_task().unwrap();
        drop(task);
        assert!(gate.try_install().is_none());
        drop(another);
        let installation = gate.try_install().unwrap();
        assert!(gate.enter_task().is_none());
        assert!(gate.try_install().is_none());
        drop(installation);
        assert!(gate.enter_task().is_some());
    }

    #[test]
    fn racing_tasks_and_installation_never_overlap() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        let gate = UpdateGate::default();
        let active = Arc::new(AtomicUsize::new(0));
        let installing = Arc::new(AtomicBool::new(false));
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let gate = gate.clone();
                let active = active.clone();
                let installing = installing.clone();
                scope.spawn(move || {
                    for _ in 0..1000 {
                        if let Some(task) = gate.enter_task() {
                            active.fetch_add(1, Ordering::SeqCst);
                            assert!(!installing.load(Ordering::SeqCst));
                            std::thread::yield_now();
                            active.fetch_sub(1, Ordering::SeqCst);
                            drop(task);
                        }
                    }
                });
            }
            for _ in 0..1000 {
                if let Some(guard) = gate.try_install() {
                    installing.store(true, Ordering::SeqCst);
                    assert_eq!(active.load(Ordering::SeqCst), 0);
                    std::thread::yield_now();
                    installing.store(false, Ordering::SeqCst);
                    drop(guard);
                }
            }
        });
    }
}
