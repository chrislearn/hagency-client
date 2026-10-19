//! What an assistant may do in Rinx, and the person's consent that bounds it
//! (ADR 0007).
//!
//! The tools are defined here once, independent of any transport. Today the
//! OctoSense shells reach them through Rinx's `ServiceExecutor`
//! (`crate::module`): the chat pane calls `rinx.<tool>` and the host runs it
//! on the UI thread. A later host route (Rinx's own peer, through the
//! app-peers broker) calls the same [`TOOLS`] through [`execute`]; the
//! system agent never holds these tools itself, it asks Rinx's peer.
//!
//! Consent stays in Rinx:
//! - `read_room` needs the person's grant for that room on the signed-in
//!   account. The first use shows Rinx's read sheet (once / always / deny);
//!   "always" persists per account ([`grants`]) and Settings → Privacy
//!   revokes it. No answer within [`PROMPT_TIMEOUT`] is a denial.
//! - `send_message` shows the room and the exact text in Rinx's send sheet and
//!   sends only on the person's yes. The tool is declared self-confirmed, so
//!   the chat pane does not ask a second time.
//! - `draft_message` fills the composer and never sends.
//!
//! Every call runs as the account that is signed in when it starts: Matrix
//! reads and sends go through the mini-app [`Lease`](crate::octoscript_apps::Lease)
//! (app `assistant`) and adapters, so an account switch or logout revokes
//! them. It also answers every waiting call `Unavailable` and turns a late
//! result into `Unavailable`, so nothing of the previous account reaches the
//! assistant.
pub mod grants;
mod host;
pub mod sheet;

pub use host::{
    account_changed_on_ui, answer, cancel, execute, expire, granted_rooms, install, invalidate, revoke_room,
    set_current_room, set_mini_apps, take_draft, uninstall, AssistantAction,
};

use grants::Grants;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// How long a sheet waits for the person; no answer is a denial. Shorter
/// than the chat pane's 60 s call deadline, so the assistant hears "denied"
/// rather than a timeout.
pub const PROMPT_TIMEOUT: Duration = Duration::from_secs(45);
/// Longest draft or message the assistant may write (bytes).
pub const MAX_TEXT_BYTES: usize = 4 * 1024;
/// Most messages `read_room` asks for; the reader serves at most
/// [`READ_CAP`] of them.
pub const MAX_READ_LIMIT: u32 = 50;
/// The mini-app reader's own cap (`matrix.rooms_messages`).
pub const READ_CAP: u32 = 30;
/// Bytes of structured data one result may carry (the wire's cap is 16 KiB).
const MAX_DATA_BYTES: usize = 15 * 1024;
const MAX_ARG_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    Read,
    Act,
    Destructive,
}

/// One tool as the assistant is told about it.
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    /// JSON schema of the argument object.
    pub parameters: &'static str,
    pub risk: Risk,
    /// Rinx's own sheet is the person's one confirmation.
    pub confirms_itself: bool,
}

pub const BRIEF: &str = "Rinx is the person's Matrix messenger. Call `status` first to see whether they are \
signed in and which room is open. Name a room by its exact name or its `!id` from `list_rooms`. Reading a \
room's messages needs the person's permission for that room, which Rinx asks for itself; a denial is final \
for this request. `draft_message` only fills the composer. `send_message` shows the person the exact text \
and sends only if they confirm in Rinx.";

