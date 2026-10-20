use crate::agent_access::{AgentAccessAction, ui::AgentAccessPanelWidgetExt};

use makepad_widgets::*;
use super::speech_settings::SpeechSettingsWidgetExt;
use crate::shared::navigation_bar_button::NavigationBarButtonWidgetExt;

use crate::{app::AppState, home::navigation_tab_bar::{NavigationBarAction, SelectedTab, get_own_profile}, profile::user_profile::UserProfile, settings::{PopulateMode, account_settings::AccountSettingsWidgetExt, app_settings::AppSettingsWidgetExt, privacy_settings::PrivacySettingsWidgetExt}};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.SettingsCategoryItem = NavigationBarButton {
        width: Fill height: 40 flow: Right spacing: 10
        align: Align{x: 0 y: 0.5} padding: Inset{left: 12 right: 12}
        draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_SELECTED}
        icon := Icon {icon_walk: Walk{width: 18 height: 18} draw_icon.color: mod.widgets.RINX_INK}
        label := RinxLabel {
            width: Fill max_lines: 1 text_overflow: Ellipsis
            draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: mod.widgets.RINX_UI_SIZE}}
        }
    }

    // One content tree preserves edits and scroll positions when categories or
    // window sizes change. Narrow windows use the same menu with icons only.
    mod.widgets.SettingsScreen = #(SettingsScreen::register_widget(vm)) {
        width: Fill height: Fill flow: Overlay
        main_content := SolidView {
            width: Fill height: Fill flow: Right
            draw_bg.color: mod.widgets.RINX_PAGE
            settings_navigation := SolidView {
                width: 184 height: Fill flow: Down
                padding: Inset{bottom: 12}
                draw_bg.color: mod.widgets.RINX_SURFACE
                navigation_header := View {
                    width: Fill height: 64 align: Align{y: 0.5} padding: Inset{left: 22 right: 22}
                    navigation_title := RinxLabel {
                        text: #(crate::i18n::tr("Settings")) i18n_text: "Settings"
                        draw_text.text_style: theme.font_bold {font_size: mod.widgets.RINX_HEADING_SIZE}
                    }
                    navigation_icon := View {
                        visible: false width: 18 height: 18
                        Icon {icon_walk: Walk{width: 18 height: 18} draw_icon +: {svg: ICON_SETTINGS color: mod.widgets.RINX_INK}}
                    }
                }
                settings_category_menu := ScrollYView {
                    width: Fill height: Fill flow: Down spacing: 4 padding: Inset{left: 10 right: 10}
                    category_account_button := mod.widgets.SettingsCategoryItem {
                        tooltip_text: #(crate::i18n::tr("Account"))
                        icon.draw_icon.svg: ICON_PEOPLE
                        label +: {text: #(crate::i18n::tr("Account")) i18n_text: "Account"}
                    }
                    category_general_button := mod.widgets.SettingsCategoryItem {
                        tooltip_text: #(crate::i18n::tr("General"))
                        icon.draw_icon.svg: ICON_SETTINGS
                        label +: {text: #(crate::i18n::tr("General")) i18n_text: "General"}
                    }
                    category_interface_button := mod.widgets.SettingsCategoryItem {
                        tooltip_text: #(crate::i18n::tr("Interface"))
                        icon.draw_icon.svg: crate_resource("self://resources/icons/article_palette.svg")
                        label +: {text: #(crate::i18n::tr("Interface")) i18n_text: "Interface"}
                    }
                    category_messages_button := mod.widgets.SettingsCategoryItem {
                        tooltip_text: #(crate::i18n::tr("Messages"))
                        icon.draw_icon.svg: ICON_THREADS
                        label +: {text: #(crate::i18n::tr("Messages")) i18n_text: "Messages"}
                    }
                    category_agents_button := mod.widgets.SettingsCategoryItem {
                        tooltip_text: #(crate::i18n::tr("Agent services"))
                        icon.draw_icon.svg: crate_resource("self://resources/icons/robot.svg")
                        label +: {text: #(crate::i18n::tr("Agent services")) i18n_text: "Agent services"}
                    }
                    category_speech_button := mod.widgets.SettingsCategoryItem {
                        tooltip_text: #(crate::i18n::tr("Speech"))
                        icon.draw_icon.svg: crate_resource("self://resources/icons/microphone.svg")
                        label +: {text: #(crate::i18n::tr("Speech")) i18n_text: "Speech"}
                    }
                    category_privacy_button := mod.widgets.SettingsCategoryItem {
                        tooltip_text: #(crate::i18n::tr("Privacy"))
                        icon.draw_icon.svg: ICON_SHIELD
                        label +: {text: #(crate::i18n::tr("Privacy")) i18n_text: "Privacy"}
                    }
                    category_about_button := mod.widgets.SettingsCategoryItem {
                        tooltip_text: #(crate::i18n::tr("About"))
                        icon.draw_icon.svg: ICON_INFO
                        label +: {text: #(crate::i18n::tr("About")) i18n_text: "About"}
                    }
                }
            }
            navigation_divider := SolidView {width: 1 height: Fill draw_bg.color: mod.widgets.RINX_BORDER}
            settings_detail := View {
                width: Fill height: Fill flow: Down
                settings_header := View {
                    width: Fill height: 64 flow: Right spacing: 12
                    padding: Inset{left: 24 right: 16} align: Align{y: 0.5}
                    settings_header_title := RinxLabel {
                        width: Fill text: #(crate::i18n::tr("Account"))
                        draw_text.text_style: theme.font_bold {font_size: mod.widgets.RINX_TITLE_SIZE}
                    }
                    close_button := RinxIconButton {draw_icon.svg: ICON_CLOSE}
                }
                header_divider := SolidView {width: Fill height: 1 draw_bg.color: mod.widgets.RINX_BORDER}
                settings_sections_wrapper := View {
                    width: Fill height: Fill
                    settings_sections := PageFlip {
                        width: Fill height: Fill active_page: @account_settings_page
                        account_settings_page := ScrollYView {
                            width: Fill height: Fill flow: Down padding: Inset{left: 24 right: 24 top: 20}
                            account_settings := AccountSettings {width: Fill{max: 880} section_title.visible: false}
                            tsp_settings_screen := TspSettingsScreen {}
                            View {width: Fill height: 24}
                        }
                        general_settings_page := ScrollYView {
                            width: Fill height: Fill flow: Down padding: Inset{left: 24 right: 24 top: 20}
                            general_settings := GeneralSettings {width: Fill{max: 880}}
                            View {width: Fill height: 24}
                        }
                        interface_settings_page := ScrollYView {
                            width: Fill height: Fill flow: Down padding: Inset{left: 24 right: 24 top: 20}
                            interface_settings := InterfaceSettings {width: Fill{max: 880}}
                            View {width: Fill height: 24}
                        }
                        messages_settings_page := ScrollYView {
                            width: Fill height: Fill flow: Down padding: Inset{left: 24 right: 24 top: 20}
                            messages_settings := MessageSettings {width: Fill{max: 880}}
                            View {width: Fill height: 24}
                        }
                        agents_settings_page := ScrollYView {
                            width: Fill height: Fill flow: Down padding: Inset{left: 24 right: 24 top: 20}
                            agents_settings := AgentServiceSettings {width: Fill{max: 880}}
                            View {width: Fill height: 24}
                        }
                        external_bots_settings_page := View {
                            width: Fill height: Fill
                            external_bots := AgentAccessPanel {}
                        }
                        speech_settings_page := ScrollYView {
                            width: Fill height: Fill flow: Down padding: Inset{left: 24 right: 24 top: 20}
                            speech_settings := SpeechSettings {width: Fill{max: 880}}
                            View {width: Fill height: 24}
                        }
                        privacy_settings_page := ScrollYView {
                            width: Fill height: Fill flow: Down padding: Inset{left: 24 right: 24 top: 20}
                            privacy_settings := PrivacySettings {width: Fill{max: 880} section_title.visible: false section_divider.visible: false}
                            View {width: Fill height: 24}
                        }
                        about_settings_page := ScrollYView {
                            width: Fill height: Fill flow: Down padding: Inset{left: 24 right: 24 top: 20}
                            about_settings := AboutSettings {width: Fill{max: 880} section_title.visible: false}
                            View {width: Fill height: 24}
                        }
                    }
                }
            }
        }

        // We want all modals to appear in front of the settings screen.
        create_wallet_modal := Modal {
            content := CreateWalletModal {}
        }

        create_did_modal := Modal {
            content := CreateDidModal {}
        }
    }
}

/// The settings categories, one per menu item and content page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SettingsCategory {
    #[default]
    Account,
    General,
    Interface,
    Messages,
    Agents,
    Speech,
    Privacy,
    About,
}

/// A settings category was selected; leave any management page opened above it.
#[derive(Clone, Debug)]
pub(crate) struct SettingsPageChanged;

/// The top-level widget showing all app and user settings/preferences.
#[derive(Script, Widget)]
pub struct SettingsScreen {
    #[deref] view: View,
    #[rust] selected_category: SettingsCategory,
    #[rust]
    external_bots_open: bool,
    #[rust]
    external_escape: bool,
}

impl ScriptHook for SettingsScreen {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| self.sync_selected_category(cx));
    }
}

impl Widget for SettingsScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.external_bots_open
            && scope.data.get::<AppState>().is_some_and(|a| a.selected_tab != SelectedTab::Settings)
        {
            self.close_external_bots(cx);
        }
        if matches!(event, Event::KeyUp(k) if k.key_code == KeyCode::Escape) && self.external_escape {
            self.external_escape = false;
            return;
        }
        if self.external_bots_open && matches!(event, Event::KeyDown(k) if k.key_code == KeyCode::Escape) {
            self.external_escape = true;
            self.close_external_bots(cx);
            return;
        }
        self.view.handle_event(cx, event, scope);

        // ScriptReapply preserves text fields (String / ArcStringMut bail out),
        // but still resets animator-driven controls and `script_apply_eval`-driven
        // bits (avatar, button colors, the selected category). Re-apply just those.
        // Never re-`set_text` user-editable inputs here, that would wipe in-progress edits.
        if matches!(event, Event::ScriptReapply | Event::LiveEdit) {
            if let Some(app_state) = scope.data.get::<AppState>() {
                self.populate_subwidgets(cx, PopulateMode::AfterReapply, None, app_state);
            }
            self.sync_selected_category(cx);
        }

        if let Event::Actions(actions) = event {
            if [ids!(category_account_button), ids!(category_general_button), ids!(category_interface_button),
                ids!(category_messages_button), ids!(category_agents_button), ids!(category_speech_button),
                ids!(category_privacy_button), ids!(category_about_button)]
                .iter().any(|id| self.view.navigation_bar_button(cx, *id).clicked(actions))
            {
                cx.action(SettingsPageChanged);
            }
            for action in actions {
                if action.downcast_ref::<crate::desktop_integration::OpenDesktopSettings>().is_some() {
                    self.set_selected_category(cx, SettingsCategory::General);
                }
                if let Some(action) = action.downcast_ref::<AgentAccessAction>() {
                    match action {
                        AgentAccessAction::Open if scope.data.get::<AppState>().is_some_and(|a| a.selected_tab == SelectedTab::Settings) => {
                            let state = scope.data.get::<AppState>().unwrap();
                            self.view.agent_access_panel(cx, ids!(external_bots)).action(cx, action, &state.agent_access);
                            self.external_bots_open = true;
                            self.selected_category = SettingsCategory::Agents;
                            self.sync_selected_category(cx);
                        }
                        AgentAccessAction::Close => self.close_external_bots(cx),
                        _ => {}
                    }
                }
            }
            if self.view.navigation_bar_button(cx, ids!(category_account_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Account);
            } else if self.view.navigation_bar_button(cx, ids!(category_general_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::General);
            } else if self.view.navigation_bar_button(cx, ids!(category_interface_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Interface);
            } else if self.view.navigation_bar_button(cx, ids!(category_messages_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Messages);
            } else if self.view.navigation_bar_button(cx, ids!(category_agents_button)).clicked(actions) {
                self.close_external_bots(cx);
                self.set_selected_category(cx, SettingsCategory::Agents);
            } else if self.view.navigation_bar_button(cx, ids!(category_speech_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Speech);
            } else if self.view.navigation_bar_button(cx, ids!(category_privacy_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Privacy);
            } else if self.view.navigation_bar_button(cx, ids!(category_about_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::About);
            }
        }

        // Close the pane if:
        // 1. The close button is clicked,
        // 2. The back navigational gesture/action occurs (e.g., Back on Android),
        // 3. The escape key is pressed if this pane has key focus,
        // 4. The back mouse button is clicked while this settings view is actively shown.
        let area = self.view.area();
        let close_pane = {
            matches!(
                event,
                Event::Actions(actions) if self.button(cx, ids!(close_button)).clicked(actions)
            )
            || (
                scope.data.get::<AppState>().is_some_and(|a| a.selected_tab == SelectedTab::Settings)
                && event.back_pressed()
            )
            || match event.hits(cx, area) {
                Hit::KeyUp(key) => key.key_code == KeyCode::Escape,
                Hit::FingerDown(_fde) => {
                    cx.set_key_focus(area);
                    false
                }
                _ => false,
            }
        };
        if close_pane {
            if self.external_bots_open && !matches!(event, Event::Actions(actions) if self.button(cx, ids!(close_button)).clicked(actions)) {
                self.close_external_bots(cx);
            } else {
                self.close_external_bots(cx);
                cx.action(NavigationBarAction::CloseSettings);
            }
        }

        #[cfg(feature = "tsp")]
        if let Event::Actions(actions) = event {
            use crate::tsp::{
                create_did_modal::CreateDidModalAction,
                create_wallet_modal::CreateWalletModalAction,
            };

            for action in actions {
                // Handle the create wallet modal being opened or closed.
                match action.downcast_ref() {
                    Some(CreateWalletModalAction::Open) => {
                        use crate::tsp::create_wallet_modal::CreateWalletModalWidgetExt;
                        self.view.create_wallet_modal(cx, ids!(create_wallet_modal.content)).show(cx);
                        self.view.modal(cx, ids!(create_wallet_modal)).open(cx);
                    }
                    Some(CreateWalletModalAction::Close) => {
                        self.view.modal(cx, ids!(create_wallet_modal)).close(cx);
                    }
                    None => { }
                }

                // Handle the create DID modal being opened or closed.
                match action.downcast_ref() {
                    Some(CreateDidModalAction::Open) => {
                        use crate::tsp::create_did_modal::CreateDidModalWidgetExt;
                        self.view.create_did_modal(cx, ids!(create_did_modal.content)).show(cx);
                        self.view.modal(cx, ids!(create_did_modal)).open(cx);
                    }
                    Some(CreateDidModalAction::Close) => {
                        self.view.modal(cx, ids!(create_did_modal)).close(cx);
                    }
                    None => { }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let compact = cx.peek_walk_turtle(walk).size.x < 600.0;
        if let Some(mut navigation) = self.view.view(cx, ids!(settings_navigation)).borrow_mut() {
            navigation.walk.width = Size::Fixed(if compact {64.0} else {184.0});
        }
        self.view.label(cx, ids!(navigation_title)).set_visible(cx, !compact);
        self.view.view(cx, ids!(navigation_icon)).set_visible(cx, compact);
        for id in [id!(category_account_button), id!(category_general_button), id!(category_interface_button), id!(category_messages_button), id!(category_agents_button), id!(category_speech_button), id!(category_privacy_button), id!(category_about_button)] {
            self.view.navigation_bar_button(cx, &[id]).label(cx, ids!(label)).set_visible(cx, !compact);
        }
        for id in [id!(settings_header), id!(account_settings_page), id!(general_settings_page), id!(interface_settings_page), id!(messages_settings_page), id!(agents_settings_page), id!(speech_settings_page), id!(privacy_settings_page), id!(about_settings_page)] {
            if let Some(mut view) = self.view.view(cx, &[id]).borrow_mut() {
                view.layout.padding.left = if compact {12.0} else {24.0};
                view.layout.padding.right = if compact {12.0} else {24.0};
            }
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl SettingsScreen {
    /// Fetches the current user's profile and uses it to populate the settings screen.
    pub fn populate(&mut self, cx: &mut Cx, own_profile: Option<UserProfile>, app_state: &AppState) {
        let Some(profile) = own_profile.or_else(|| get_own_profile(cx)) else {
            error!("Failed to get own profile for settings screen.");
            return;
        };
        self.populate_subwidgets(cx, PopulateMode::Initial, Some(profile), app_state);
        self.view.button(cx, ids!(close_button)).reset_hover(cx);
        self.sync_selected_category(cx);
        cx.set_key_focus(self.view.area());
        self.redraw(cx);
    }

    /// Single place deciding which sub-widgets get (re)synced and how.
    /// Both the initial-open and `Event::ScriptReapply` paths route here.
    ///
    /// `AppSettings` is missing from `AfterReapply` on purpose, since it
    /// restores itself inline from `on_after_apply` to avoid the flicker
    /// the late path used to produce. `AccountSettings` still needs the
    /// late path for its `script_apply_eval`-driven bits (button colors,
    /// avatar), cuz those can't run from inside `on_after_apply`.
    fn populate_subwidgets(
        &mut self,
        cx: &mut Cx,
        mode: PopulateMode,
        profile: Option<UserProfile>,
        app_state: &AppState,
    ) {
        match mode {
            PopulateMode::Initial => {
                self.view.account_settings(cx, ids!(account_settings)).populate(cx, profile);
                for id in [id!(general_settings), id!(interface_settings), id!(messages_settings), id!(agents_settings)] {
                    self.view.app_settings(cx, &[id]).populate(cx, &app_state.app_prefs);
                }
                self.view.speech_settings(cx, ids!(speech_settings)).populate(cx, &app_state.app_prefs.speech);
                self.view.privacy_settings(cx, ids!(privacy_settings)).populate(cx);
            }
            PopulateMode::AfterReapply => {
                self.view.account_settings(cx, ids!(account_settings)).restore_after_reapply(cx);
            }
        }
    }

    fn close_external_bots(&mut self, cx: &mut Cx) {
        self.view.agent_access_panel(cx, ids!(external_bots)).action(cx, &AgentAccessAction::Close, &Default::default());
        self.external_bots_open = false;
        self.sync_selected_category(cx);
    }

    fn set_selected_category(&mut self, cx: &mut Cx, category: SettingsCategory) {
        if category != SettingsCategory::Agents && self.external_bots_open {
            self.close_external_bots(cx);
        }
        self.selected_category = category;
        self.sync_selected_category(cx);
    }

    /// Keeps menu selection, detail heading and content page in sync.
    fn sync_selected_category(&mut self, cx: &mut Cx) {
        let page = match self.selected_category {
            SettingsCategory::Account => id!(account_settings_page),
            SettingsCategory::General => id!(general_settings_page),
            SettingsCategory::Interface => id!(interface_settings_page),
            SettingsCategory::Messages => id!(messages_settings_page),
            SettingsCategory::Agents if self.external_bots_open => id!(external_bots_settings_page),
            SettingsCategory::Agents => id!(agents_settings_page),
            SettingsCategory::Speech => id!(speech_settings_page),
            SettingsCategory::Privacy => id!(privacy_settings_page),
            SettingsCategory::About => id!(about_settings_page),
        };
        self.view.page_flip(cx, ids!(settings_sections)).set_active_page(cx, page);

        let heading = match self.selected_category {
            SettingsCategory::Account => "Account",
            SettingsCategory::General => "General",
            SettingsCategory::Interface => "Interface",
            SettingsCategory::Messages => "Messages",
            SettingsCategory::Agents if self.external_bots_open => "External bots and AppServices",
            SettingsCategory::Agents => "Agent services",
            SettingsCategory::Speech => "Speech",
            SettingsCategory::Privacy => "Privacy",
            SettingsCategory::About => "About",
        };
        self.view.label(cx, ids!(settings_header_title)).set_text(cx, crate::i18n::tr(heading));
        let tabs = [
            (SettingsCategory::Account, ids!(category_account_button)),
            (SettingsCategory::General, ids!(category_general_button)),
            (SettingsCategory::Interface, ids!(category_interface_button)),
            (SettingsCategory::Messages, ids!(category_messages_button)),
            (SettingsCategory::Agents, ids!(category_agents_button)),
            (SettingsCategory::Speech, ids!(category_speech_button)),
            (SettingsCategory::Privacy, ids!(category_privacy_button)),
            (SettingsCategory::About, ids!(category_about_button)),
        ];
        for (category, id) in tabs {
            self.view.navigation_bar_button(cx, id).set_selected(cx, category == self.selected_category);
        }
        self.redraw(cx);
    }
}

impl SettingsScreenRef {
    pub fn open_personal_info(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_selected_category(cx, SettingsCategory::Account);
        }
    }

    /// See [`SettingsScreen::populate()`].
    pub fn populate(&self, cx: &mut Cx, own_profile: Option<UserProfile>, app_state: &AppState) {
        let Some(mut inner) = self.borrow_mut() else { return; };
        inner.populate(cx, own_profile, app_state);
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use makepad_widgets::makepad_draw::cx_draw::CxDraw;

    fn screen(cx: &mut Cx) -> SettingsScreen {
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            vm.bx.captured_errors = Some(Vec::new());
            let value = script_eval!(vm, {mod.widgets.SettingsScreen {}});
            let screen = SettingsScreen::script_from_value(vm, value);
            assert!(vm.take_errors().is_empty());
            screen
        })
    }

    fn draw(cx: &mut Cx, screen: &mut SettingsScreen, width: f64) {
        cx.redraw_all();
        let pass = DrawPass::new(cx);
        pass.set_size(cx, dvec2(width, 760.0));
        let mut list = DrawList2d::new(cx);
        let overlay = Overlay {draw_list: DrawList::new(cx)};
        let event = DrawEvent::default();
        let mut draw = CxDraw::new(cx, &event);
        let mut cx2d = Cx2d::new(&mut draw);
        cx2d.begin_pass(&pass, None);
        list.begin_always(&mut cx2d);
        overlay.begin(&mut cx2d);
        cx2d.begin_root_turtle(dvec2(width, 760.0), Layout::flow_overlay());
        screen.draw_walk_all(&mut cx2d, &mut Scope::empty(), Walk {
            width: Size::Fixed(width), height: Size::Fixed(760.0), ..Walk::default()
        });
        overlay.end(&mut cx2d);
        cx2d.end_pass_sized_turtle();
        list.end(&mut cx2d);
        cx2d.end_pass(&pass);
    }

    #[cfg(any(target_os = "macos", all(target_os = "linux", not(target_env = "ohos"))))]
    #[test]
    fn desktop_updates_settings_offer_download_and_manual_check_without_install_controls() {
        use crate::updates::{Phase, Snapshot, UpdateState};
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut screen = screen(&mut cx);
        screen.set_selected_category(&mut cx, SettingsCategory::About);
        cx.global::<UpdateState>().0 = Snapshot {
            enabled: true, phase: Phase::Available, version: "0.7.0".into(), ..Default::default()
        };
        for width in [1100.0, 375.0] {
            draw(&mut cx, &mut screen, width);
            let updates = screen.view.view(&cx, ids!(about_settings.updates));
            assert!(!updates.is_empty());
            let download = screen.view.button(&cx, ids!(about_settings.updates.download));
            assert!(download.visible());
            let rect = download.area().rect(&cx);
            assert!(rect.size.x > 0.0 && rect.pos.x + rect.size.x <= width + 0.1);
            assert!(screen.view.button(&cx, ids!(about_settings.updates.check)).visible());
            assert!(!screen.view.button(&cx, ids!(about_settings.updates.install)).visible());
            assert!(screen.view.check_box(&cx, ids!(about_settings.updates.auto_check)).visible());
            assert!(!screen.view.check_box(&cx, ids!(about_settings.updates.auto_download)).visible());
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn updates_settings_show_ready_and_deferred_installation_in_narrow_windows() {
        use crate::updates::{Phase, Snapshot, UpdateState};
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut screen = screen(&mut cx);
        screen.set_selected_category(&mut cx, SettingsCategory::About);
        cx.global::<UpdateState>().0 = Snapshot {
            enabled: true, phase: Phase::Ready, version: "0.7.0".into(),
            notes: "A verified update with release notes.".into(), ..Default::default()
        };
        for width in [1100.0, 375.0] {
            draw(&mut cx, &mut screen, width);
            let button = screen.view.button(&cx, ids!(about_settings.updates.install));
            assert!(button.visible());
            let rect = button.area().rect(&cx);
            assert!(rect.size.x > 0.0 && rect.pos.x + rect.size.x <= width + 0.1);
            assert!(!screen.view.button(&cx, ids!(about_settings.updates.later)).visible());
        }
        cx.global::<UpdateState>().0.phase = Phase::Waiting;
        draw(&mut cx, &mut screen, 375.0);
        assert!(!screen.view.button(&cx, ids!(about_settings.updates.install)).visible());
        assert!(screen.view.button(&cx, ids!(about_settings.updates.later)).visible());
        assert!(screen.view.label(&cx, ids!(about_settings.updates.status)).text().contains("Agent tasks"));
    }

    #[test]
    fn tray_settings_open_general_and_fit_narrow_windows() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut screen = screen(&mut cx);
        let mut state = AppState { selected_tab: SelectedTab::Settings, ..Default::default() };
        cx.global::<crate::desktop_integration::DesktopSettings>().enabled = true;
        let actions = cx.capture_actions(|cx| cx.action(crate::desktop_integration::OpenDesktopSettings));
        screen.handle_event(&mut cx, &Event::Actions(actions), &mut Scope::with_data(&mut state));
        assert_eq!(screen.selected_category, SettingsCategory::General);
        for width in [1100.0, 375.0] {
            draw(&mut cx, &mut screen, width);
            let detail = screen.view.view(&cx, ids!(settings_detail)).area().rect(&cx);
            let card = screen.view.view(&cx, ids!(general_settings.desktop_settings));
            assert!(card.visible());
            for id in [id!(autostart_toggle), id!(start_hidden_toggle), id!(minimize_to_tray_toggle), id!(close_to_tray_toggle)] {
                let control = screen.view.check_box(&cx, &[id!(general_settings), id]);
                let rect = control.area().rect(&cx);
                assert!(rect.size.x > 0.0 && rect.size.y > 0.0);
                assert!(rect.pos.x >= detail.pos.x && rect.pos.x + rect.size.x <= width + 0.1);
            }
        }
        cx.global::<crate::desktop_integration::DesktopSettings>().enabled = false;
        draw(&mut cx, &mut screen, 1100.0);
        assert!(!screen.view.view(&cx, ids!(general_settings.desktop_settings)).visible());
    }

    #[test]
    fn external_bots_stay_inside_settings_navigation_and_back_returns_to_agent_services() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut screen=screen(&mut cx);
        let mut state = AppState { selected_tab: SelectedTab::Settings, ..Default::default() };
        screen.set_selected_category(&mut cx,SettingsCategory::Agents);
        let actions=cx.capture_actions(|cx|cx.action(AgentAccessAction::Open));
        screen.handle_event(&mut cx,&Event::Actions(actions),&mut Scope::with_data(&mut state));
        assert!(screen.external_bots_open);
        assert_eq!(screen.view.label(&cx,ids!(settings_header_title)).text(),"External bots and AppServices");
        assert_eq!(screen.view.label(&cx,ids!(linked_bots_heading)).text(),"Linked external bots");
        assert!(screen.view.label(&cx,ids!(external_bots_intro)).text().contains("main sidebar"));
        let input=screen.view.text_input(&cx,ids!(external_bots.agent_id));input.set_text(&mut cx,"@external:example.org");
        for width in [1100.0,375.0] {
            draw(&mut cx,&mut screen,width);
            let nav=screen.view.view(&cx,ids!(settings_navigation)).area().rect(&cx);
            let panel=screen.view.widget(&cx,ids!(external_bots)).area().rect(&cx);
            assert!(nav.size.x>=64.0);assert!(panel.pos.x>=nav.pos.x+nav.size.x);
            assert!(panel.pos.x+panel.size.x<=width+1.0);
        }
        screen.handle_event(&mut cx,&Event::ScriptReapply,&mut Scope::with_data(&mut state));
        assert!(screen.external_bots_open);assert_eq!(input.text(),"@external:example.org");
        let actions=cx.capture_actions(|cx|cx.action(AgentAccessAction::Close));
        screen.handle_event(&mut cx,&Event::Actions(actions),&mut Scope::with_data(&mut state));
        assert!(!screen.external_bots_open);assert_eq!(state.selected_tab,SelectedTab::Settings);
        assert_eq!(screen.view.label(&cx,ids!(settings_header_title)).text(),"Agent services");
        let actions=cx.capture_actions(|cx|cx.action(AgentAccessAction::Open));
        screen.handle_event(&mut cx,&Event::Actions(actions),&mut Scope::with_data(&mut state));
        assert!(screen.external_bots_open);
        state.selected_tab=SelectedTab::Home;
        screen.handle_event(&mut cx,&Event::Actions(Vec::new()),&mut Scope::with_data(&mut state));
        assert!(!screen.external_bots_open);
        assert_eq!(crate::i18n::text_for(crate::i18n::Language::Chinese,"Linked external bots"),"已关联的外部机器人");
    }

    #[test]
    fn category_menu_switches_detail_and_preserves_unsaved_inputs() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut screen = screen(&mut cx);
        let mut state = AppState::default();
        let input = screen.view.text_input(&cx, ids!(display_name_input));
        assert!(!input.is_empty());
        input.set_text(&mut cx, "Unsaved display name");
        let input_uid = input.widget_uid();
        let thumbnail = screen.view.text_input(&cx, ids!(interface_settings.thumb_custom_input));
        assert!(!thumbnail.is_empty());
        thumbnail.set_text(&mut cx, "425");
        let thumbnail_uid = thumbnail.widget_uid();
        for (category, button, heading) in [
            (SettingsCategory::General, id!(category_general_button), "General"),
            (SettingsCategory::Interface, id!(category_interface_button), "Interface"),
            (SettingsCategory::Messages, id!(category_messages_button), "Messages"),
            (SettingsCategory::Agents, id!(category_agents_button), "Agent services"),
            (SettingsCategory::Speech, id!(category_speech_button), "Speech"),
            (SettingsCategory::Privacy, id!(category_privacy_button), "Privacy"),
            (SettingsCategory::About, id!(category_about_button), "About"),
            (SettingsCategory::Account, id!(category_account_button), "Account"),
        ] {
            let uid = screen.view.navigation_bar_button(&cx, &[button]).widget_uid();
            let actions = cx.capture_actions(|cx| cx.widget_action(uid,
                crate::shared::navigation_bar_button::NavigationBarButtonAction::Clicked));
            screen.handle_event(&mut cx, &Event::Actions(actions), &mut Scope::with_data(&mut state));
            assert_eq!(screen.selected_category, category);
            assert_eq!(screen.view.label(&cx, ids!(settings_header_title)).text(), crate::i18n::tr(heading));
            for id in [id!(category_account_button), id!(category_general_button), id!(category_interface_button), id!(category_messages_button), id!(category_agents_button), id!(category_speech_button), id!(category_privacy_button), id!(category_about_button)] {
                let item = screen.view.navigation_bar_button(&cx, &[id]);
                assert_eq!(item.borrow().unwrap().animator_in_state(&cx, ids!(active.on)), id == button);
            }
            screen.handle_event(&mut cx, &Event::ScriptReapply, &mut Scope::with_data(&mut state));
            assert_eq!(screen.view.text_input(&cx, ids!(display_name_input)).widget_uid(), input_uid);
            assert_eq!(input.text(), "Unsaved display name");
            assert_eq!(screen.view.text_input(&cx, ids!(interface_settings.thumb_custom_input)).widget_uid(), thumbnail_uid);
            assert_eq!(thumbnail.text(), "425");
        }
    }

    #[test]
    fn separated_preference_controls_keep_their_existing_actions() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let _window = WindowHandle::new(&mut cx);
        let mut screen = screen(&mut cx);
        let mut state = AppState::default();
        let original = state.app_prefs.clone();
        let groups = [
            (id!(general_settings), id!(send_on_cmd_enter_toggle)),
            (id!(interface_settings), id!(ui_zoom_plus_button)),
            (id!(messages_settings), id!(read_receipts_privacy_dropdown)),
            (id!(agents_settings), id!(agent_access)),
        ];
        for (group, control) in groups {
            let panel = screen.view.app_settings(&cx, &[group]);
            for (_, other) in groups {
                assert_eq!(!panel.widget(&cx, &[other]).is_empty(), other == control,
                    "each control must belong to exactly one category");
            }
            panel.populate(&mut cx, &state.app_prefs);
        }
        screen.set_selected_category(&mut cx, SettingsCategory::General);
        let toggle = screen.view.check_box(&cx, ids!(general_settings.send_on_cmd_enter_toggle));
        let actions = cx.capture_actions(|cx| cx.widget_action(toggle.widget_uid(), CheckBoxAction::Change(true)));
        screen.handle_event(&mut cx, &Event::Actions(actions), &mut Scope::with_data(&mut state));
        assert!(!state.app_prefs.send_on_enter);
        screen.set_selected_category(&mut cx, SettingsCategory::Interface);
        let plus = screen.view.button(&cx, ids!(interface_settings.ui_zoom_plus_button));
        let actions = cx.capture_actions(|cx| cx.widget_action(plus.widget_uid(), ButtonAction::Clicked(KeyModifiers::default())));
        screen.handle_event(&mut cx, &Event::Actions(actions), &mut Scope::with_data(&mut state));
        assert_eq!(state.app_prefs.ui_zoom, original.ui_zoom.zoom_in_by(crate::settings::app_preferences::UiZoom::BUTTON_STEP));
        assert_eq!(state.app_prefs.speech, original.speech);
        assert_eq!(state.app_prefs.read_receipts_privacy, original.read_receipts_privacy);
    }

    #[test]
    fn scrolling_viewport_reaches_panel_edge_while_content_keeps_its_gutter() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut screen = screen(&mut cx);
        screen.set_selected_category(&mut cx, SettingsCategory::Speech);
        for (width, gutter) in [(1100.0, 24.0), (375.0, 12.0)] {
            draw(&mut cx, &mut screen, width);
            let detail = screen.view.view(&cx, ids!(settings_detail)).area().rect(&cx);
            let page = screen.view.view(&cx, ids!(speech_settings_page)).area().rect(&cx);
            let content = screen.view.widget(&cx, ids!(speech_settings)).area().rect(&cx);
            assert!((page.pos.x - detail.pos.x).abs() < 0.1);
            assert!((page.pos.x + page.size.x - detail.pos.x - detail.size.x).abs() < 0.1,
                "the scroll track must use the panel's outer edge");
            assert!((content.pos.x - page.pos.x - gutter).abs() < 0.1,
                "content keeps its readable inset independently of the scrollbar");
            let menu = screen.view.view(&cx, ids!(settings_navigation)).area().rect(&cx);
            let list = screen.view.view(&cx, ids!(settings_category_menu)).area().rect(&cx);
            assert!((list.pos.x + list.size.x - menu.pos.x - menu.size.x).abs() < 0.1);
        }
    }

    #[test]
    fn category_menu_and_detail_fit_wide_and_narrow_windows() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut screen = screen(&mut cx);
        for category in [SettingsCategory::General, SettingsCategory::Interface, SettingsCategory::Messages, SettingsCategory::Agents] {
            screen.set_selected_category(&mut cx, category);
            for (width, menu_width, labels) in [(1100.0, 184.0, true), (375.0, 64.0, false), (720.0, 184.0, true)] {
                draw(&mut cx, &mut screen, width);
                let menu = screen.view.view(&cx, ids!(settings_navigation)).area().rect(&cx);
                let detail = screen.view.view(&cx, ids!(settings_detail)).area().rect(&cx);
                assert_eq!(menu.size.x, menu_width);
                assert_eq!(detail.pos.x, menu_width + 1.0);
                assert!(detail.size.x > 0.0 && detail.pos.x + detail.size.x <= width + 0.1);
                let mut last_y = 0.0;
                for id in [id!(category_account_button), id!(category_general_button), id!(category_interface_button), id!(category_messages_button), id!(category_agents_button), id!(category_speech_button), id!(category_privacy_button), id!(category_about_button)] {
                    let item = screen.view.navigation_bar_button(&cx, &[id]);
                    let rect = item.area().rect(&cx);
                    let icon = item.icon(&cx, ids!(icon)).area().rect(&cx);
                    assert!(rect.pos.y >= last_y && rect.size.y == 40.0);
                    assert!(icon.size.x > 0.0 && icon.pos.x >= menu.pos.x && icon.pos.x + icon.size.x <= menu.pos.x + menu.size.x);
                    assert_eq!(item.label(&cx, ids!(label)).visible(), labels);
                    last_y = rect.pos.y + rect.size.y;
                }
            }
        }
    }
}
