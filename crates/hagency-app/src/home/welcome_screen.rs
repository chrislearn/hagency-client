use makepad_widgets::*;
use crate::home::navigation_tab_bar::NavigationBarAction;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.WelcomeScreen = #(WelcomeScreen::register_widget(vm)) {
        width: Fill height: Fill flow: Down
        align: Align{x: 0.0 y: 0.5}
        padding: 40
        spacing: 20
        show_bg: true
        draw_bg.color: (COLOR_PRIMARY)
        title := RinxLabel {
            width: Fill height: Fit
            text: #(crate::i18n::tr("Your Hagency chats")) i18n_text: "Your Hagency chats"
            draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_bold {font_size: mod.widgets.RINX_DISPLAY_SIZE}}
        }
        description := RinxLabel {
            width: Fill height: Fit flow: Flow.Right{wrap: true}
            text: #(crate::i18n::tr("Choose a chat in the sidebar, start a conversation with a contact, or explore rooms on your server."))
            i18n_text: "Choose a chat in the sidebar, start a conversation with a contact, or explore rooms on your server."
            draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: mod.widgets.RINX_BODY_SIZE}}
        }
        View {width: Fill height: Fit flow: Right spacing: 12
            new_chat := RinxButton {text: #(crate::i18n::tr("New chat")) i18n_text: "New chat"}
            explore := RinxButton {text: #(crate::i18n::tr("Explore rooms")) i18n_text: "Explore rooms"}
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct WelcomeScreen {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
}
impl Widget for WelcomeScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        if let Event::Actions(actions) = event {
            if self.view.button(cx, ids!(new_chat)).clicked(actions) {
                cx.action(NavigationBarAction::NewChat);
            }
            if self.view.button(cx, ids!(explore)).clicked(actions) {
                cx.action(NavigationBarAction::GoToAddRoom);
            }
        }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chat_empty_state_has_real_new_chat_and_explore_actions() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut screen = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm); crate::app::register_widgets(vm);
            vm.bx.captured_errors = Some(Vec::new());
            let value = script_eval!(vm, {mod.widgets.WelcomeScreen {}});
            let screen = WelcomeScreen::script_from_value(vm, value);
            assert!(vm.take_errors().is_empty()); screen
        });
        assert_eq!(screen.view.label(&cx, ids!(title)).text(), crate::i18n::tr("Your Hagency chats"));
        for (button, contacts) in [(id!(new_chat), true), (id!(explore), false)] {
            let uid = screen.view.button(&cx, &[button]).widget_uid();
            let actions = cx.capture_actions(|cx| cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default())));
            let emitted = cx.capture_actions(|cx| screen.handle_event(cx, &Event::Actions(actions), &mut Scope::empty()));
            assert!(emitted.iter().any(|a| match a.downcast_ref::<NavigationBarAction>() {
                Some(NavigationBarAction::NewChat) => contacts,
                Some(NavigationBarAction::GoToAddRoom) => !contacts,
                _ => false,
            }));
        }
    }
}