pub const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "status",
        description: "Whether Rinx is signed in, which room is open and which mini app is running.",
        parameters: r#"{"type":"object","properties":{},"additionalProperties":false}"#,
        risk: Risk::Read,
        confirms_itself: false,
    },
    ToolSpec {
        name: "list_rooms",
        description: "The joined rooms: name and id only.",
        parameters: r#"{"type":"object","properties":{},"additionalProperties":false}"#,
        risk: Risk::Read,
        confirms_itself: false,
    },
    ToolSpec {
        name: "open_room",
        description: "Show a joined room in Rinx.",
        parameters: r#"{"type":"object","properties":{"room":{"type":"string","description":"Exact room name or !id"}},"required":["room"],"additionalProperties":false}"#,
        risk: Risk::Act,
        confirms_itself: false,
    },
    ToolSpec {
        name: "draft_message",
        description: "Open a room and put text in its message composer for the person to review and send. Never sends.",
        parameters: r#"{"type":"object","properties":{"room":{"type":"string"},"text":{"type":"string","maxLength":4096}},"required":["room","text"],"additionalProperties":false}"#,
        risk: Risk::Act,
        confirms_itself: false,
    },
    ToolSpec {
        name: "read_room",
        description: "The latest text messages of a joined room, oldest first (at most 30 are returned). The first read of a room asks the person for permission in Rinx.",
        parameters: r#"{"type":"object","properties":{"room":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":50}},"required":["room"],"additionalProperties":false}"#,
        risk: Risk::Read,
        confirms_itself: false,
    },
    ToolSpec {
        name: "open_mini_app",
        description: "Open the mini app the person has reviewed in Rinx's Mini apps screen. The person still grants its services by pressing Run.",
        parameters: r#"{"type":"object","properties":{"app":{"type":"string","description":"The reviewed app's id or name"},"room":{"type":"string","description":"The room it was reviewed for, if any"}},"required":["app"],"additionalProperties":false}"#,
        risk: Risk::Act,
        confirms_itself: false,
    },
    ToolSpec {
        name: "send_message",
        description: "Send a text message to a joined room. Rinx shows the person the room and the exact text and sends only if they confirm.",
        parameters: r#"{"type":"object","properties":{"room":{"type":"string"},"text":{"type":"string","maxLength":4096}},"required":["room","text"],"additionalProperties":false}"#,
        risk: Risk::Destructive,
        confirms_itself: true,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    Failed,
    Refused,
    Denied,
    Unavailable,
}

/// One call's answer: text for the model, JSON data beside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub call_id: String,
    pub outcome: Outcome,
    pub text: String,
    /// JSON text, or empty.
    pub data: String,
}

impl Reply {
    fn new(call_id: &str, outcome: Outcome, text: impl Into<String>) -> Self {
        Reply { call_id: call_id.into(), outcome, text: text.into(), data: String::new() }
    }
    fn with_data(mut self, data: Value) -> Self {
        self.data = data.to_string();
        if self.data.len() > MAX_DATA_BYTES {
            self.data.clear();
        }
        self
    }
}

pub enum Exec {
    Done(Reply),
    /// Answered later through the sink (a sheet is up, or Matrix is working).
    Pending,
}

/// Where later answers go. Called from any thread, at most once per call.
pub type Sink = Arc<dyn Fn(Reply) + Send + Sync>;
/// The end of one Matrix operation. Called from any thread, exactly once.
pub type Done = Box<dyn FnOnce(Result<Value, String>) + Send>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomRef {
    pub id: String,
    pub name: String,
}

/// The mini app the person reviewed in the Mini apps screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewedApp {
    pub id: String,
    pub name: String,
    /// The room its grant covers, if any.
    pub room: Option<String>,
}

/// What the tools need from Rinx. The live implementation drives the UI and
/// the Matrix SDK; the tests use a fake.
pub trait Host {
    /// The signed-in account, `None` when signed out or logging out.
    fn account(&self) -> Option<String>;
    fn joined_rooms(&mut self) -> Vec<RoomRef>;
    fn current_room(&self) -> Option<RoomRef>;
    fn running_mini_app(&self) -> Option<String>;
    fn reviewed_mini_app(&self) -> Option<ReviewedApp>;
    fn open_room(&mut self, room: &RoomRef);
    /// Open `room` and put `text` in its composer. Must not send.
    fn draft(&mut self, room: &RoomRef, text: &str);
    fn open_mini_app(&mut self, app: &ReviewedApp);
    /// Read the latest `limit` messages of `room` as `account`.
    fn read(&mut self, account: &str, room: &RoomRef, limit: u32, done: Done);
    /// Send `text` to `room` as `account`.
    fn send(&mut self, account: &str, room: &RoomRef, text: &str, done: Done);
    /// The sheet should show this prompt, or close.
    fn prompt_changed(&mut self, prompt: Option<&Prompt>);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptKind {
    /// May the assistant read this room? Once, always, or deny.
    ReadGrant { limit: u32 },
    /// Send exactly this text? Send or cancel.
    Send { text: String },
}

/// One question waiting for the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub call_id: String,
    pub account: String,
    pub room: RoomRef,
    pub kind: PromptKind,
    pub deadline: Instant,
}

