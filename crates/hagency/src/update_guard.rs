//! Process-local admission fence: finish existing Agent work before updating.
use std::sync::{Arc, LazyLock, Mutex};

#[derive(Default)]
struct State {
    tasks: usize,
    updating: bool,
}

#[derive(Clone, Default)]
pub struct UpdateGate(Arc<Mutex<State>>);

pub struct TaskGuard(UpdateGate);
pub struct InstallGuard(UpdateGate);

impl UpdateGate {
    pub fn enter_task(&self) -> Option<TaskGuard> {
        let mut state = self.0.lock().ok()?;
        if state.updating {
            return None;
        }
        state.tasks += 1;
        Some(TaskGuard(self.clone()))
    }
    /// A failed attempt leaves running tasks and admission unchanged.
    pub fn try_install(&self) -> Option<InstallGuard> {
        let mut state = self.0.lock().ok()?;
        if state.updating || state.tasks != 0 {
            return None;
        }
        state.updating = true;
        Some(InstallGuard(self.clone()))
    }
}
impl Drop for TaskGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.0.lock() {
            state.tasks -= 1;
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
