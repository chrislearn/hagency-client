//! Read-only native design fixture, isolated from accounts and execution.
//! HAGENCY_APP_DATA_DIR must point to a disposable folder when running this.
//! cargo run -p hagency-app --example project_board_preview -- [--detail] [--narrow] [--dark]
#[cfg(debug_assertions)]
mod preview {
    use hagency_app::{makepad_widgets::*, hagency::board::ProjectBoardWidgetRefExt};
    use hagency::project_board::{Snapshot, STATES, CAPABILITY};
    use serde_json::json;
    app_main!(Preview);
    script_mod! {
        use mod.prelude.widgets.*
        use mod.widgets.*
        startup() do #(Preview::script_component(vm)){
            ui: Root {
                window := Window {window.inner_size: vec2(1360,860)
                    body +: {flow: Down padding: 24 spacing: 16 show_bg: true draw_bg.color: mod.widgets.RINX_PAGE
                        RinxLabel {text: "Project board · native design preview" draw_text.text_style: theme.font_bold{font_size: 22}}
                        board := ProjectBoard {}
                    }
                }
            }
        }
    }
    #[derive(Script, ScriptHook)]
    pub struct Preview {
        #[live]
        ui: WidgetRef,
        #[rust]
        loaded: bool,
    }
    impl AppMain for Preview {
        fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
            makepad_widgets::theme_mod(vm);
            script_eval!(vm,{mod.theme = mod.themes.light});
            hagency_app::theme::init_standalone(vm);
            makepad_widgets::widgets_mod(vm);
            makepad_widgets::desktop_style::apply_widgets(vm);
            hagency_app::app::register_widgets(vm);
            self::script_mod(vm)
        }
        fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
            if !self.loaded {
                self.loaded = true;
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
                if std::env::args().any(|a| a == "--narrow") {
                    let mut window = self.ui.window(cx, ids!(window));
                    script_apply_eval!(cx,window,{window.inner_size:vec2(660,820)});
                }
                let titles = [
                    "Define the shared board experience",
                    "Review task permissions and membership",
                    "解决拖拽与滚动手势冲突",
                    "Verify the two-member workflow",
                    "Ship the native board",
                ];
                let lanes:Vec<_>=STATES.iter().enumerate().map(|(index,state)|{
                    let count=if index==0 {3}else if index==1{2}else{1};
                    let tasks:Vec<_>=(0..count).map(|offset|json!({"id":format!("task_{index}_{offset}"),"projectId":"preview","number":index*3+offset+1,"title":if offset==0{titles[index]}else if offset==1{"Long titles wrap naturally and keep the card readable across different window sizes"}else{"Explore keyboard and menu alternatives"},"description":"A shared workspace for people and Agents.\n\nKeep each task focused, show clear ownership, and preserve drafts when another member edits the same task.\n\n验收：布局清晰，中文正常显示；变更后可以恢复草稿。","state":state,"rank":(offset+1)*1024,"priority":if index==2{"urgent"}else if index==1{"high"}else{"normal"},"assigneeMxid":if offset==2{None}else{Some("@alice:example.test")},"labels":if index==0{vec!["design","desktop"]}else{vec!["P0"]},"dueDate":"2026-11-01","version":1,"createdBy":"@alice:example.test","createdAt":1,"updatedAt":1,"archivedAt":null})).collect();
                    json!({"state":state,"total":tasks.len(),"tasks":tasks,"nextCursor":null,"sequence":8})
                }).collect();
                let snapshot:Snapshot=serde_json::from_value(json!({"projectId":"preview","sequence":8,"capability":CAPABILITY,"members":["@alice:example.test","@bob:example.test"],"lanes":lanes})).unwrap();
                let selected = if std::env::args().any(|a| a == "--detail") {
                    Some("task_0_0")
                } else {
                    None
                };
                self.ui
                    .project_board(cx, ids!(board))
                    .show_design_preview(cx, snapshot, selected);
            }
            self.ui.handle_event(cx, event, &mut Scope::empty());
        }
    }
    pub fn run() {
        if std::env::var_os("HAGENCY_APP_DATA_DIR").is_none() {
            // Before Makepad or any worker threads start; only this dev fixture.
            unsafe {
                std::env::set_var(
                    "HAGENCY_APP_DATA_DIR",
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../target/kanban-preview"),
                );
            }
        }
        app_main();
    }
}
#[cfg(debug_assertions)]
fn main() {
    preview::run();
}
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("Use a debug build for the read-only design preview.");
}