/// The person's answer to the front prompt. On the send sheet `Once` sends
/// and `Always` is not offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Once,
    Always,
    Deny,
}

enum Call {
    Status,
    ListRooms,
    OpenRoom { room: String },
    Draft { room: String, text: String },
    Read { room: String, limit: u32 },
    OpenMiniApp { app: String, room: Option<String> },
    Send { room: String, text: String },
}

/// Calls whose Matrix work is running, by call id, with the account
/// generation they started in. Removing an entry is how a call is answered
/// exactly once: by its result, by an account switch, or never (cancelled).
type Inflight = Arc<Mutex<HashMap<String, u64>>>;

pub struct Assistant {
    sink: Option<Sink>,
    grants: Grants,
    prompts: VecDeque<Prompt>,
    inflight: Inflight,
    generation: Arc<AtomicU64>,
    /// The generation the waiting prompts and inflight calls belong to.
    seen_generation: u64,
    /// The account the last call ran as.
    account: Option<String>,
}

impl Assistant {
    pub fn new(grants: Grants, generation: Arc<AtomicU64>) -> Self {
        let seen_generation = generation.load(Ordering::Acquire);
        Assistant {
            sink: None,
            grants,
            prompts: VecDeque::new(),
            inflight: Arc::default(),
            generation,
            seen_generation,
            account: None,
        }
    }

    pub fn set_sink(&mut self, sink: Option<Sink>) {
        self.sink = sink;
    }

    pub fn grants(&self) -> &Grants {
        &self.grants
    }

    pub fn grants_mut(&mut self) -> &mut Grants {
        &mut self.grants
    }

    pub fn front_prompt(&self) -> Option<&Prompt> {
        self.prompts.front()
    }

    /// Run one call. Replies that come later go to the sink.
    pub fn execute(&mut self, host: &mut dyn Host, call_id: &str, tool: &str, args: &str, now: Instant) -> Exec {
        // The account first: work of a previous account never survives into this call.
        let account = host.account();
        self.sync_account(host, account.as_deref());
        let call = match parse(tool, args) {
            Ok(call) => call,
            Err(reason) => return Exec::Done(Reply::new(call_id, Outcome::Refused, reason)),
        };
        if let Call::Status = call {
            return Exec::Done(status(host, call_id, account.is_some()));
        }
        let Some(account) = account else {
            return Exec::Done(Reply::new(call_id, Outcome::Unavailable, "Rinx is not signed in to Matrix."));
        };
        match call {
            Call::Status => unreachable!(),
            Call::ListRooms => Exec::Done(list_rooms(host, call_id)),
            Call::OpenRoom { room } => Exec::Done(match resolve(host, &room) {
                Ok(room) => {
                    host.open_room(&room);
                    Reply::new(call_id, Outcome::Ok, format!("Opened {}.", room.name))
                        .with_data(json!({"room": {"id": room.id, "name": room.name}}))
                }
                Err(reason) => Reply::new(call_id, Outcome::Refused, reason),
            }),
            Call::Draft { room, text } => Exec::Done(match resolve(host, &room) {
                Ok(room) => {
                    host.draft(&room, &text);
                    Reply::new(
                        call_id,
                        Outcome::Ok,
                        format!("The draft is in {}'s composer. It was not sent; the person sends it.", room.name),
                    )
                    .with_data(json!({"room": {"id": room.id, "name": room.name}, "sent": false}))
                }
                Err(reason) => Reply::new(call_id, Outcome::Refused, reason),
            }),
            Call::Read { room, limit } => {
                let room = match resolve(host, &room) {
                    Ok(room) => room,
                    Err(reason) => return Exec::Done(Reply::new(call_id, Outcome::Refused, reason)),
                };
                if self.grants.allows(&account, &room.id) {
                    self.start_read(host, call_id, &account, &room, limit);
                } else {
                    self.ask(host, Prompt {
                        call_id: call_id.into(),
                        account,
                        room,
                        kind: PromptKind::ReadGrant { limit },
                        deadline: now + PROMPT_TIMEOUT,
                    });
                }
                Exec::Pending
            }
            Call::OpenMiniApp { app, room } => Exec::Done(open_mini_app(host, call_id, &app, room.as_deref())),
            Call::Send { room, text } => {
                let room = match resolve(host, &room) {
                    Ok(room) => room,
                    Err(reason) => return Exec::Done(Reply::new(call_id, Outcome::Refused, reason)),
                };
                self.ask(host, Prompt {
                    call_id: call_id.into(),
                    account,
                    room,
                    kind: PromptKind::Send { text },
                    deadline: now + PROMPT_TIMEOUT,
                });
                Exec::Pending
            }
        }
    }

