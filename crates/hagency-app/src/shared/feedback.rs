//! Persistent, local feedback. Errors explain the next step; diagnostics stay
//! available behind Details instead of competing with the user's task.
use makepad_widgets::*;
use super::popup_list::PopupKind;

pub(crate) fn colors(theme: &crate::theme::Snapshot, kind: PopupKind) -> (Vec4f, Vec4f) {
    let role = match kind {
        PopupKind::Error => "danger",
        PopupKind::Warning => "warning",
        PopupKind::Success => "success",
        PopupKind::Info => "info",
        PopupKind::Blank => return (theme.ink, theme.surface),
    };
    (
        theme.role(&format!("color.status.{role}.foreground")),
        theme.role(&format!("color.status.{role}.background")),
    )
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    mod.widgets.InlineNotice = set_type_default() do #(InlineNotice::register_widget(vm)) {
        ..mod.widgets.RoundedView
        visible: false width: Fill height: Fit flow: Down spacing: 8 padding: 14
        draw_bg +: {color: RINX_DANGER_BG border_color: RINX_DANGER_FG border_size: 1 border_radius: 8}
        header := View {width: Fill height: Fit flow: Right spacing: 10 align: Align{y: 0.5}
            symbol := Icon {icon_walk: Walk{width: 20 height: 20} draw_icon +: {svg: ICON_WARNING color: RINX_DANGER_FG}}
            title := RinxLabel {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text +: {color: RINX_DANGER_FG text_style: theme.font_bold{font_size: RINX_HEADING_SIZE}}}
            dismiss := RinxIconButton {visible: false draw_icon.svg: ICON_CLOSE}
        }
        message := RinxLabel {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text +: {color: RINX_DANGER_FG text_style: theme.font_regular{font_size: RINX_BODY_SIZE}}}
        next_action := RinxButton {visible: false}
        details_toggle := RinxGhostButton {visible: false text: #(crate::i18n::tr("Show details")) i18n_text: "Show details"}
        details := RinxLabel {visible: false width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text +: {color: RINX_DANGER_FG text_style: theme.font_regular{font_size: RINX_META_SIZE}}}
    }
}

