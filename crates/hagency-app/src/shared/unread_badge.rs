//! This module defines a badge that shows the count of unread mentions
//! (`@`-prefixed, in red) or unread messages (bare count, in gray).
//!
//! Color alone does not carry the mention/message distinction: the `@` prefix
//! is the redundant channel that keeps the two readable for color-blind users
//! and in isolation, where there is no neighbouring badge to compare against.

use crate::theme::Snapshot as ThemeSnapshot;
use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.UnreadBadge = #(UnreadBadge::register_widget(vm)) {

        width: Fit height: Fit
        align: Align{x: 0.5 y: 0.5}
        pill := RoundedView {
            width: Fit height: Fit padding: Inset{left: 5 right: 5 top: 1 bottom: 1}
            draw_bg +: {color: RINX_FIELD border_radius: RINX_CONTROL_RADIUS}
            label_count := RinxLabel {
                width: Fit height: Fit padding: 0
                flow: Flow.Right{wrap: false}
                draw_text +: {color: RINX_MUTED text_style: RBX_TEXT_BADGE {}}
            }
        }
        dot := RoundedView {
            visible: false width: 10 height: 10
            draw_bg +: {color: RINX_ACCENT border_radius: 5}
        }
    }
}


#[derive(Script, Widget)]
pub struct UnreadBadge {
    #[rust]
    appearance: ThemeSnapshot,
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    #[live]
    is_marked_unread: bool,
    #[live]
    unread_mentions: u64,
    #[live]
    unread_messages: u64,
    /// The above 3 value that were last drawn (to avoid re-applying their styling).
    #[rust] last_drawn: Option<(bool, u64, u64)>,
}

impl ScriptHook for UnreadBadge {
    fn on_after_apply(
        &mut self,
        vm: &mut ScriptVm,
        apply: &Apply,
        _scope: &mut Scope,
        _value: ScriptValue,
    ) {
        self.appearance = crate::theme::snapshot_for_vm(vm);
        if apply.is_script_reapply() {
            self.last_drawn = None;
        }
    }
}

impl Widget for UnreadBadge {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {

        let now = (self.is_marked_unread, self.unread_mentions, self.unread_messages);
        if self.last_drawn != Some(now) {
            self.last_drawn = Some(now);
            let mentions = self.unread_mentions > 0;
            let count = if mentions { self.unread_mentions } else { self.unread_messages };
            let mut pill = self.view(cx, ids!(pill));
            pill.set_visible(cx, count > 0);
            self.view(cx, ids!(dot)).set_visible(cx, self.is_marked_unread && count == 0);
            self.visible = count > 0 || self.is_marked_unread;
            if count > 0 {
                let (fg, bg) = if mentions {
                    (self.appearance.role("color.status.danger.foreground"), self.appearance.role("color.status.danger.background"))
                } else if self.is_marked_unread {
                    (self.appearance.on_accent, self.appearance.accent)
                } else {
                    (self.appearance.ink, self.appearance.field)
                };
                let prefix = if mentions { "@" } else { "" };
                let suffix = if count > 99 { "+" } else { "" };
                let label = self.label(cx, ids!(pill.label_count));
                label.set_text(cx, &format!("{prefix}{}{suffix}", count.min(99)));
                label.set_text_color(cx, fg);
                script_apply_eval!(cx, pill, {draw_bg.color: #(bg)});
            }
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl UnreadBadgeRef {
    /// Sets the unread mentions and messages counts without explicitly redrawing the badge.
    pub fn update_counts(&self, is_marked_unread: bool, num_unread_mentions: u64, num_unread_messages: u64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.is_marked_unread = is_marked_unread;
            inner.unread_mentions = num_unread_mentions;
            inner.unread_messages = num_unread_messages;
            inner.visible = is_marked_unread || num_unread_mentions > 0 || num_unread_messages > 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use makepad_widgets::makepad_draw::cx_draw::CxDraw;

    #[test]
    fn real_badge_distinguishes_messages_manual_unread_mentions_and_clear() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let badge=cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);crate::app::register_widgets(vm);
            let value=script_eval!(vm,{mod.widgets.UnreadBadge {}});
            WidgetRef::script_from_value(vm,value)
        });
        let pass=DrawPass::new(&mut cx);
        pass.set_size(&mut cx,dvec2(60.0,40.0));
        let mut list=DrawList2d::new(&mut cx);
        for (marked,mentions,messages,text,visible) in [
            (false,0,0,"",false),
            (false,0,1,"1",true),
            (false,0,2,"2",true),
            (true,0,2,"2",true),
            (true,0,0,"",true),
            (false,0,0,"",false),
            (false,1,3,"@1",true),
            (false,0,99,"99",true),
            (false,0,100,"99+",true),
            (true,100,101,"@99+",true),
        ] {
            badge.as_unread_badge().update_counts(marked,mentions,messages);
            cx.redraw_all();
            let event=DrawEvent::default();
            let mut draw=CxDraw::new(&mut cx,&event);
            let mut cx2d=Cx2d::new(&mut draw);
            cx2d.begin_pass(&pass,None);list.begin_always(&mut cx2d);
            cx2d.begin_root_turtle(dvec2(60.0,40.0),Layout::flow_overlay());
            badge.draw_all(&mut cx2d, &mut Scope::empty());
            cx2d.end_pass_sized_turtle();list.end(&mut cx2d);cx2d.end_pass(&pass);
            drop(cx2d);drop(draw);
            assert_eq!(badge.visible(),visible);
            if visible {
                assert_eq!(badge.view(&cx, ids!(dot)).visible(), text.is_empty());
            }
            if !text.is_empty() {
                let label=badge.label(&cx,ids!(pill.label_count));
                assert_eq!(label.text().trim(),text,"marked={marked}, mentions={mentions}, messages={messages}");
                let rect=label.area().rect(&cx);let outer=badge.area().rect(&cx);
                assert!(rect.pos.x>=outer.pos.x && rect.pos.x+rect.size.x<=outer.pos.x+outer.size.x,"{text}: {rect:?} outside {outer:?}");
            }
        }
    }
}
