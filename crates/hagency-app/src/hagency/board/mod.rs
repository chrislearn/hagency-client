//! A Project-scoped shared board. Every async reply is pinned to account, Project
//! and view generation; writes retain their original intent until acknowledged.
use makepad_widgets::*;
use ::hagency::native_owner::Command;
use ::hagency::project_board::*;
use super::backend::Service;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

mod connection;
use connection::BoardConnection;
mod view;
mod controller;
mod interaction;
pub use view::script_mod;

#[derive(Clone, Debug, Default)]
enum CardAction {
    Open,
    Start(Vec2d),
    Drag(Vec2d),
    Drop(Vec2d),
    #[default]
    None,
}
#[derive(Script, ScriptHook, Widget)]
pub struct KanbanCard {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    #[rust]
    dragging: bool,
    #[rust]
    focused: bool,
}
impl Widget for KanbanCard {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let uid = self.widget_uid();
        // Mouse users can grab the whole card. Touch keeps ordinary list scrolling;
        // the explicit handle remains the touch drag target.
        let handle = self.view.view(cx, ids!(drag_handle)).area();
        let area = if matches!(event, Event::TouchUpdate(_)) {
            handle
        } else {
            self.view.area()
        };
        match event.hits(cx, area) {
            Hit::FingerDown(e) => {
                self.dragging = true;
                cx.set_key_focus(self.view.area());
                cx.widget_action(uid, CardAction::Start(e.abs));
            }
            Hit::FingerMove(e) if self.dragging => cx.widget_action(uid, CardAction::Drag(e.abs)),
            Hit::FingerUp(e) if self.dragging => {
                self.dragging = false;
                cx.widget_action(uid, CardAction::Drop(e.abs));
                if e.was_tap() {
                    cx.widget_action(uid, CardAction::Open);
                }
            }
            Hit::KeyDown(key)
                if !key.is_repeat
                    && matches!(key.key_code, KeyCode::ReturnKey | KeyCode::Space) =>
            {
                cx.widget_action(uid, CardAction::Open)
            }
            Hit::KeyFocus(_) => {
                self.focused = true;
                self.view.redraw(cx);
            }
            Hit::KeyFocusLost(_) => {
                self.focused = false;
                self.view.redraw(cx);
            }
            Hit::FingerHoverIn(_) => cx.set_cursor(MouseCursor::Hand),
            _ => (),
        }
        self.view.handle_event(cx, event, scope);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let border_size = if self.focused { 2.0 } else { 1.0 };
        script_apply_eval!(cx,self.view,{draw_bg.border_size:#(border_size)});
        if self.focused {
            script_apply_eval!(cx,self.view,{draw_bg.border_color:mod.widgets.RINX_ACCENT});
        }
        let result = self.view.draw_walk(cx, scope, walk);
        if result.is_done() {
            cx.add_nav_stop(self.view.area(), NavRole::TextInput, Inset::default());
        }
        result
    }
}
#[derive(Clone, Debug)]
enum Read {
    Snapshot,
    Changes,
    Page(usize),
    Detail(String),
    Comments(String, bool),
    Activity(String, bool),
}
#[derive(Debug)]
struct Completed {
    identity: String,
    project: String,
    epoch: u64,
    request: u64,
    kind: Read,
    detail_generation: u64,
    result: Result<Value, String>,
}
#[derive(Debug)]
struct Saved {
    identity: String,
    project: String,
    epoch: u64,
    request: u64,
    result: Result<Value, String>,
}
#[derive(Clone)]
struct Drag {
    task: BoardTask,
    start: Vec2d,
    at: Vec2d,
    moved: bool,
}
#[derive(Script, ScriptHook, Widget)]
pub struct ProjectBoard {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    #[rust]
    connection_override: Option<BoardConnection>,
    #[rust]
    identity: String,
    #[rust]
    project: String,
    #[rust]
    active: bool,
    #[rust]
    epoch: u64,
    #[rust]
    snapshot: Option<Snapshot>,
    #[rust]
    query: Query,
    #[rust]
    members: Vec<String>,
    #[rust]
    reads: HashSet<u64>,
    #[rust]
    loading: bool,
    #[rust]
    detail_request: u64,
    #[rust]
    detail_loaded: bool,
    #[rust]
    pending: Option<BoardAction>,
    #[rust]
    saving: bool,
    #[rust]
    save_request: u64,
    #[rust]
    retained: HashMap<String, BoardAction>,
    #[rust]
    timer: Timer,
    #[rust]
    selected: Option<BoardTask>,
    #[rust]
    creating: bool,
    #[rust]
    dirty: bool,
    #[rust]
    conflict: bool,
    #[rust]
    comments: Vec<Value>,
    #[rust]
    comments_after: i64,
    #[rust]
    comments_request: u64,
    #[rust]
    activity: Vec<Value>,
    #[rust]
    activity_after: i64,
    #[rust]
    activity_request: u64,
    #[rust]
    card_areas: HashMap<String, Area>,
    #[rust]
    card_targets: HashMap<WidgetUid, BoardTask>,
    #[rust]
    more_targets: HashMap<WidgetUid, usize>,
    #[rust]
    drag: Option<Drag>,
    #[rust]
    drag_frame: NextFrame,
    #[rust]
    scroll_x: f64,
    #[rust(true)]
    foreground: bool,
    #[rust]
    retry_delay: f64,
    #[rust]
    offline: bool,
    #[rust]
    failed: bool,
    #[rust]
    discard_armed: bool,
}
fn nonce() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
fn tr(key: &str) -> &str {
    crate::i18n::tr(key)
}
fn state_label(state: &str) -> &str {
    tr(match state {
        "todo" => "To do",
        "in_progress" => "In progress",
        "blocked" => "Blocked",
        "in_review" => "In review",
        "done" => "Done",
        _ => "Unknown",
    })
}
fn priority_label(priority: &str) -> &str {
    tr(match priority {
        "low" => "Low",
        "normal" => "Normal",
        "high" => "High",
        "urgent" => "Urgent",
        _ => "Normal",
    })
}
fn member_label(mxid: &str) -> &str {
    mxid.strip_prefix('@')
        .and_then(|m| m.split_once(':').map(|(name, _)| name))
        .unwrap_or(mxid)
}
fn timestamp(value: &Value) -> String {
    value
        .as_i64()
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .map(|at| {
            at.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M %:z")
                .to_string()
        })
        .unwrap_or_default()
}
fn error_message(code: &str) -> &str {
    tr(match code {
        "board_unsupported" => "This server does not support project boards yet.",
        "board_version_conflict" | "board_snapshot_changed" | "board_neighbours_changed" => {
            "The board changed on another device. Refreshed the board; your draft is preserved."
        }
        "project_membership_required"
        | "authorization_expired"
        | "owner_authorization_required"
        | "sign_in_required"
        | "matrix_account_changed"
        | "device_authorization_required"
        | "device_authorization_expired"
        | "invalid_device_authorization" => {
            "Project access has expired. Sign in again or check your membership."
        }
        "invalid_board_fields"
        | "invalid_board_date"
        | "invalid_board_labels"
        | "invalid_board_payload" => "Check the title, labels and date before saving.",
        "board_assignee_not_member" => {
            "The assignee is no longer a Project member. Choose another member."
        }
        "board_human_assignee_required" => {
            "Choose a person. Agent execution will be available separately."
        }
        _ => "The server could not confirm this request. Check your connection and retry.",
    })
}
fn denied(code: &str) -> bool {
    matches!(
        code,
        "project_membership_required"
            | "authorization_expired"
            | "owner_authorization_required"
            | "sign_in_required"
            | "matrix_account_changed"
            | "matrix_authorization_required"
    )
}
fn uncertain(code: &str) -> bool {
    !matches!(
        code,
        "board_unsupported"
            | "invalid_board_fields"
            | "invalid_board_date"
            | "invalid_board_labels"
            | "invalid_board_payload"
            | "invalid_board_comment"
            | "invalid_board_neighbours"
            | "invalid_board_version"
            | "invalid_board_state"
            | "invalid_identifier"
            | "board_assignee_not_member"
            | "board_human_assignee_required"
            | "board_version_conflict"
            | "board_snapshot_changed"
            | "board_neighbours_changed"
            | "board_idempotency_conflict"
            | "board_archive_conflict"
            | "board_capacity_exceeded"
            | "board_task_not_found"
    )
}
fn column_path(index: usize) -> &'static [LiveId] {
    match index {
        0 => ids!(todo),
        1 => ids!(in_progress),
        2 => ids!(blocked),
        3 => ids!(in_review),
        _ => ids!(done),
    }
}
impl ProjectBoardRef {
    pub fn bind(&self, cx: &mut Cx, identity: &str, project: Option<&str>, active: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.bind(cx, identity, project, active);
        }
    }
}
#[cfg(test)]
mod tests;

