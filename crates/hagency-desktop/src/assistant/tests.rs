use super::*;

/// A Rinx with two rooms that records what the tools did.
struct Fake {
    account: Option<String>,
    rooms: Vec<RoomRef>,
    opened: Vec<String>,
    drafts: Vec<(String, String)>,
    reads: Vec<(String, String, u32, Done)>,
    sends: Vec<(String, String, String, Done)>,
    prompt: Option<Prompt>,
    reviewed: Option<ReviewedApp>,
    opened_apps: Vec<String>,
}

impl Fake {
    fn new() -> Self {
        Fake {
            account: Some("@alice:x".into()),
            rooms: vec![
                RoomRef { id: "!team:x".into(), name: "Team".into() },
                RoomRef { id: "!home:x".into(), name: "Home".into() },
            ],
            opened: vec![],
            drafts: vec![],
            reads: vec![],
            sends: vec![],
            prompt: None,
            reviewed: None,
            opened_apps: vec![],
        }
    }
}

impl Host for Fake {
    fn account(&self) -> Option<String> {
        self.account.clone()
    }
    fn joined_rooms(&mut self) -> Vec<RoomRef> {
        self.rooms.clone()
    }
    fn current_room(&self) -> Option<RoomRef> {
        self.opened.last().and_then(|id| self.rooms.iter().find(|r| &r.id == id).cloned())
    }
    fn running_mini_app(&self) -> Option<String> {
        None
    }
    fn reviewed_mini_app(&self) -> Option<ReviewedApp> {
        self.reviewed.clone()
    }
    fn open_room(&mut self, room: &RoomRef) {
        self.opened.push(room.id.clone());
    }
    fn draft(&mut self, room: &RoomRef, text: &str) {
        self.opened.push(room.id.clone());
        self.drafts.push((room.id.clone(), text.into()));
    }
    fn open_mini_app(&mut self, app: &ReviewedApp) {
        self.opened_apps.push(app.id.clone());
    }
    fn read(&mut self, account: &str, room: &RoomRef, limit: u32, done: Done) {
        self.reads.push((account.into(), room.id.clone(), limit, done));
    }
    fn send(&mut self, account: &str, room: &RoomRef, text: &str, done: Done) {
        self.sends.push((account.into(), room.id.clone(), text.into(), done));
    }
    fn prompt_changed(&mut self, prompt: Option<&Prompt>) {
        self.prompt = prompt.cloned();
    }
}

type Replies = Arc<Mutex<Vec<Reply>>>;

fn assistant() -> (Assistant, Replies, Arc<AtomicU64>) {
    let generation = Arc::new(AtomicU64::new(0));
    let mut a = Assistant::new(Grants::in_memory(), generation.clone());
    let replies: Replies = Arc::default();
    let sink = replies.clone();
    a.set_sink(Some(Arc::new(move |r| sink.lock().unwrap().push(r))));
    (a, replies, generation)
}

fn done(exec: Exec) -> Reply {
    match exec {
        Exec::Done(reply) => reply,
        Exec::Pending => panic!("expected an immediate answer"),
    }
}

fn pending(exec: Exec) {
    assert!(matches!(exec, Exec::Pending), "expected Pending");
}

fn take(replies: &Replies) -> Vec<Reply> {
    std::mem::take(&mut *replies.lock().unwrap())
}

fn messages() -> Value {
    json!({"messages": [{"sender": "Bob", "body": "hi"}], "unread_count": 0})
}

#[test]
fn the_manifest_is_the_first_action_set() {
    let names: Vec<_> = TOOLS.iter().map(|t| t.name).collect();
    assert_eq!(names, ["status", "list_rooms", "open_room", "draft_message", "read_room", "open_mini_app", "send_message"]);
    for tool in TOOLS {
        let schema: Value = serde_json::from_str(tool.parameters).expect("schema parses");
        assert_eq!(schema["type"], "object");
        // Only send confirms itself, and it is the only destructive tool.
        assert_eq!(tool.confirms_itself, tool.name == "send_message");
        assert_eq!(tool.risk == Risk::Destructive, tool.name == "send_message");
    }
    assert_eq!(TOOLS.iter().find(|t| t.name == "read_room").unwrap().risk, Risk::Read);
}

