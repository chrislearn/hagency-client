//! Shared account picker for the navigation menu, Settings and the login screen.
use makepad_widgets::*;
use crate::{
    accounts::{AccountAction, SavedAccount},
    sliding_sync::{account_changing, current_user_id},
};

fn picker_accounts(saved: Vec<SavedAccount>, active: Option<&matrix_sdk::ruma::UserId>, hide_current: bool) -> Vec<SavedAccount> {
    saved.into_iter().filter(|account| !hide_current || active != Some(account.user_id.as_ref())).collect()
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    let AccountPickerRow = RobrixIconButton {
        height: RINX_BUTTON_HEIGHT padding: Inset{left: 12 right: 12 top: 0 bottom: 0}
        width: Fill margin: 0 align: Align{x: 0 y: 0.5}
        draw_bg +: {color: mod.widgets.RINX_SURFACE color_hover: mod.widgets.RINX_HOVER color_down: mod.widgets.RINX_PRESSED border_size: 0 border_radius: 4}
        draw_icon.color: mod.widgets.RINX_ACCENT
        draw_text +: {color: mod.widgets.RINX_INK color_hover: mod.widgets.RINX_INK color_down: mod.widgets.RINX_INK text_style: mod.widgets.RBX_TEXT_BODY}
    }
    mod.widgets.AccountList = set_type_default() do #(AccountList::register_widget(vm)) {
        ..mod.widgets.View
        width: Fill, height: Fit, flow: Down, spacing: 4
        show_add: true
        hide_current: false
        account_container := View {
            width: Fill, height: Fit, flow: Down
            accounts := PortalList {
                width: Fill, height: 40, flow: Down
                Account := View {
                    width: Fill, height: 40
                    select := AccountPickerRow {
                        width: Fill, height: 40
                        icon_walk: Walk{width: 0, height: 0}
                        spacing: 0
                        text: ""
                    }
                }
            }
        }
        add := AccountPickerRow {
            width: Fill, height: 40
            icon_walk: Walk{width: 18, height: 18}
            spacing: 12 draw_icon.svg: ICON_ADD_USER
            text: #(crate::i18n::tr("Add Account")) i18n_text: "Add Account"
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct AccountList {
    #[deref]
    view: View,
    #[live(true)]
    show_add: bool,
    #[live(false)]
    hide_current: bool,
    #[rust]
    rows: Vec<SavedAccount>,
}
impl Widget for AccountList {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        if let Event::Actions(actions) = event {
            if account_changing() || crate::logout::logout_state_machine::is_logout_in_progress() {
                return;
            }
            if self.show_add && self.view.button(cx, ids!(add)).clicked(actions) {
                cx.action(AccountAction::Add);
            }
            for (index, row) in self
                .view
                .portal_list(cx, ids!(accounts))
                .items_with_actions(actions)
            {
                if row.button(cx, ids!(select)).clicked(actions) {
                    if let Some(account) = self.rows.get(index) {
                        cx.action(AccountAction::Select(account.user_id.clone()));
                    }
                }
            }
        }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let active = current_user_id();
        let saved = picker_accounts(crate::accounts::saved(), active.as_deref(), self.hide_current);
        if self.rows != saved {
            self.view.redraw(cx);
        }
        self.rows = saved;
        self.view
            .view(cx, ids!(account_container))
            .set_visible(cx, !self.rows.is_empty());
        let height = self.rows.len().min(4) as f64 * 40.0;
        let mut list = self.view.portal_list(cx, ids!(accounts));
        // Theme/language reapplication can restore the template height. Check
        // the actual walk rather than caching the last requested height.
        if list.walk(cx).height != Size::Fixed(height) {
            script_apply_eval!(cx, list, { height: #(height) });
        }
        self.view
            .button(cx, ids!(add))
            .set_visible(cx, self.show_add);
        self.view
            .button(cx, ids!(add))
            .set_enabled(cx, !account_changing());
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.borrow_mut::<PortalList>() {
                list.set_item_range(cx, 0, self.rows.len());
                while let Some(index) = list.next_visible_item(cx) {
                    let Some(account) = self.rows.get(index) else {
                        continue;
                    };
                    let row = list.item(cx, index, id!(Account));
                    let is_active = active.as_ref() == Some(&account.user_id);
                    let text = if is_active {
                        format!("{} · {}", account.user_id, crate::i18n::tr("Active"))
                    } else {
                        account.user_id.to_string()
                    };
                    row.button(cx, ids!(select)).set_text(cx, &text);
                    row.button(cx, ids!(select))
                        .set_enabled(cx, !is_active && !account_changing());
                    row.draw_all(cx, scope);
                }
            }
        }
        DrawStep::done()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_hides_only_current_account_and_login_picker_retains_all_identities() {
        let alice=matrix_sdk::ruma::user_id!("@alice:server");
        let bob=matrix_sdk::ruma::user_id!("@bob:server");
        let saved=vec![SavedAccount{user_id:alice.to_owned(),homeserver:"https://server/".into()},SavedAccount{user_id:bob.to_owned(),homeserver:"https://server/".into()}];
        assert_eq!(picker_accounts(saved.clone(),Some(alice),true),vec![saved[1].clone()]);
        assert_eq!(picker_accounts(saved.clone(),Some(alice),false),saved);
        assert_eq!(picker_accounts(saved.clone(),None,true),saved);
    }
    #[test]
    fn registered_account_row_keeps_add_action_and_menu_scoped_filter() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);crate::app::register_widgets(vm);vm.bx.captured_errors=Some(Vec::new());
            let value=script_eval!(vm,{mod.widgets.AccountList {hide_current: true}});
            let menu=AccountList::script_from_value(vm,value);
            assert!(menu.hide_current && menu.show_add);
            let value=script_eval!(vm,{mod.widgets.AccountList {}});
            let login=AccountList::script_from_value(vm,value);
            assert!(!login.hide_current && login.show_add);
            vm.with_cx_mut(|cx| assert!(!menu.view.button(cx,ids!(add)).is_empty()));
            assert!(vm.take_errors().is_empty());
        });
    }
}
