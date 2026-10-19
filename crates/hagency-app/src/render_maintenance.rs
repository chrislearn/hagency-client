//! Keep the pinned Windows renderer's GPU completion receipts moving.
//!
//! D3D11 charges replacement instance buffers until their submission completes.
//! Its render path submits frames but does not poll the completion query. Without
//! an explicit poll, ordinary redraws eventually exhaust the allocation budget
//! and subsequent draw calls are silently skipped, leaving partial/blank windows.
use makepad_widgets::{Cx, Event};
#[cfg(target_os = "windows")]
use makepad_widgets::Timer;

/// The backend's once-per-frame collector inspects only 64 allocation records.
/// A full chat can replace more than that in one redraw. Visit its inventory
/// with a bounded amount of extra work instead of accumulating old charges.
#[cfg(any(target_os = "windows", test))]
fn collect_completed(
    budget: &mut makepad_widgets::makepad_platform::retained_instances::RetainedAllocationBudget,
    completed: u64,
) {
    for _ in 0..budget.record_count().div_ceil(64).min(16) {
        budget.collect(completed);
    }
}

#[derive(Default)]
pub struct RenderMaintenance {
    #[cfg(target_os = "windows")]
    timer: Timer,
    #[cfg(target_os = "windows")]
    timer_ticks: u64,
}

impl RenderMaintenance {
    pub fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        #[cfg(target_os = "windows")]
        {
            if matches!(event, Event::Shutdown) {
                if !self.timer.is_empty() {
                    cx.stop_timer(self.timer);
                    self.timer = Timer::empty();
                }
                return;
            }
            if self.timer.is_empty() {
                // Complete the last submitted frame even when the UI is idle.
                self.timer = cx.start_interval(0.25);
            }
            if matches!(event, Event::Draw(_)) || self.timer.is_event(event).is_some() {
                // Nonblocking; this observes an older query and queues the next
                // one. It neither redraws the UI nor waits for the GPU.
                let completed = cx.frame_completion_serial();
                collect_completed(&mut cx.draw_lists.1.allocations, completed);
                if self.timer.is_event(event).is_some() {
                    self.timer_ticks += 1;
                    if (self.timer_ticks == 4 || self.timer_ticks % 40 == 0)
                        && std::env::var_os("HAGENCY_RENDER_TRACE").is_some()
                    {
                        let budget = &cx.draw_lists.1.allocations;
                        makepad_widgets::log!(
                            "GPU allocation maintenance: submitted={} completed={} bytes={} records={} refusals={}",
                            cx.frame_submission_serial(), completed,
                            budget.bytes(), budget.record_count(), budget.refusals());
                    }
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (cx, event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use makepad_widgets::makepad_platform::retained_instances::RetainedAllocationBudget;

    #[test]
    fn completed_allocations_keep_up_with_complex_redraws() {
        let mut budget = RetainedAllocationBudget::new(4 * 1024 * 1024);
        let mut live = Vec::new();
        // No backend disposal worker in this unit fixture; keep its metadata
        // queue below 4096. The native example also exercises worker draining.
        for frame in 1..=12 {
            // 256 independent draw calls, beyond the upstream 64-record pass.
            collect_completed(&mut budget, frame - 1);
            live.clear();
            for _ in 0..256 {
                let allocation = budget.reserve(1024).expect("no missing draw calls");
                allocation.submitted(frame);
                live.push(allocation);
            }
            assert!(
                budget.bytes() <= 768 * 1024,
                "retired records must not accumulate"
            );
        }
        assert_eq!(budget.refusals(), 0);
        live.clear();
        collect_completed(&mut budget, 12);
        assert_eq!(budget.bytes(), 0);
    }

    #[test]
    fn pending_gpu_work_and_live_owners_are_not_collected() {
        let mut budget = RetainedAllocationBudget::new(1024);
        let allocation = budget.reserve(512).unwrap();
        allocation.submitted(10);
        collect_completed(&mut budget, 10);
        assert_eq!(budget.bytes(), 512, "live owner retains its allocation");
        drop(allocation);
        collect_completed(&mut budget, 9);
        assert_eq!(budget.bytes(), 512, "GPU must finish before collection");
        collect_completed(&mut budget, 10);
        assert_eq!(budget.bytes(), 0);
    }
}