#[test]
fn status_and_list_rooms_answer_at_once() {
    let (mut a, _, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    let status = done(a.execute(&mut host, "c1", "status", "{}", now));
    assert_eq!(status.outcome, Outcome::Ok);
    let data: Value = serde_json::from_str(&status.data).unwrap();
    assert_eq!(data["signed_in"], true);
    assert_eq!(data["current_room"], Value::Null);
    let rooms = done(a.execute(&mut host, "c2", "list_rooms", "", now));
    let data: Value = serde_json::from_str(&rooms.data).unwrap();
    assert_eq!(data["rooms"], json!([{"id": "!team:x", "name": "Team"}, {"id": "!home:x", "name": "Home"}]));
    // Opening a room by name makes it the current one.
    assert_eq!(done(a.execute(&mut host, "c3", "open_room", r#"{"room":"team"}"#, now)).outcome, Outcome::Ok);
    let status = done(a.execute(&mut host, "c4", "status", "{}", now));
    assert!(status.data.contains("!team:x"));
}

#[test]
fn signed_out_is_unavailable_except_status() {
    let (mut a, _, _) = assistant();
    let mut host = Fake::new();
    host.account = None;
    let now = Instant::now();
    let status = done(a.execute(&mut host, "c1", "status", "{}", now));
    assert_eq!(status.outcome, Outcome::Ok);
    assert!(status.data.contains(r#""signed_in":false"#));
    for (tool, args) in [
        ("list_rooms", "{}"),
        ("open_room", r#"{"room":"Team"}"#),
        ("draft_message", r#"{"room":"Team","text":"hi"}"#),
        ("read_room", r#"{"room":"Team"}"#),
        ("send_message", r#"{"room":"Team","text":"hi"}"#),
        ("open_mini_app", r#"{"app":"poll"}"#),
    ] {
        assert_eq!(done(a.execute(&mut host, "c", tool, args, now)).outcome, Outcome::Unavailable, "{tool}");
    }
    assert!(host.reads.is_empty() && host.sends.is_empty() && host.prompt.is_none());
}

#[test]
fn arguments_are_strict() {
    let (mut a, _, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    let long = "x".repeat(MAX_TEXT_BYTES + 1);
    for (tool, args) in [
        ("nope", "{}"),
        ("status", "[1]"),
        ("list_rooms", r#"{"all":true}"#),
        ("open_room", "{}"),
        ("open_room", r#"{"room":7}"#),
        ("open_room", r#"{"room":"  "}"#),
        ("open_room", r#"{"room":"Nowhere"}"#),
        ("open_room", r#"{"room":"!other:x"}"#),
        ("read_room", r#"{"room":"Team","limit":51}"#),
        ("read_room", r#"{"room":"Team","limit":0}"#),
        ("read_room", r#"{"room":"Team","limit":"5"}"#),
        ("draft_message", &format!(r#"{{"room":"Team","text":"{long}"}}"#)),
        ("send_message", r#"{"room":"Team","text":"hi","silent":true}"#),
    ] {
        let reply = done(a.execute(&mut host, "c", tool, args, now));
        assert_eq!(reply.outcome, Outcome::Refused, "{tool} {args}");
    }
    assert!(host.opened.is_empty() && host.prompt.is_none());
    // Two rooms with one name need the id.
    host.rooms.push(RoomRef { id: "!team2:x".into(), name: "Team".into() });
    let reply = done(a.execute(&mut host, "c", "open_room", r#"{"room":"Team"}"#, now));
    assert_eq!(reply.outcome, Outcome::Refused);
    assert!(reply.text.contains("!team2:x"));
}

#[test]
fn a_draft_never_sends() {
    let (mut a, replies, _) = assistant();
    let mut host = Fake::new();
    let reply = done(a.execute(&mut host, "c1", "draft_message", r#"{"room":"Home","text":"On my way"}"#, Instant::now()));
    assert_eq!(reply.outcome, Outcome::Ok);
    assert!(reply.data.contains(r#""sent":false"#));
    assert_eq!(host.drafts, vec![("!home:x".to_string(), "On my way".to_string())]);
    assert_eq!(host.opened, vec!["!home:x".to_string()]);
    assert!(host.sends.is_empty() && host.prompt.is_none() && take(&replies).is_empty());
}

#[test]
fn a_read_waits_for_the_grant_and_a_denial_reads_nothing() {
    let (mut a, replies, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    pending(a.execute(&mut host, "c1", "read_room", r#"{"room":"Team","limit":5}"#, now));
    let prompt = host.prompt.clone().expect("the read sheet is up");
    assert_eq!(prompt.room.id, "!team:x");
    assert_eq!(prompt.kind, PromptKind::ReadGrant { limit: 5 });
    assert!(host.reads.is_empty(), "nothing is read before the person answers");
    a.answer(&mut host, Choice::Deny);
    assert!(host.prompt.is_none() && host.reads.is_empty());
    let replies = take(&replies);
    assert_eq!(replies.len(), 1);
    assert_eq!((replies[0].call_id.as_str(), replies[0].outcome), ("c1", Outcome::Denied));
    // Denied once is not remembered: the next read asks again.
    pending(a.execute(&mut host, "c2", "read_room", r#"{"room":"Team"}"#, now));
    assert!(host.prompt.is_some());
}

#[test]
fn an_unanswered_sheet_times_out_as_a_denial() {
    let (mut a, replies, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    pending(a.execute(&mut host, "c1", "read_room", r#"{"room":"Team"}"#, now));
    pending(a.execute(&mut host, "c2", "send_message", r#"{"room":"Home","text":"hi"}"#, now + Duration::from_secs(30)));
    a.expire(&mut host, now + Duration::from_secs(44));
    assert!(take(&replies).is_empty(), "not yet");
    a.expire(&mut host, now + PROMPT_TIMEOUT);
    let r = take(&replies);
    assert_eq!(r.iter().map(|r| (r.call_id.as_str(), r.outcome)).collect::<Vec<_>>(), [("c1", Outcome::Denied)]);
    // The send sheet moves to the front, then times out on its own clock.
    assert_eq!(host.prompt.as_ref().map(|p| p.call_id.as_str()), Some("c2"));
    a.expire(&mut host, now + Duration::from_secs(30) + PROMPT_TIMEOUT);
    assert_eq!(take(&replies)[0].outcome, Outcome::Denied);
    assert!(host.prompt.is_none() && host.reads.is_empty() && host.sends.is_empty());
}

#[test]
fn always_persists_per_account_and_revoke_asks_again() {
    let (mut a, replies, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    pending(a.execute(&mut host, "c1", "read_room", r#"{"room":"Team","limit":50}"#, now));
    a.answer(&mut host, Choice::Always);
    let (account, room, limit, finish) = host.reads.pop().expect("read after the grant");
    assert_eq!((account.as_str(), room.as_str(), limit), ("@alice:x", "!team:x", READ_CAP), "the limit is clamped");
    finish(Ok(messages()));
    let r = take(&replies);
    assert_eq!(r[0].outcome, Outcome::Ok);
    assert!(r[0].data.contains(r#""body":"hi""#));
    // The grant is reused for this account: no sheet.
    pending(a.execute(&mut host, "c2", "read_room", r#"{"room":"!team:x"}"#, now));
    assert!(host.prompt.is_none());
    assert_eq!(host.reads.len(), 1);
    host.reads.clear();
    // Another account on this device does not inherit it.
    host.account = Some("@bob:x".into());
    pending(a.execute(&mut host, "c3", "read_room", r#"{"room":"Team"}"#, now));
    assert!(host.prompt.is_some(), "bob is asked");
    assert!(host.reads.is_empty());
    a.answer(&mut host, Choice::Deny);
    // Revoking in Settings makes alice's next read ask again.
    host.account = Some("@alice:x".into());
    a.grants_mut().revoke("@alice:x", "!team:x").unwrap();
    pending(a.execute(&mut host, "c4", "read_room", r#"{"room":"Team"}"#, now));
    assert!(host.prompt.is_some());
    assert!(host.reads.is_empty());
}

#[test]
fn a_message_is_sent_only_after_the_person_confirms() {
    let (mut a, replies, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    pending(a.execute(&mut host, "c1", "send_message", r#"{"room":"Team","text":"Ship it at 5"}"#, now));
    let prompt = host.prompt.clone().expect("the send sheet is up");
    assert_eq!(prompt.kind, PromptKind::Send { text: "Ship it at 5".into() }, "the exact text");
    assert!(host.sends.is_empty(), "nothing is sent before the person confirms");
    a.answer(&mut host, Choice::Deny);
    assert!(host.sends.is_empty());
    assert_eq!(take(&replies)[0].outcome, Outcome::Denied);
    pending(a.execute(&mut host, "c2", "send_message", r#"{"room":"Team","text":"Ship it at 5"}"#, now));
    a.answer(&mut host, Choice::Once);
    let (account, room, text, finish) = host.sends.pop().expect("sent after the yes");
    assert_eq!((account.as_str(), room.as_str(), text.as_str()), ("@alice:x", "!team:x", "Ship it at 5"));
    assert!(take(&replies).is_empty(), "the answer waits for Matrix");
    finish(Ok(json!({})));
    let r = take(&replies);
    assert_eq!((r[0].call_id.as_str(), r[0].outcome), ("c2", Outcome::Ok));
    assert!(r[0].data.contains(r#""sent":true"#));
}

#[test]
fn an_account_switch_ends_waiting_calls_and_drops_late_results() {
    let (mut a, replies, generation) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    // A read already running and a send waiting on its sheet.
    a.grants_mut().grant("@alice:x", "!team:x").unwrap();
    pending(a.execute(&mut host, "c1", "read_room", r#"{"room":"Team"}"#, now));
    pending(a.execute(&mut host, "c2", "send_message", r#"{"room":"Home","text":"hi"}"#, now));
    assert!(host.prompt.is_some());
    // Logout and a new login, from the Matrix thread.
    generation.fetch_add(1, Ordering::AcqRel);
    host.account = Some("@bob:x".into());
    a.account_changed(&mut host);
    assert!(host.prompt.is_none(), "the sheet closes");
    let mut r = take(&replies);
    r.sort_by(|a, b| a.call_id.cmp(&b.call_id));
    assert_eq!(
        r.iter().map(|r| (r.call_id.as_str(), r.outcome)).collect::<Vec<_>>(),
        [("c1", Outcome::Unavailable), ("c2", Outcome::Unavailable)]
    );
    // The previous account's read finishing late reaches no one.
    let (_, _, _, finish) = host.reads.pop().unwrap();
    finish(Ok(messages()));
    assert!(take(&replies).is_empty());
    // The late sheet answer cannot send either.
    a.answer(&mut host, Choice::Once);
    assert!(host.sends.is_empty());
}

#[test]
fn a_result_racing_the_switch_is_unavailable_not_data() {
    let (mut a, replies, generation) = assistant();
    let mut host = Fake::new();
    a.grants_mut().grant("@alice:x", "!team:x").unwrap();
    pending(a.execute(&mut host, "c1", "read_room", r#"{"room":"Team"}"#, Instant::now()));
    // The account changes on the Matrix thread; the read ends before the UI hears of it.
    generation.fetch_add(1, Ordering::AcqRel);
    let (_, _, _, finish) = host.reads.pop().unwrap();
    finish(Ok(messages()));
    let r = take(&replies);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].outcome, Outcome::Unavailable);
    assert!(r[0].data.is_empty(), "no messages of the old account");
    // And the UI's later sweep answers nothing twice.
    a.account_changed(&mut host);
    assert!(take(&replies).is_empty());
}

#[test]
fn a_switch_noticed_at_the_next_call_still_revokes() {
    let (mut a, replies, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    pending(a.execute(&mut host, "c1", "send_message", r#"{"room":"Home","text":"hi"}"#, now));
    host.account = Some("@bob:x".into());
    done(a.execute(&mut host, "c2", "status", "{}", now));
    assert_eq!(take(&replies)[0].outcome, Outcome::Unavailable);
    assert!(host.prompt.is_none());
}

#[test]
fn cancel_closes_the_sheet_and_drops_the_result() {
    let (mut a, replies, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    pending(a.execute(&mut host, "c1", "send_message", r#"{"room":"Home","text":"hi"}"#, now));
    a.cancel(&mut host, "c1");
    assert!(host.prompt.is_none());
    a.answer(&mut host, Choice::Once);
    assert!(host.sends.is_empty());
    a.grants_mut().grant("@alice:x", "!team:x").unwrap();
    pending(a.execute(&mut host, "c2", "read_room", r#"{"room":"Team"}"#, now));
    a.cancel(&mut host, "c2");
    let (_, _, _, finish) = host.reads.pop().unwrap();
    finish(Ok(messages()));
    assert!(take(&replies).is_empty());
}

#[test]
fn only_the_reviewed_mini_app_opens_and_only_for_its_room() {
    let (mut a, _, _) = assistant();
    let mut host = Fake::new();
    let now = Instant::now();
    assert_eq!(done(a.execute(&mut host, "c1", "open_mini_app", r#"{"app":"poll"}"#, now)).outcome, Outcome::Refused);
    host.reviewed = Some(ReviewedApp { id: "org.example.poll".into(), name: "Poll".into(), room: Some("!team:x".into()) });
    assert_eq!(done(a.execute(&mut host, "c2", "open_mini_app", r#"{"app":"chess"}"#, now)).outcome, Outcome::Refused);
    let wrong_room = done(a.execute(&mut host, "c3", "open_mini_app", r#"{"app":"Poll","room":"Home"}"#, now));
    assert_eq!(wrong_room.outcome, Outcome::Refused);
    assert!(host.opened_apps.is_empty());
    let ok = done(a.execute(&mut host, "c4", "open_mini_app", r#"{"app":"poll","room":"Team"}"#, now));
    assert_eq!(ok.outcome, Outcome::Ok);
    assert!(ok.text.contains("Run"), "the person still grants it");
    assert_eq!(host.opened_apps, vec!["org.example.poll".to_string()]);
}

#[test]
fn a_big_read_drops_the_oldest_messages_to_fit() {
    let body = "y".repeat(1000);
    let many: Vec<Value> = (0..30).map(|i| json!({"sender": "Bob", "body": format!("{i}{body}")})).collect();
    let reply = read_reply("c1", "Team", json!({"id": "!team:x"}), json!({"messages": many}));
    assert!(!reply.data.is_empty() && reply.data.len() <= MAX_DATA_BYTES);
    let data: Value = serde_json::from_str(&reply.data).unwrap();
    let kept = data["messages"].as_array().unwrap();
    assert!(kept.len() < 30 && kept.last().unwrap()["body"].as_str().unwrap().starts_with("29"), "the newest stay");
    assert_eq!(data["omitted"].as_u64().unwrap() as usize, 30 - kept.len());
}