    /// The person answered the front prompt.
    pub fn answer(&mut self, host: &mut dyn Host, choice: Choice) {
        let account = host.account();
        self.sync_account(host, account.as_deref());
        let Some(prompt) = self.prompts.pop_front() else { return };
        host.prompt_changed(self.prompts.front());
        if account.as_deref() != Some(prompt.account.as_str()) {
            self.reply(Reply::new(&prompt.call_id, Outcome::Unavailable, "The Rinx account changed."));
            return;
        }
        match (prompt.kind, choice) {
            (PromptKind::ReadGrant { .. }, Choice::Deny) => self.reply(Reply::new(
                &prompt.call_id,
                Outcome::Denied,
                format!("The person did not allow reading {}.", prompt.room.name),
            )),
            (PromptKind::Send { .. }, Choice::Deny) => self.reply(Reply::new(
                &prompt.call_id,
                Outcome::Denied,
                format!("The person did not send the message to {}.", prompt.room.name),
            )),
            (PromptKind::ReadGrant { limit }, Choice::Always) => {
                if let Err(e) = self.grants.grant(&prompt.account, &prompt.room.id) {
                    // Still read this once: the person said yes; only remembering failed.
                    eprintln!("assistant: could not save the room grant: {e}");
                }
                self.start_read(host, &prompt.call_id, &prompt.account, &prompt.room, limit);
            }
            (PromptKind::ReadGrant { limit }, Choice::Once) => {
                self.start_read(host, &prompt.call_id, &prompt.account, &prompt.room, limit);
            }
            (PromptKind::Send { text }, Choice::Once | Choice::Always) => {
                self.start_send(host, &prompt.call_id, &prompt.account, &prompt.room, &text);
            }
        }
    }

    /// Deny every prompt whose time ran out.
    pub fn expire(&mut self, host: &mut dyn Host, now: Instant) {
        let front = self.prompts.front().map(|p| p.call_id.clone());
        let (expired, waiting): (Vec<Prompt>, Vec<Prompt>) = self.prompts.drain(..).partition(|p| p.deadline <= now);
        self.prompts = waiting.into();
        for prompt in &expired {
            self.reply(Reply::new(
                &prompt.call_id,
                Outcome::Denied,
                format!("The person did not answer in Rinx in time; nothing was done in {}.", prompt.room.name),
            ));
        }
        if self.prompts.front().map(|p| &p.call_id) != front.as_ref() {
            host.prompt_changed(self.prompts.front());
        }
    }

    /// The caller gave up: close its sheet, drop its result. No reply.
    pub fn cancel(&mut self, host: &mut dyn Host, call_id: &str) {
        let was_front = self.prompts.front().is_some_and(|p| p.call_id == call_id);
        self.prompts.retain(|p| p.call_id != call_id);
        if was_front {
            host.prompt_changed(self.prompts.front());
        }
        self.inflight.lock().unwrap().remove(call_id);
    }