#[derive(Script, Widget)]
pub struct InlineNotice {
    #[deref]
    view: View,
    #[rust]
    kind: PopupKind,
    #[rust]
    title: String,
    #[rust]
    message: String,
    #[rust]
    details: String,
    #[rust]
    expanded: bool,
    #[rust]
    restyle: bool,
    #[rust]
    shown: bool,
    #[rust]
    dismissible: bool,
    #[rust]
    action: String,
}
impl ScriptHook for InlineNotice {
    fn on_after_apply(
        &mut self,
        _vm: &mut ScriptVm,
        _apply: &Apply,
        _scope: &mut Scope,
        _value: ScriptValue,
    ) {
        self.restyle = true;
    }
}
impl InlineNotice {
    fn render(&mut self, cx: &mut Cx) {
        let theme = crate::theme::snapshot(cx);
        let (fg, bg) = colors(&theme, self.kind);
        let view = &mut self.view;
        view.set_visible(cx, self.shown);
        view.button(cx, ids!(dismiss))
            .set_visible(cx, self.dismissible);
        view.button(cx, ids!(next_action))
            .set_visible(cx, !self.action.is_empty());
        view.button(cx, ids!(next_action))
            .set_text(cx, crate::i18n::tr(&self.action));
        script_apply_eval!(cx,view,{draw_bg +: {color:#(bg) border_color:#(fg)}});
        let mut symbol = view.widget(cx, ids!(symbol));
        script_apply_eval!(cx,symbol,{draw_icon.color:#(fg)});
        if matches!(self.kind, PopupKind::Error | PopupKind::Warning) {
            script_apply_eval!(cx,symbol,{draw_icon.svg:mod.widgets.ICON_WARNING});
        } else if self.kind == PopupKind::Success {
            script_apply_eval!(cx,symbol,{draw_icon.svg:mod.widgets.ICON_CHECKMARK});
        } else {
            script_apply_eval!(cx,symbol,{draw_icon.svg:mod.widgets.ICON_INFO});
        }
        for (id, text) in [
            (ids!(title), self.title.as_str()),
            (ids!(message), self.message.as_str()),
            (ids!(details), self.details.as_str()),
        ] {
            view.label(cx, id).set_text(cx, crate::i18n::tr(text));
            view.label(cx, id).set_text_color(cx, fg);
        }
        view.label(cx, ids!(message))
            .set_visible(cx, !self.message.is_empty());
        view.button(cx, ids!(details_toggle))
            .set_visible(cx, !self.details.is_empty());
        view.button(cx, ids!(details_toggle)).set_text(
            cx,
            crate::i18n::tr(if self.expanded {
                "Hide details"
            } else {
                "Show details"
            }),
        );
        view.label(cx, ids!(details))
            .set_visible(cx, self.expanded && !self.details.is_empty());
        self.restyle = false;
    }
}
impl Widget for InlineNotice {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        if let Event::Actions(actions) = event {
            if self.view.button(cx, ids!(details_toggle)).clicked(actions) {
                self.expanded = !self.expanded;
                self.render(cx);
                self.view.redraw(cx);
            }
            if self.view.button(cx, ids!(dismiss)).clicked(actions) {
                self.shown = false;
                self.view.set_visible(cx, false);
                self.view.redraw(cx);
            }
        }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if self.restyle {
            self.render(cx);
        }
        self.view.draw_walk(cx, scope, walk)
    }
}
impl InlineNoticeRef {
    pub fn message(&self) -> String {
        self.borrow()
            .map(|inner| inner.message.clone())
            .unwrap_or_default()
    }
    pub fn is_visible(&self) -> bool {
        self.borrow().is_some_and(|inner| inner.view.visible())
    }
    pub fn set_action(&self, cx: &mut Cx, label: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            if inner.action == label {
                return;
            }
            inner.action = label.into();
            inner.render(cx);
            inner.view.redraw(cx);
        }
    }
    pub fn action_clicked(&self, cx: &mut Cx, actions: &Actions) -> bool {
        self.borrow().is_some_and(|inner| {
            inner.shown
                && !inner.action.is_empty()
                && inner.view.button(cx, ids!(next_action)).clicked(actions)
        })
    }
    pub fn show(
        &self,
        cx: &mut Cx,
        kind: PopupKind,
        title: &str,
        message: &str,
        details: &str,
        dismissible: bool,
    ) {
        if let Some(mut inner) = self.borrow_mut() {
            let changed =
                inner.message != message || inner.details != details || inner.title != title;
            if !changed
                && inner.kind == kind
                && inner.view.visible()
                && inner.view.button(cx, ids!(dismiss)).visible() == dismissible
            {
                return;
            }
            inner.kind = kind;
            inner.title = title.into();
            inner.message = message.into();
            inner.details = details.into();
            inner.shown = true;
            inner.dismissible = dismissible;
            if changed {
                inner.expanded = false;
                inner.action.clear();
            }
            inner
                .view
                .button(cx, ids!(dismiss))
                .set_visible(cx, dismissible);
            inner.view.set_visible(cx, true);
            inner.render(cx);
            inner.view.redraw(cx);
        }
    }
    pub fn clear(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            if !inner.shown && inner.title.is_empty() {
                return;
            }
            inner.shown = false;
            inner.title.clear();
            inner.message.clear();
            inner.details.clear();
            inner.action.clear();
            inner.expanded = false;
            inner.view.set_visible(cx, false);
            inner.view.redraw(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_details_are_collapsed_and_dismissal_survives_restyling() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut notice = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            vm.bx.captured_errors = Some(Vec::new());
            let value = script_eval!(vm,{mod.widgets.InlineNotice {}});
            let notice = InlineNotice::script_from_value(vm, value);
            assert!(vm.take_errors().is_empty());
            notice
        });
        notice.kind = PopupKind::Error;
        notice.title = "Could not create Agent".into();
        notice.message = "Check your connection.".into();
        notice.details = "invalid_arguments".into();
        notice.shown = true;
        notice.dismissible = true;
        notice.render(&mut cx);
        assert!(notice.view.visible());
        assert!(!notice.view.label(&cx, ids!(details)).visible());
        let button = notice.view.button(&cx, ids!(details_toggle));
        let actions = cx.capture_actions(|cx| {
            cx.widget_action(
                button.widget_uid(),
                ButtonAction::Clicked(KeyModifiers::default()),
            )
        });
        notice.handle_event(&mut cx, &Event::Actions(actions), &mut Scope::empty());
        assert!(notice.expanded);
        assert!(notice.view.label(&cx, ids!(details)).visible());
        let dismiss = notice.view.button(&cx, ids!(dismiss));
        let actions = cx.capture_actions(|cx| {
            cx.widget_action(
                dismiss.widget_uid(),
                ButtonAction::Clicked(KeyModifiers::default()),
            )
        });
        notice.handle_event(&mut cx, &Event::Actions(actions), &mut Scope::empty());
        assert!(!notice.shown);
        notice.render(&mut cx);
        assert!(!notice.view.visible());
    }
}
