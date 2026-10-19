//! Management pages share the workspace bounds, leaving navigation available.
//! Confirmation, verification and media dialogs keep their own modal scope.
use makepad_widgets::*;
use makepad_widgets::makepad_platform::event::finger::TouchState;

#[derive(Clone, Copy, Default)]
pub struct WorkspacePageBounds(pub Option<Rect>);

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    mod.widgets.WorkspacePages = #(WorkspacePages::register_widget(vm)) {
        width: Fill height: Fill flow: Overlay
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct WorkspacePages {
    #[deref]
    view: View,
}

impl Widget for WorkspacePages {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if event.requires_visibility() {
            let Some(bounds) = cx.global::<WorkspacePageBounds>().0 else { return };
            // Modal backgrounds must not claim a new press in adjacent menus.
            // Keep move/up events flowing so a drag that leaves the page can finish.
            let outside = match event {
                Event::MouseDown(e) => !bounds.contains(e.abs),
                Event::Scroll(e) => !bounds.contains(e.abs),
                Event::TouchUpdate(e) => e.touches.iter().all(|touch|
                    touch.state == TouchState::Start && !bounds.contains(touch.abs)),
                _ => false,
            };
            if outside { return; }
        }
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let Some(page) = cx.global::<WorkspacePageBounds>().0 else {
            // A lazy/adaptive page may need another frame to finish layout.
            // Never fall back to covering the whole window while it is pending.
            return DrawStep::done();
        };
        let outer = cx.global::<ModalBounds>().0;
        cx.global::<ModalBounds>().0 = Some(match outer {
            Some(host) => {
                let left = page.pos.x.max(host.pos.x);
                let top = page.pos.y.max(host.pos.y);
                Rect {
                    pos: dvec2(left, top),
                    size: dvec2(
                        (page.pos.x + page.size.x)
                            .min(host.pos.x + host.size.x)
                            .max(left)
                            - left,
                        (page.pos.y + page.size.y)
                            .min(host.pos.y + host.size.y)
                            .max(top)
                            - top,
                    ),
                }
            }
            None => page,
        });
        let step = self.view.draw_walk(cx, scope, walk);
        cx.global::<ModalBounds>().0 = outer;
        let mut open = false;
        self.view.children(&mut |_, page| open |= page.as_modal().is_open());
        if open {
            // Management pages must allow the adjacent navigation to scroll.
            // Dialogs drawn after this host can still install their narrower scope.
            cx.block_scrolling_except_within(self.view.area());
        }
        step
    }
}
