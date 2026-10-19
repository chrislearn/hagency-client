//! Compatibility with the pinned Makepad CachedWidget during a DSL rebake.
//!
//! Upstream keeps its old template on Rebake/ScriptReapply, then mistakes the
//! newly visited single child for an additional child. Clear only that template
//! through its public hook; the cached child and its runtime state stay alive.
use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets_internal.*
    mod.widgets.CachedWidget = #(RebakeCachedWidget::register_widget(vm))
}

#[derive(Script, Widget)]
pub struct RebakeCachedWidget {
    #[deref]
    cached: CachedWidget,
}

impl ScriptHook for RebakeCachedWidget {
    fn on_before_apply(
        &mut self,
        vm: &mut ScriptVm,
        apply: &Apply,
        scope: &mut Scope,
        value: ScriptValue,
    ) {
        if matches!(apply, Apply::Rebake | Apply::ScriptReapply) {
            self.cached
                .on_before_apply(vm, &Apply::Reload, scope, value);
        }
    }
}

impl Widget for RebakeCachedWidget {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.cached.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.cached.draw_walk(cx, scope, walk)
    }
}