    /// The account changed or signed out (any thread bumped the generation):
    /// every waiting and running call ends `Unavailable`.
    pub fn account_changed(&mut self, host: &mut dyn Host) {
        let account = host.account();
        self.revoke_all(host);
        self.account = account;
    }

    /// The host is going away: close the sheet and forget every call.
    pub fn shut_down(&mut self, host: &mut dyn Host) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.seen_generation = self.generation.load(Ordering::Acquire);
        self.inflight.lock().unwrap().clear();
        if !self.prompts.is_empty() {
            self.prompts.clear();
            host.prompt_changed(None);
        }
        self.sink = None;
        self.account = None;
    }

    fn sync_account(&mut self, host: &mut dyn Host, account: Option<&str>) {
        let generation = self.generation.load(Ordering::Acquire);
        if generation != self.seen_generation || (self.account.is_some() && self.account.as_deref() != account) {
            if generation == self.seen_generation {
                // Noticed here first (no invalidation reached us): make it one.
                self.generation.fetch_add(1, Ordering::AcqRel);
            }
            self.revoke_all(host);
        }
        self.account = account.map(str::to_owned);
    }

    fn revoke_all(&mut self, host: &mut dyn Host) {
        let current = self.generation.load(Ordering::Acquire);
        self.seen_generation = current;
        let prompts: Vec<Prompt> = self.prompts.drain(..).collect();
        if !prompts.is_empty() {
            host.prompt_changed(None);
        }
        let stale: Vec<String> = {
            let mut inflight = self.inflight.lock().unwrap();
            let stale: Vec<String> = inflight.iter().filter(|(_, g)| **g != current).map(|(id, _)| id.clone()).collect();
            for id in &stale {
                inflight.remove(id);
            }
            stale
        };
        for id in prompts.iter().map(|p| p.call_id.clone()).chain(stale) {
            self.reply(Reply::new(&id, Outcome::Unavailable, "The Rinx account changed or signed out."));
        }
    }

    fn ask(&mut self, host: &mut dyn Host, prompt: Prompt) {
        self.prompts.push_back(prompt);
        if self.prompts.len() == 1 {
            host.prompt_changed(self.prompts.front());
        }
    }

    fn reply(&self, reply: Reply) {
        if let Some(sink) = &self.sink {
            sink(reply);
        }
    }

    /// A completion that answers once, in the generation it started in.
    fn finisher(&self, call_id: &str, ok: impl FnOnce(Value) -> Reply + Send + 'static) -> Done {
        let generation = self.generation.load(Ordering::Acquire);
        self.inflight.lock().unwrap().insert(call_id.to_owned(), generation);
        let inflight = self.inflight.clone();
        let current = self.generation.clone();
        let sink = self.sink.clone();
        let call_id = call_id.to_owned();
        Box::new(move |result| {
            let Some(started) = inflight.lock().unwrap().remove(&call_id) else { return };
            let reply = if started != current.load(Ordering::Acquire) {
                // Late: the account changed while Matrix worked. Its data never leaves.
                Reply::new(&call_id, Outcome::Unavailable, "The Rinx account changed or signed out.")
            } else {
                match result {
                    Ok(value) => ok(value),
                    Err(e) => Reply::new(&call_id, Outcome::Failed, e),
                }
            };
            if let Some(sink) = sink {
                sink(reply);
            }
        })
    }

    fn start_read(&mut self, host: &mut dyn Host, call_id: &str, account: &str, room: &RoomRef, limit: u32) {
        let (id, name) = (call_id.to_owned(), room.name.clone());
        let room_json = json!({"id": room.id, "name": room.name});
        let done = self.finisher(call_id, move |value| read_reply(&id, &name, room_json, value));
        host.read(account, room, limit.min(READ_CAP), done);
    }

    fn start_send(&mut self, host: &mut dyn Host, call_id: &str, account: &str, room: &RoomRef, text: &str) {
        let (id, name) = (call_id.to_owned(), room.name.clone());
        let room_json = json!({"id": room.id, "name": room.name});
        let done = self.finisher(call_id, move |_| {
            Reply::new(&id, Outcome::Ok, format!("The person confirmed; sent to {name}."))
                .with_data(json!({"room": room_json, "sent": true}))
        });
        host.send(account, room, text, done);
    }
}

