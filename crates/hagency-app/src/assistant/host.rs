//! The live [`Host`]: Rinx's UI and Matrix session, on the UI thread.
use super::{Assistant, Choice, Done, Exec, Host, Prompt, ReviewedApp, RoomRef, Sink, grants::Grants};
use crate::{
    app::AppStateAction,
    home::rooms_list::RoomsListRef,
    octoscript_apps::MiniAppsAction,
    room::BasicRoomDetails,
    utils::RoomNameId,
};
use makepad_widgets::*;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

/// Bumped on every account change, logout or login failure (any thread).
static GENERATION: LazyLock<Arc<AtomicU64>> = LazyLock::new(Default::default);

#[derive(Default)]
struct View {
    current_room: Option<RoomRef>,
    running_mini_app: Option<String>,
    reviewed_mini_app: Option<ReviewedApp>,
}

thread_local! {
    static ASSISTANT: RefCell<Option<Assistant>> = const { RefCell::new(None) };
    static VIEW: RefCell<View> = RefCell::new(View::default());
    /// Drafts waiting for their room's composer, by room id.
    static DRAFTS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// UI-thread actions of the assistant's sheet and account handling.
#[derive(Clone, Debug)]
pub enum AssistantAction {
    /// Show this prompt in the sheet (replacing any shown one).
    ShowPrompt(Prompt),
    /// Close the sheet.
    HidePrompt,
    /// The account changed somewhere; end the previous account's calls.
    AccountChanged,
    /// The persisted room grants changed.
    GrantsChanged,
}

fn with<R>(f: impl FnOnce(&mut Assistant) -> R) -> R {
    ASSISTANT.with(|a| {
        let mut a = a.borrow_mut();
        let a = a.get_or_insert_with(|| {
            let path = crate::app_data_dir().join("assistant").join("room_grants.json");
            Assistant::new(Grants::load(path), GENERATION.clone())
        });
        f(a)
    })
}

struct RinxHost<'a> {
    cx: &'a mut Cx,
}

impl Host for RinxHost<'_> {
    fn account(&self) -> Option<String> {
        if crate::logout::logout_state_machine::is_logout_in_progress() {
            return None;
        }
        crate::sliding_sync::current_user_id().map(|id| id.to_string())
    }

    fn joined_rooms(&mut self) -> Vec<RoomRef> {
        if !self.cx.has_global::<RoomsListRef>() {
            return Vec::new();
        }
        self.cx
            .get_global::<RoomsListRef>()
            .mini_app_share_rooms()
            .into_iter()
            .map(|room| RoomRef { id: room.room_id().to_string(), name: room.display().into_owned() })
            .collect()
    }

    fn current_room(&self) -> Option<RoomRef> {
        VIEW.with(|v| v.borrow().current_room.clone())
    }

    fn running_mini_app(&self) -> Option<String> {
        VIEW.with(|v| v.borrow().running_mini_app.clone())
    }

    fn reviewed_mini_app(&self) -> Option<ReviewedApp> {
        VIEW.with(|v| v.borrow().reviewed_mini_app.clone())
    }

    fn open_room(&mut self, room: &RoomRef) {
        let Ok(room_id) = matrix_sdk::ruma::OwnedRoomId::try_from(room.id.as_str()) else { return };
        let name = if self.cx.has_global::<RoomsListRef>() {
            self.cx.get_global::<RoomsListRef>().get_room_name(&room_id)
        } else {
            None
        };
        let details = BasicRoomDetails::Name(name.unwrap_or_else(|| RoomNameId::empty(room_id)));
        self.cx.action(AppStateAction::NavigateToRoom { room_to_close: None, destination_room: details });
    }

    fn draft(&mut self, room: &RoomRef, text: &str) {
        // The room's composer takes it when it next handles an event; it is
        // text in the composer, never a send.
        DRAFTS.with(|d| d.borrow_mut().insert(room.id.clone(), text.to_owned()));
        self.open_room(room);
        self.cx.redraw_all();
    }

    fn open_mini_app(&mut self, _app: &ReviewedApp) {
        self.cx.action(MiniAppsAction::OpenReviewed);
    }

    fn read(&mut self, account: &str, room: &RoomRef, limit: u32, done: Done) {
        run_matrix(account, room, "matrix.rooms_messages", serde_json::json!({"room_id": room.id, "limit": limit}), done);
    }

    fn send(&mut self, account: &str, room: &RoomRef, text: &str, done: Done) {
        run_matrix(account, room, "matrix.rooms_send", serde_json::json!({"room_id": room.id, "body": text}), done);
    }

    fn prompt_changed(&mut self, prompt: Option<&Prompt>) {
        match prompt {
            Some(prompt) => self.cx.action(AssistantAction::ShowPrompt(prompt.clone())),
            None => self.cx.action(AssistantAction::HidePrompt),
        }
    }
}

