//! Interactive native acceptance window: real NativeOwner -> Server -> PostgreSQL.
//! Only Pasion/Matrix identity sources are fixtures. No in-memory board substitute.
//! Start Server's project_board_fixture, then set HAGENCY_BOARD_FIXTURE_INFO.
#[cfg(debug_assertions)]
mod interactive {
    use hagency::native_owner::{
        self, Command, MatrixAccessToken, MatrixTokenSource, NativeError, NativeOwner,
    };
    use hagency::project_board::{BoardAction, Create, Fields, STATES};
    use hagency_app::{hagency::board::ProjectBoardWidgetRefExt, makepad_widgets::*};
    use serde_json::Value;
    use std::{
        path::PathBuf,
        sync::{Arc, OnceLock},
    };

    struct FixtureTokens(&'static str);
    impl MatrixTokenSource for FixtureTokens {
        fn access_token(
            &self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>>
                    + Send
                    + '_,
            >,
        > {
            Box::pin(async {
                Ok(MatrixAccessToken {
                    access_token: format!("fixture-oauth-{}", self.0),
                    client_id: "fixture-native-client".into(),
                })
            })
        }
    }
    struct Live {
        owner: Arc<NativeOwner>,
        project: String,
        account: &'static str,
        _runtime: tokio::runtime::Runtime,
    }
    static LIVE: OnceLock<Live> = OnceLock::new();
    app_main!(InteractiveBoard);
    script_mod! {
        use mod.prelude.widgets.*
        use mod.widgets.*
        startup() do #(InteractiveBoard::script_component(vm)) {
            ui: Root {
                window := Window {window.inner_size:vec2(1440,900) window.title:"Hagency · Project board"
                    body +: {flow:Down padding:24 spacing:14 show_bg:true draw_bg.color:mod.widgets.RINX_PAGE
                        RinxLabel {text:"项目看板 · 本地交互验收" draw_text.text_style:theme.font_bold{font_size:22}}
                        context := RinxLabel {draw_text.color:mod.widgets.RINX_MUTED}
                        board := ProjectBoard {}
                    }
                }
            }
        }
    }
    #[derive(Script, ScriptHook)]
    pub struct InteractiveBoard {
        #[live]
        ui: WidgetRef,
        #[rust]
        loaded: bool,
    }
    impl AppMain for InteractiveBoard {
        fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
            // Packaged acceptance windows use staged assets instead of source-tree paths.
            if let Ok(path) = std::env::var("HAGENCY_BOARD_RESOURCE_DIR") {
                vm.cx_mut().package_root = Some(path);
            }
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
                let live = LIVE
                    .get()
                    .expect("fixture connection prepared before launch");
                self.ui.label(cx, ids!(context)).set_text(
                    cx,
                    &format!(
                        "{} · 真实本地 Server / PostgreSQL · 操作会保存到测试数据库",
                        live.account
                    ),
                );
                self.ui.project_board(cx, ids!(board)).connect_development(
                    cx,
                    live.owner.clone(),
                    &live.project,
                    live.account,
                );
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
            }
            self.ui.handle_event(cx, event, &mut Scope::empty());
        }
    }
    async fn prepare(
        data: &std::path::Path,
    ) -> Result<(Arc<NativeOwner>, String, &'static str), Box<dyn std::error::Error>> {
        let info: Value = serde_json::from_slice(&std::fs::read(std::env::var(
            "HAGENCY_BOARD_FIXTURE_INFO",
        )?)?)?;
        let origin = info["origin"].as_str().ok_or("fixture origin missing")?;
        let parsed: url::Url = origin.parse()?;
        if parsed.scheme() != "http"
            || parsed.host_str() != Some("127.0.0.1")
            || info["identitySource"]
                != "fixture Pasion/Matrix; real native authorization and board PostgreSQL/API"
        {
            return Err(
                "only the explicitly identified disposable loopback Server fixture is accepted"
                    .into(),
            );
        }
        let project = info["projectId"]
            .as_str()
            .ok_or("fixture Project missing")?
            .to_owned();
        let account = if std::env::args().any(|a| a == "--bob") {
            "bob"
        } else {
            "alice"
        };
        let owner = Arc::new(
            native_owner::open_with_matrix(
                &data.join(account),
                origin,
                &format!("@{account}:example.test"),
                Arc::new(FixtureTokens(account)),
            )
            .await?,
        );
        // Stable original intents seed exactly once. Reopening never overwrites edits.
        let titles = [
            "明确看板首页与任务详情的验收标准",
            "完善多成员权限与会话校验",
            "解决拖动与横向滚动的手势冲突",
            "检查长标题和中文排版",
            "验证断网后的原请求重试",
            "核对标签与负责人筛选",
            "完善键盘和菜单移动",
            "准备发布前的回归记录",
        ];
        if account == "alice" {
            for (lane, state) in STATES.iter().enumerate() {
                for (offset, title) in titles
                    .iter()
                    .enumerate()
                    .take(if lane == 0 { 8 } else { 2 })
                {
                    owner.execute(Command::ProjectBoard {project_id:project.clone(),action:BoardAction::Create {input:Create {
                    idempotency_key:format!("interactive-seed-v1-{lane}-{offset}"),
                    state:(*state).into(),
                    fields:Fields {
                        title:if lane==0 {(*title).into()} else {format!("{} · {}",["待办","实现交互反馈","解决布局问题","双成员验收","完成接口契约"][lane],offset+1)},
                        description:"目标：让团队成员可以直接完成创建、编辑、移动、评论与归档。\n\n验收：滚动正常；变更由 Server 确认；其他成员可以看到；冲突不覆盖草稿。".into(),
                        priority:if lane==2 {"urgent"} else if lane==1 {"high"} else {"normal"}.into(),
                        assignee_mxid:Some(if offset%2==0 {"@alice:example.test"} else {"@bob:example.test"}.into()),
                        labels:vec![if lane==0 {"design"} else {"P0"}.into()],
                        parent_id:None,
                        checklist:Vec::new(),
                        due_date:Some(if offset%2==0 {"2026-10-22"} else {"2026-11-01"}.into()),
                    },
                }}}).await?;
                }
            }
        }
        Ok((owner, project, account))
    }
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let data = std::env::var_os("HAGENCY_APP_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("hagency-kanban-interactive"));
        std::fs::create_dir_all(&data)?;
        // This development executable starts no threads before configuring its own directory.
        unsafe {
            std::env::set_var("HAGENCY_APP_DATA_DIR", &data);
        }
        std::fs::write(data.join("ui-language.json"), "\"zh-CN\"")?;
        let runtime = tokio::runtime::Runtime::new()?;
        let (owner, project, account) = runtime.block_on(prepare(&data))?;
        LIVE.set(Live {
            owner,
            project,
            account,
            _runtime: runtime,
        })
        .map_err(|_| "already initialized")?;
        app_main();
        Ok(())
    }
}
#[cfg(debug_assertions)]
fn main() {
    if let Err(error) = interactive::run() {
        eprintln!("Interactive board fixture unavailable: {error}");
        std::process::exit(1);
    }
}
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("This acceptance fixture requires a debug build.");
}