fn status(host: &mut dyn Host, call_id: &str, signed_in: bool) -> Reply {
    let room = if signed_in { host.current_room() } else { None };
    let app = if signed_in { host.running_mini_app() } else { None };
    let text = if !signed_in {
        "Rinx is not signed in to Matrix.".to_owned()
    } else {
        format!(
            "Signed in. {}. {}.",
            room.as_ref().map_or("No room is open".to_owned(), |r| format!("The open room is {}", r.name)),
            app.as_ref().map_or("No mini app is running".to_owned(), |a| format!("The mini app {a} is running")),
        )
    };
    Reply::new(call_id, Outcome::Ok, text).with_data(json!({
        "signed_in": signed_in,
        "current_room": room.map(|r| json!({"id": r.id, "name": r.name})),
        "open_mini_app": app,
    }))
}

fn list_rooms(host: &mut dyn Host, call_id: &str) -> Reply {
    let rooms = host.joined_rooms();
    let total = rooms.len();
    let mut listed = Vec::new();
    let mut bytes = 64;
    for room in &rooms {
        let entry = json!({"id": room.id, "name": room.name});
        bytes += entry.to_string().len() + 1;
        if bytes > MAX_DATA_BYTES {
            break;
        }
        listed.push(entry);
    }
    let truncated = listed.len() < total;
    let mut text = format!("{total} joined rooms");
    if truncated {
        text.push_str(&format!(" (the first {} are listed)", listed.len()));
    }
    text.push('.');
    Reply::new(call_id, Outcome::Ok, text).with_data(json!({"rooms": listed, "total": total, "truncated": truncated}))
}

fn open_mini_app(host: &mut dyn Host, call_id: &str, app: &str, room: Option<&str>) -> Reply {
    let Some(reviewed) = host.reviewed_mini_app() else {
        return Reply::new(
            call_id,
            Outcome::Refused,
            "No mini app is reviewed. The person imports and reviews a bundle in Rinx's Mini apps screen first.",
        );
    };
    let named = |s: &str| s.eq_ignore_ascii_case(&reviewed.id) || s.eq_ignore_ascii_case(&reviewed.name);
    if !named(app.trim()) {
        return Reply::new(
            call_id,
            Outcome::Refused,
            format!("'{app}' is not the reviewed mini app; the reviewed one is {} ({}).", reviewed.name, reviewed.id),
        );
    }
    if let Some(room) = room {
        let wanted = match resolve(host, room) {
            Ok(room) => room,
            Err(reason) => return Reply::new(call_id, Outcome::Refused, reason),
        };
        if reviewed.room.as_deref() != Some(wanted.id.as_str()) {
            return Reply::new(
                call_id,
                Outcome::Refused,
                format!(
                    "{} was reviewed for {}; the person must review it again for {}.",
                    reviewed.name,
                    reviewed.room.as_deref().unwrap_or("no room"),
                    wanted.name
                ),
            );
        }
    }
    host.open_mini_app(&reviewed);
    Reply::new(
        call_id,
        Outcome::Ok,
        format!("Opened {} in Rinx. The person grants its services by pressing Run.", reviewed.name),
    )
    .with_data(json!({"app": reviewed.id, "room": reviewed.room, "running": false}))
}