/// One Matrix service call through the mini-app adapters, under a lease for
/// the app `assistant`, this account and this one room.
fn run_matrix(account: &str, room: &RoomRef, service: &'static str, args: serde_json::Value, done: Done) {
    let lease = crate::octoscript_apps::assistant_lease(account, &room.id, service);
    crate::sliding_sync::spawn_async_task(async move {
        let result = crate::octoscript_apps::matrix_request(lease.clone(), service.to_owned(), args).await;
        lease.revoke();
        done(result);
    });
}

/// Takes the Rinx module's reply sink: its executor is live.
pub fn install(sink: Sink) {
    with(|a| a.set_sink(Some(sink)));
}

/// The module is closing: close the sheet, forget every call.
pub fn uninstall(cx: &mut Cx) {
    with(|a| a.shut_down(&mut RinxHost { cx }));
    DRAFTS.with(|d| d.borrow_mut().clear());
}

pub fn execute(cx: &mut Cx, call_id: &str, tool: &str, args: &str) -> Exec {
    with(|a| a.execute(&mut RinxHost { cx }, call_id, tool, args, Instant::now()))
}

pub fn cancel(cx: &mut Cx, call_id: &str) {
    with(|a| a.cancel(&mut RinxHost { cx }, call_id));
}

pub fn answer(cx: &mut Cx, choice: Choice) {
    with(|a| a.answer(&mut RinxHost { cx }, choice));
    cx.action(AssistantAction::GrantsChanged);
}

pub fn expire(cx: &mut Cx) {
    with(|a| a.expire(&mut RinxHost { cx }, Instant::now()));
}

/// Any thread: the account changed, logged out, or its session failed.
pub fn invalidate() {
    GENERATION.fetch_add(1, Ordering::AcqRel);
    Cx::post_action(AssistantAction::AccountChanged);
}

/// The UI side of [`invalidate`].
pub fn account_changed_on_ui(cx: &mut Cx) {
    with(|a| a.account_changed(&mut RinxHost { cx }));
    DRAFTS.with(|d| d.borrow_mut().clear());
}

/// The rooms the signed-in account lets the assistant read.
pub fn granted_rooms() -> Vec<String> {
    let Some(account) = crate::sliding_sync::current_user_id() else { return Vec::new() };
    with(|a| a.grants().rooms(account.as_str()))
}

pub fn revoke_room(cx: &mut Cx, room: &str) -> Result<(), String> {
    let account = crate::sliding_sync::current_user_id().ok_or("Not signed in")?;
    with(|a| a.grants_mut().revoke(account.as_str(), room))?;
    cx.action(AssistantAction::GrantsChanged);
    Ok(())
}

/// The room shown in Rinx now, if any.
pub fn set_current_room(room: Option<&RoomNameId>) {
    let room = room.map(|r| RoomRef { id: r.room_id().to_string(), name: r.display().into_owned() });
    VIEW.with(|v| v.borrow_mut().current_room = room);
}

/// The Mini apps screen's reviewed and running app.
pub fn set_mini_apps(reviewed: Option<ReviewedApp>, running: Option<String>) {
    VIEW.with(|v| {
        let mut v = v.borrow_mut();
        v.reviewed_mini_app = reviewed;
        v.running_mini_app = running;
    });
}

/// The draft the assistant left for this room's composer, once.
pub fn take_draft(room_id: &str) -> Option<String> {
    DRAFTS.with(|d| {
        let mut d = d.borrow_mut();
        if d.is_empty() { None } else { d.remove(room_id) }
    })
}
