//! Read-only views built from the production widgets; no sign-in or backend.
//! Run with a disposable absolute HAGENCY_APP_DATA_DIR and MAKEPAD_HIDE_WINDOWS=1.
//! --scene=projects|agents|overview|create|login|settings|chats|contacts --width=1280 --dark --zh
#[cfg(debug_assertions)]
mod preview {
    use hagency_app::{makepad_widgets::*, hagency::ui::HagencyPanelWidgetRefExt};
    use hagency_app::app::{AppState,SavedDockState,SelectedRoom};
    use hagency_app::utils::RoomNameId;
    use hagency_app::home::{project_tree::ProjectTreeWidgetRefExt, mobile::MobileHubWidgetRefExt};
    app_main!(Preview);

    fn arg(name: &str, default: &str) -> String {
        std::env::args()
            .find_map(|a| a.strip_prefix(&format!("--{name}=")).map(str::to_owned))
            .unwrap_or_else(|| default.into())
    }
    script_mod! {
        use mod.prelude.widgets.*
        use mod.widgets.*
        startup() do #(Preview::script_component(vm)) {
            ui: Root {window := Window {
                window.inner_size: vec2(#(arg("width", "1280").parse::<f64>().unwrap()), 860)
                window.title: "Hagency · Native workspace review"
                body +: {
                    workspace := View {width:Fill height:Fill flow:Right
                        sidebar_holder := View {width:200 height:Fill sidebar := RoomsSideBar {}}
                        project_context_review := SolidView {visible:false width:240 height:Fill flow:Down padding:Inset{left:10 right:10 top:16 bottom:12} spacing:12 draw_bg.color:RINX_PANEL_BG
                            View {width:Fill height:28 flow:Right spacing:4 align:Align{y:0.5}
                                RinxLabel {width:Fill height:Fill align:Align{y:0.5} text:#(hagency_app::i18n::tr("Projects")) i18n_text:"Projects" draw_text.text_style:theme.font_bold{font_size:RINX_HEADING_SIZE}}
                                RinxIconButton {width:28 height:28 draw_icon.svg:crate_resource("self://resources/icons/article_refresh.svg")}
                                RinxIconButton {width:28 height:28 draw_icon.svg:ICON_ADD}
                            }
                            review_projects := View {width:Fill height:Fit flow:Down spacing:6
                                NavigationBarButton {width:Fill height:40 flow:Right spacing:8 padding:8 align:Align{y:0.5}
                                    Icon {icon_walk:Walk{width:16 height:16} draw_icon +: {svg:ICON_HIERARCHY color:RINX_ACCENT}}
                                    RinxLabel {width:Fill text:"Product design"}
                                }
                                NavigationBarButton {width:Fill height:36 padding:Inset{left:30} align:Align{y:0.5} RinxLabel {text:"Product roadmap"}}
                                NavigationBarButton {width:Fill height:40 flow:Right spacing:8 padding:8 align:Align{y:0.5}
                                    Icon {icon_walk:Walk{width:16 height:16} draw_icon +: {svg:ICON_HIERARCHY color:RINX_MUTED}}
                                    RinxLabel {width:Fill text:"Research & engineering"}
                                }
                            }
                        }
                        panel := HagencyPanel {}
                        dock_review := MainDesktopUI {visible:false}
                        chat_review := View {visible:false width:Fill height:Fill flow:Right
                            chat_list_pane := SolidView {width:#(if arg("width","1280").parse::<f64>().unwrap()>=900.0 {Size::Fixed(280.0)}else{Size::fill()}) height:Fill flow:Down draw_bg.color:RINX_SURFACE
                                RinxLabel {text:#(hagency_app::i18n::tr("Chats")) i18n_text:"Chats" padding:16 draw_text.text_style:theme.font_bold{font_size:RINX_HEADING_SIZE}}
                                chat_list := ProjectTree {}
                            }
                            chat_welcome := GroupChatsWelcome {}
                        }
                        contacts_review := MobileHub {visible:false}
                        usage_review := ScrollYView {visible:false width:Fill height:Fill flow:Down spacing:20 padding:16
                            RinxLabel {text:"Agent response usage · sample data" draw_text.text_style:theme.font_bold{font_size:RINX_HEADING_SIZE}}
                            usage_measured := Message {}
                            usage_missing := CondensedMessage {}
                            usage_human := Message {}
                            usage_mobile := MobileMessage {}
                        }

                    }
                    review_login := LoginScreen {visible:false}
                    review_settings := SettingsScreen {visible:false}
                }
            }}
        }
    }
    #[derive(Script, ScriptHook)]
    pub struct Preview {
        #[live]
        ui: WidgetRef,
        #[rust]
        loaded: bool,
        #[rust]
        state: AppState,
    }
    impl Preview {
        fn scene(&mut self, cx: &mut Cx, scene: &str) {
            let width=arg("width","1280").parse::<f64>().unwrap();
            hagency_app::home::home_screen::select_design_preview_mode(cx,width);
            self.ui.view(cx,ids!(project_context_review)).set_visible(cx,width>=900.0 && (matches!(scene,"projects"|"overview"|"create") || scene.starts_with("legacy-projects")));
            self.ui.view(cx,ids!(review_projects)).set_visible(cx,!scene.ends_with("-empty") && !scene.ends_with("-error"));
            self.ui
                .view(cx, ids!(workspace))
                .set_visible(cx, !matches!(scene, "login" | "settings"));
            self.ui
                .view(cx, ids!(sidebar_holder))
                .set_visible(cx, arg("width", "1280").parse::<f64>().unwrap() >= 900.0);
            self.ui
                .widget(cx, ids!(review_login))
                .set_visible(cx, scene == "login");
            self.ui
                .widget(cx, ids!(review_settings))
                .set_visible(cx, scene == "settings");
            let chats=scene.starts_with("chats");
            let contacts=scene=="contacts";
            let legacy=scene.starts_with("legacy-");
            self.ui.widget(cx,ids!(dock_review)).set_visible(cx,legacy);
            if legacy {
                let room=SelectedRoom::JoinedRoom {room_name_id:RoomNameId::empty("!retained:review.invalid".try_into().unwrap())};
                self.state.saved_dock_state_home=SavedDockState {
                    dock_items:std::collections::HashMap::from([
                        (id!(root),DockItem::Tabs {tabs:vec![id!(home_tab),room.tab_id(),id!(group_chats_welcome_tab)],selected:0,closable:true,hide_tab_bar:true}),
                        (id!(home_tab),DockItem::Tab {name:"Home".into(),kind:id!(welcome_screen),template:id!(PermanentTab)}),
                        (room.tab_id(),DockItem::Tab {name:"Retained chat".into(),kind:id!(room_screen),template:id!(CloseableTab)}),
                        (id!(group_chats_welcome_tab),DockItem::Tab {name:"Group Chats".into(),kind:id!(welcome_screen),template:id!(CloseableTab)}),
                    ]),
                    open_rooms:std::collections::HashMap::from([(room.tab_id(),room.clone())]),room_order:vec![room],selected_room:None,
                };
            }
            self.state.selected_tab=if contacts {hagency_app::home::navigation_tab_bar::SelectedTab::Contacts}else{hagency_app::home::navigation_tab_bar::SelectedTab::Home};
            let usage = scene == "usage";
            self.ui.widget(cx,ids!(panel)).set_visible(cx,!chats && !contacts && !legacy && !usage);
            self.ui.widget(cx,ids!(usage_review)).set_visible(cx,usage);
            if usage {
                for (id, identity, body, agent) in [
                    (id!(usage_measured), "measured", "**Here is the research summary.**\n\n✅ 本轮处理已结束 · 已运行 3 秒\n本次用量 50 Token（输入 30 / 输出 20；缓存输入 10，推理输出 5）· 工具请求 2 次，已返回 1 次", true),
                    (id!(usage_missing), "missing", "Hello!", true),
                    (id!(usage_human), "human", "Thanks for the update.", false),
                    (id!(usage_mobile), "mobile", "A mobile Agent reply.", true),
                ] {
                    let item = self.ui.widget(cx,&[id]);
                    hagency_app::home::room_screen::show_usage_design_preview(cx,&item,identity,body,agent);
                }
            }
            self.ui.widget(cx,ids!(chat_review)).set_visible(cx,chats);
            self.ui.widget(cx,ids!(contacts_review)).set_visible(cx,contacts);
            if chats {
                let wide=arg("width","1280").parse::<f64>().unwrap()>=900.0;
                self.ui.widget(cx,ids!(chat_welcome)).set_visible(cx,wide);
                self.ui.project_tree(cx,ids!(chat_list)).show_chats_design_preview(cx,scene.strip_prefix("chats-").unwrap_or("ready"));
            }
            if contacts {
                self.ui.mobile_hub(cx,ids!(contacts_review)).show_contacts_design_preview(cx);
            }
            self.ui
                .hagency_panel(cx, ids!(panel))
                .show_design_preview(cx, scene);
            *cx.global::<hagency_app::home::rooms_sidebar::DesktopWorkspace>() =
                if chats {hagency_app::home::rooms_sidebar::DesktopWorkspace::GroupChats}else{hagency_app::home::rooms_sidebar::DesktopWorkspace::Other};
            if chats || usage {cx.action(hagency_app::home::main_desktop_ui::MainDesktopUiAction::ActivateChats);}
            else if contacts {cx.action(hagency_app::home::navigation_tab_bar::NavigationBarAction::TabSelected(hagency_app::home::navigation_tab_bar::SelectedTab::Contacts));}
            else {cx.action(if scene.contains("agents") {
                hagency_app::hagency::ui::HagencyAction::OpenAgents
            } else {
                hagency_app::hagency::ui::HagencyAction::OpenProjects
            });}
        }
    }
    impl AppMain for Preview {
        fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
            hagency_app::app::register_standalone_widgets(vm);
            self::script_mod(vm)
        }
        fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
            if !self.loaded {
                self.loaded = true;
                if std::env::args().any(|a| a == "--zh") {
                    hagency_app::i18n::set_language(cx, hagency_app::i18n::Language::Chinese)
                        .unwrap();
                }
                if std::env::args().any(|a| a == "--dark") {
                    hagency_app::theme::select(
                        cx,
                        hagency_app::theme::Selection {
                            appearance: hagency_app::theme::Appearance::Dark,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
                let scale = arg("text-scale", "1").parse::<f64>().unwrap();
                if scale != 1.0 {
                    let mut package = hagency_app::theme::packages::export_current(cx).unwrap();
                    for dark in [false, true] {
                        package.set(
                            Some(dark),
                            "typography.scale",
                            octosense_theme_contract::number(scale),
                        );
                    }
                    let mut preferences = hagency_app::theme::packages::current(cx).unwrap();
                    preferences.package = Some(package);
                    hagency_app::theme::packages::apply(cx, preferences).unwrap();
                }
                self.scene(cx, &arg("scene", "projects"));
            }
            self.ui
                .handle_event(cx, event, &mut Scope::with_data(&mut self.state));
            if arg("scene","projects").starts_with("legacy-")
                && let Event::Actions(actions)=event
                && actions.iter().any(|a|matches!(a.downcast_ref(),Some(hagency_app::home::main_desktop_ui::MainDesktopUiAction::LoadDockFromAppState))) {
                self.ui.dock(cx,ids!(dock_review.dock)).item(id!(home_tab)).as_hagency_panel()
                    .show_design_preview(cx,&arg("scene","projects"));
            }
            if matches!(event, Event::LiveEdit | Event::ScriptReapply) {
                hagency_app::i18n::refresh_ui(cx, &self.ui);
            }
            if let Event::Custom(command) = event {
                if let Some(scene) = command.strip_prefix("scene:") {
                    self.scene(cx, scene);
                }
                if let Some(appearance) = command.strip_prefix("appearance:") {
                    hagency_app::theme::select(
                        cx,
                        hagency_app::theme::Selection {
                            appearance: if appearance == "dark" {
                                hagency_app::theme::Appearance::Dark
                            } else {
                                hagency_app::theme::Appearance::Light
                            },
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
            }
        }
    }
    pub fn run() {
        assert!(
            std::env::var_os("HAGENCY_APP_DATA_DIR").is_some(),
            "Set a disposable HAGENCY_APP_DATA_DIR"
        );
        app_main();
    }
}
#[cfg(debug_assertions)]
fn main() {
    preview::run();
}
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("This read-only review needs a debug build.");
}