/// The reader's answer, bounded: the oldest messages go first when it is too big.
fn read_reply(call_id: &str, name: &str, room: Value, value: Value) -> Reply {
    let mut messages = match value {
        Value::Array(items) => items,
        Value::Object(mut map) => match map.remove("messages") {
            Some(Value::Array(items)) => items,
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    let returned = messages.len();
    let data = |messages: &Vec<Value>| json!({"room": room, "messages": messages, "omitted": returned - messages.len()});
    while !messages.is_empty() && data(&messages).to_string().len() > MAX_DATA_BYTES {
        messages.remove(0);
    }
    let mut text = format!("{} messages from {name}, oldest first", messages.len());
    if messages.len() < returned {
        text.push_str(&format!(" ({} older ones did not fit)", returned - messages.len()));
    }
    text.push('.');
    Reply::new(call_id, Outcome::Ok, text).with_data(data(&messages))
}

/// A joined room by `!id` or exact (case-insensitive) name.
fn resolve(host: &mut dyn Host, query: &str) -> Result<RoomRef, String> {
    let query = query.trim();
    let rooms = host.joined_rooms();
    if query.starts_with('!') {
        return rooms
            .into_iter()
            .find(|r| r.id == query)
            .ok_or_else(|| format!("{query} is not a room this account has joined."));
    }
    let matches: Vec<RoomRef> = rooms.into_iter().filter(|r| r.name.eq_ignore_ascii_case(query)).collect();
    match matches.len() {
        0 => Err(format!("No joined room is named '{query}'. Use list_rooms for the exact names.")),
        1 => Ok(matches.into_iter().next().unwrap()),
        n => Err(format!(
            "{n} rooms are named '{query}'; name one by its id: {}.",
            matches.iter().take(5).map(|r| r.id.as_str()).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// Strict arguments: an object with only the tool's keys, each of its type.
fn parse(tool: &str, args: &str) -> Result<Call, String> {
    let Some(spec) = TOOLS.iter().find(|t| t.name == tool) else {
        return Err(format!(
            "Rinx has no tool '{tool}'; its tools are {}.",
            TOOLS.iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
        ));
    };
    let fields = match serde_json::from_str::<Value>(if args.trim().is_empty() { "{}" } else { args }) {
        Ok(Value::Object(fields)) => fields,
        _ => return Err(format!("{} takes a JSON object of arguments.", spec.name)),
    };
    let allowed: &[&str] = match tool {
        "open_room" => &["room"],
        "draft_message" | "send_message" => &["room", "text"],
        "read_room" => &["room", "limit"],
        "open_mini_app" => &["app", "room"],
        _ => &[],
    };
    if let Some(key) = fields.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(format!("{tool} does not take '{key}'."));
    }
    let string = |key: &str, max: usize| -> Result<Option<String>, String> {
        match fields.get(key) {
            None => Ok(None),
            Some(Value::String(s)) if s.trim().is_empty() => Err(format!("{tool}: '{key}' is empty.")),
            Some(Value::String(s)) if s.len() > max => Err(format!("{tool}: '{key}' is longer than {max} bytes.")),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(format!("{tool}: '{key}' must be a string.")),
        }
    };
    let required = |key: &str, max: usize| string(key, max)?.ok_or_else(|| format!("{tool} needs '{key}'."));
    Ok(match tool {
        "status" => Call::Status,
        "list_rooms" => Call::ListRooms,
        "open_room" => Call::OpenRoom { room: required("room", MAX_ARG_BYTES)? },
        "draft_message" => Call::Draft { room: required("room", MAX_ARG_BYTES)?, text: required("text", MAX_TEXT_BYTES)? },
        "send_message" => Call::Send { room: required("room", MAX_ARG_BYTES)?, text: required("text", MAX_TEXT_BYTES)? },
        "read_room" => {
            let limit = match fields.get("limit") {
                None => 20,
                Some(v) => match v.as_u64() {
                    Some(n) if (1..=MAX_READ_LIMIT as u64).contains(&n) => n as u32,
                    _ => return Err(format!("read_room: 'limit' must be an integer from 1 to {MAX_READ_LIMIT}.")),
                },
            };
            Call::Read { room: required("room", MAX_ARG_BYTES)?, limit }
        }
        "open_mini_app" => Call::OpenMiniApp { app: required("app", MAX_ARG_BYTES)?, room: string("room", MAX_ARG_BYTES)? },
        _ => unreachable!("every tool in TOOLS has a parser"),
    })
}

#[cfg(test)]
mod tests;
