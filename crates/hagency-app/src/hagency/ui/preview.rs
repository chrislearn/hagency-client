//! Read-only native layout fixtures. No identity, backend or execution is started.
use super::*;

impl HagencyPanelRef {
    pub fn show_design_preview(&self, cx: &mut Cx, scene: &str) {
        let scene=scene.strip_prefix("legacy-").unwrap_or(scene);
        let Some(mut panel) = self.borrow_mut() else {
            return;
        };
        panel.active = false;
        panel.busy = false;
        panel.display_blocked = false;
        panel.projects_loaded = true;
        panel.agents_loaded = true;
        panel.projects = vec![
            json!({"id":"design","name":"Product design","topic":"A shared workspace for research, interface design and the next release.","archived":false}),
            json!({"id":"native","name":"原生客户端 · 界面与体验","topic":"统一页面层级、组件状态与布局规范，让人与 Agent 的协作更清晰。","archived":false}),
            json!({"id":"research","name":"Research & engineering","topic":"Explore ideas, review findings and turn decisions into focused tasks.","archived":false}),
        ];
        panel.all_projects = panel.projects.clone();
        panel.agents = vec![
            json!({"id":"writer","displayName":"Research assistant","state":"active","visibility":"private","generation":1,"puppetMxid":"@research:example.test","executionDeviceId":"preview-device"}),
            json!({"id":"reviewer","displayName":"代码审阅助手","state":"active","visibility":"public","generation":1,"puppetMxid":"@reviewer:example.test","executionDeviceId":"preview-device"}),
        ];
        let projects = panel.projects.clone();
        let agents = panel.agents.clone();
        panel.labels(cx, ids!(projects), "Select a Project", &projects, "name");
        panel.labels(cx, ids!(agents), "Select my Agent", &agents, "displayName");
        panel
            .view
            .drop_down(cx, ids!(projects))
            .set_selected_item(cx, 0);
        panel
            .view
            .drop_down(cx, ids!(agents))
            .set_selected_item(cx, 0);
        for id in [
            ids!(project_detail),
            ids!(project_form),
            ids!(agent_detail),
            ids!(agent_create),
        ] {
            panel.view.view(cx, id).set_visible(cx, false);
        }
        panel.set_page(
            cx,
            if scene.starts_with("agents") {
                Page::Agents
            } else {
                Page::Projects
            },
        );
        if scene == "create" {
            panel
                .view
                .view(cx, ids!(project_form))
                .set_visible(cx, true);
            panel.begin_creation_form(cx, false);
            panel.creation_source(cx);
        }
        if scene == "overview" {
            panel
                .view
                .drop_down(cx, ids!(projects))
                .set_selected_item(cx, 1);
            panel
                .view
                .view(cx, ids!(project_detail))
                .set_visible(cx, true);
            panel.project_home =
                json!({"stats":{"tasks":24,"done":16,"overdue":2,"members":6},"archived":false});
            panel.board_catalog = vec![
                json!({"id":"main","name":"Main Board","isDefault":true}),
                json!({"id":"release","name":"Release planning","isDefault":false}),
            ];
            panel.project_section(cx, ids!(overview_section));
            panel.render_overview_stats(cx);
            panel
                .view
                .label(cx, ids!(heading))
                .set_text(cx, "Product design");
            panel
                .view
                .label(cx, ids!(overview_activity))
                .set_text(cx, "No recent activity.");
        }
        if scene.ends_with("-empty") || scene.ends_with("-error") {
            panel.projects.clear();panel.all_projects.clear();panel.agents.clear();
            panel.projects_loaded=!scene.ends_with("-error");
            panel.agents_loaded=!scene.ends_with("-error");
        }
        if scene=="agents-ledger" {panel.list_page(cx);panel.ledger_open=true;}
        if scene=="agents-pending" {
            panel.list_page(cx);panel.agent_recovery_open.set(true);
            panel.agent_pending_commands=vec![json!({"id":"preview-original-command","operation":"agent.create","state":"unknown","input":{"displayName":"Research assistant","idempotencyKey":"preview-original-command"}})];
        }
        panel.sync_navigation(cx);
        if scene=="projects-error" {panel.status(cx,project_empty_message(false,false));}
        panel.view.redraw(cx);
    }
}