/// Development-only read-only rendering. It cannot authenticate or submit work.
#[cfg(debug_assertions)]
impl ProjectBoardRef {
    pub fn show_design_preview(&self, cx: &mut Cx, snapshot: Snapshot, selected: Option<&str>) {
        if let Some(mut board) = self.borrow_mut() {
            board.active = false;
            board.project = snapshot.project_id.clone();
            let members = snapshot.members.clone();
            let task = selected
                .and_then(|id| {
                    snapshot
                        .lanes
                        .iter()
                        .flat_map(|l| &l.tasks)
                        .find(|t| t.id == id)
                })
                .cloned();
            board.snapshot = Some(snapshot);
            board.sync_members(cx, members);
            if let Some(task) = task {
                board.selected = Some(task.clone());
                board.detail_loaded = true;
                board.fill_editor(cx, task.fields());
                board.view.view(cx, ids!(inspector)).set_visible(cx, true);
                board
                    .view
                    .label(cx, ids!(detail_heading))
                    .set_text(cx, &format!("#{}", task.number));
                board
                    .view
                    .label(cx, ids!(detail_notice))
                    .set_text(cx, tr("Visible to current Project members."));
                board
                    .view
                    .label(cx, ids!(comments))
                    .set_text(cx, tr("No comments yet"));
                board
                    .view
                    .label(cx, ids!(activity))
                    .set_text(cx, tr("Created task"));
            }
            board.sync_controls(cx);
        }
    }
}
