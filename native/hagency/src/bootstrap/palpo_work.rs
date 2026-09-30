//! Process what the outbound Palpo transport has taken into custody (TS parity:
//! `lib/fleet-outbound-client.js` `processOnce` + `lib/fleet-protocol.js`
//! `recordEvent` / `POST /api/fleet/v1/probe`).
//!
//! - Matrix lane: every App Service transaction Palpo relays is scanned for the
//!   fleet's connection-probe event sent by its representative; each one is
//!   recorded as a push receipt (at most 100 kept, as TS does).
//! - Work lane, `probe`: the named event must have arrived in such a
//!   transaction. It is then re-read as the representative, the room must be
//!   invite-only, unencrypted and joined by the representative, and the room is
//!   bound as the fleet's reception. The receipt rides the next `/updates`.
//! - Work lane, `request`: not processed here yet; it stays in custody, never
//!   dropped.
//!
//! A failure is retried later, never turned into a terminal refusal: the bridge
//! does not decide the connection is dead.
use super::probe::{PROBE_EVENT, ProbeError, ProbeReceipt, decide};
use hagency_core::custody::{Kind, Lane};
use hagency_palpo::{Adapter, CancellationToken, ProbeReceipts};
use hagency_store::{DomainStore, private};
use reqwest::{StatusCode, Url, redirect::Policy};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

/// The fleet's App Service identity, from `palpo-appservice.json`.
pub(super) struct Appservice {
    homeserver: String,
    as_token: String,
    representative: String,
}
impl Appservice {
    pub(super) fn load(state: &Path, server_name: &str) -> Option<Self> {
        let raw = private::read_secret(&state.join("palpo-appservice.json")).ok()?;
        let value: Value = serde_json::from_slice(&raw).ok()?;
        let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        Some(Self {
            homeserver: text("homeserver")?,
            as_token: text("as_token")?,
            representative: format!("@{}:{server_name}", text("sender_localpart")?),
        })
    }
}

/// Durable probe bookkeeping: push receipts seen on the Matrix lane and the
/// verified receipts awaiting publication. Both files are owner-private.
pub(super) struct Probes {
    seen: PathBuf,
    outbox: PathBuf,
    lock: Mutex<()>,
}
impl Probes {
    pub(super) fn new(state: &Path) -> Arc<Self> {
        Arc::new(Self {
            seen: state.join("palpo-probe-events.json"),
            outbox: state.join("palpo-probe-receipts.json"),
            lock: Mutex::new(()),
        })
    }
    fn read(path: &Path) -> Vec<Value> {
        private::read_secret(path)
            .ok()
            .and_then(|raw| serde_json::from_slice::<Vec<Value>>(&raw).ok())
            .unwrap_or_default()
    }
    fn write(path: &Path, rows: &[Value]) {
        if let Ok(bytes) = serde_json::to_vec(rows) {
            let _ = private::replace(path, &bytes);
        }
    }
    fn record_event(&self, row: Value) {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut rows = Self::read(&self.seen);
        rows.retain(|r| r.get("sourceEventId") != row.get("sourceEventId"));
        rows.push(row);
        let excess = rows.len().saturating_sub(100);
        rows.drain(..excess);
        Self::write(&self.seen, &rows);
    }
    fn event(&self, id: &str) -> Option<Value> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        Self::read(&self.seen)
            .into_iter()
            .find(|r| r.get("sourceEventId").and_then(Value::as_str) == Some(id))
    }
    fn queue_receipt(&self, receipt: Value) {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut rows = Self::read(&self.outbox);
        if !rows.contains(&receipt) {
            rows.push(receipt);
        }
        Self::write(&self.outbox, &rows);
    }
}
impl ProbeReceipts for Probes {
    fn pending(&self) -> Vec<Value> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        Self::read(&self.outbox)
    }
    fn published(&self, receipts: &[Value]) {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut rows = Self::read(&self.outbox);
        rows.retain(|r| !receipts.contains(r));
        Self::write(&self.outbox, &rows);
    }
}

/// Reads as the representative through the App Service token (masquerade).
struct Reader {
    client: reqwest::Client,
    origin: Url,
    token: String,
    user: String,
}
impl Reader {
    fn new(appservice: &Appservice) -> Option<Self> {
        Some(Self {
            client: reqwest::Client::builder()
                .redirect(Policy::none())
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(20))
                .build()
                .ok()?,
            origin: Url::parse(&appservice.homeserver).ok()?,
            token: appservice.as_token.clone(),
            user: appservice.representative.clone(),
        })
    }
    async fn get(&self, segments: &[&str]) -> Result<Option<Value>, ()> {
        self.call_as(&self.user, reqwest::Method::GET, segments, &[], None).await
    }
    /// One App Service request acting as `user` (a member of the fleet namespace).
    async fn call_as(
        &self,
        user: &str,
        method: reqwest::Method,
        segments: &[&str],
        query: &[(&str, &str)],
        body: Option<Value>,
    ) -> Result<Option<Value>, ()> {
        let mut url = self.origin.clone();
        url.path_segments_mut().map_err(|_| ())?.extend(segments);
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("user_id", user);
            for (k, v) in query {
                pairs.append_pair(k, v);
            }
        }
        let mut request = self.client.request(method, url).bearer_auth(&self.token);
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(serde_json::to_vec(&body).map_err(|_| ())?);
        }
        let response = request.send().await.map_err(|_| ())?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if response.status() != StatusCode::OK {
            return Err(());
        }
        let bytes = response.bytes().await.map_err(|_| ())?;
        if bytes.len() > 1024 * 1024 {
            return Err(());
        }
        serde_json::from_slice(&bytes).map(Some).map_err(|_| ())
    }
    async fn event(&self, room: &str, event: &str) -> Result<Value, ()> {
        self.get(&["_matrix", "client", "v3", "rooms", room, "event", event]).await?.ok_or(())
    }
    /// TS `plaintextPrivate` reads, in the shape `probe::decide` takes.
    async fn room_facts(&self, room: &str) -> Result<Value, ()> {
        let members = self.get(&["_matrix", "client", "v3", "rooms", room, "joined_members"]).await?.ok_or(())?;
        let rules = self
            .get(&["_matrix", "client", "v3", "rooms", room, "state", "m.room.join_rules", ""])
            .await?
            .ok_or(())?;
        let encryption = self
            .get(&["_matrix", "client", "v3", "rooms", room, "state", "m.room.encryption", ""])
            .await?
            .unwrap_or(Value::Null);
        Ok(json!({"joined": members.get("joined").cloned().unwrap_or(Value::Null),
            "join_rules": rules, "encryption": encryption}))
    }
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default() as i64;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.000Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn attempt(lane: &str) -> String {
    let mut bytes = [0u8; 8];
    let _ = getrandom::fill(&mut bytes);
    format!("host_{lane}_{}", bytes.iter().map(|b| format!("{b:02x}")).collect::<String>())
}

enum Outcome {
    Idle,
    Done,
    Later,
}

/// Matrix lane: record the probe events of one relayed transaction.
async fn matrix_once(adapter: &Adapter, probes: &Probes, fleet: &str, representative: &str, generation: u64) -> Outcome {
    let Ok(Some(work)) = adapter.take(Lane::Matrix, attempt("matrix")).await else {
        return Outcome::Idle;
    };
    let transaction = work.payload.get("transactionId").and_then(Value::as_str).unwrap_or_default().to_owned();
    let events = work
        .payload
        .get("body")
        .and_then(|b| b.get("events"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut recorded = 0;
    for event in events {
        let content = event.get("content").cloned().unwrap_or(Value::Null);
        let challenge = content.get("challenge").and_then(Value::as_str).unwrap_or_default();
        if event.get("type").and_then(Value::as_str) != Some(PROBE_EVENT)
            || event.get("sender").and_then(Value::as_str) != Some(representative)
            || content.get("fleetId").and_then(Value::as_str) != Some(fleet)
            || !(16..=128).contains(&challenge.len())
            || !challenge.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            continue;
        }
        let (Some(room), Some(id)) = (
            event.get("room_id").and_then(Value::as_str),
            event.get("event_id").and_then(Value::as_str),
        ) else {
            continue;
        };
        probes.record_event(json!({"sourceRoomId": room, "sourceEventId": id, "challenge": challenge,
            "receivedAt": now_iso(), "mode": "edge", "generation": generation, "transactionId": transaction}));
        recorded += 1;
    }
    match adapter.complete(work.ticket, json!({"probes": recorded})).await {
        Ok(()) => Outcome::Done,
        Err(_) => Outcome::Later,
    }
}

/// Work lane: verify one probe and bind the reception.
async fn work_once(adapter: &Adapter, probes: &Probes, domain: &DomainStore, reader: &Reader, fleet: &str) -> Outcome {
    let Ok(Some(work)) = adapter.take(Lane::Work, attempt("work")).await else {
        return Outcome::Idle;
    };
    if work.kind != Kind::Probe {
        // Requests are admitted by a later slice; keep them in custody.
        let _ = adapter.retry_later(work.ticket).await;
        return Outcome::Later;
    }
    let body = work.payload.clone();
    let id = body.get("sourceEventId").and_then(Value::as_str).unwrap_or_default().to_owned();
    let Some(seen) = probes.event(&id) else {
        // The relayed transaction has not been processed yet (TS probe_pending).
        let _ = adapter.retry_later(work.ticket).await;
        return Outcome::Later;
    };
    let receipt = ProbeReceipt {
        source_room_id: seen["sourceRoomId"].as_str().unwrap_or_default().to_owned(),
        source_event_id: id.clone(),
        challenge: seen["challenge"].as_str().unwrap_or_default().to_owned(),
    };
    let verified = async {
        let registration = domain.provisioning_registration(fleet.to_owned()).await.map_err(|_| None)?;
        let event = reader.event(&receipt.source_room_id, &id).await.map_err(|_| None)?;
        let facts = reader.room_facts(&receipt.source_room_id).await.map_err(|_| None)?;
        let bound = decide(&registration, Some(&receipt), &body, &event, &facts).map_err(Some)?;
        domain
            .bind_reception(registration.fleet_id.clone(), registration.generation, bound.source_room_id.clone())
            .await
            .map_err(|_| None)?;
        Ok::<_, Option<ProbeError>>(bound)
    }
    .await;
    match verified {
        Ok(bound) => {
            let published = json!({"v": 1, "received": true, "fleetId": fleet,
                "sourceRoomId": bound.source_room_id, "sourceEventId": bound.source_event_id,
                "challenge": bound.challenge, "receivedAt": seen["receivedAt"], "mode": "edge",
                "generation": seen["generation"]});
            probes.queue_receipt(published.clone());
            match adapter.complete(work.ticket, published).await {
                Ok(()) => Outcome::Done,
                Err(_) => Outcome::Later,
            }
        }
        Err(refusal) => {
            eprintln!(
                "palpo probe {id} not verified yet: {}",
                refusal.map(|e| e.to_string()).unwrap_or_else(|| "Matrix or store unavailable".into())
            );
            let _ = adapter.retry_later(work.ticket).await;
            Outcome::Later
        }
    }
}

/// The fleet's approval bot accepts invites to private approval rooms (TS: the
/// bridge bot's invite poll, `handleBotInvite`, which in audit mode accepts and
/// lets the later verification decide). Accepted: an encrypted, invite-only
/// room, invited by a local human outside the fleet's own namespace. Palpo then
/// verifies the room holds exactly the owner and this bot.
async fn approval_invites_once(reader: &Reader, bot: &str, fleet: &str, server: &str) {
    let filter = r#"{"room":{"timeline":{"limit":0},"ephemeral":{"types":[]},"account_data":{"types":[]}},"presence":{"types":[]},"account_data":{"types":[]}}"#;
    let Ok(Some(sync)) = reader
        .call_as(bot, reqwest::Method::GET, &["_matrix", "client", "v3", "sync"], &[("timeout", "0"), ("filter", filter)], None)
        .await
    else {
        return;
    };
    let Some(invites) = sync.pointer("/rooms/invite").and_then(Value::as_object) else { return };
    for (room, invite) in invites {
        let state = invite.pointer("/invite_state/events").and_then(Value::as_array).cloned().unwrap_or_default();
        let find = |kind: &str| state.iter().find(|e| e.get("type").and_then(Value::as_str) == Some(kind));
        let encrypted = find("m.room.encryption").and_then(|e| e.pointer("/content/algorithm")).and_then(Value::as_str)
            == Some("m.megolm.v1.aes-sha2");
        let invite_only = find("m.room.join_rules").and_then(|e| e.pointer("/content/join_rule")).and_then(Value::as_str)
            == Some("invite");
        let inviter = state
            .iter()
            .find(|e| e.get("type").and_then(Value::as_str) == Some("m.room.member")
                && e.get("state_key").and_then(Value::as_str) == Some(bot))
            .and_then(|e| e.get("sender").and_then(Value::as_str))
            .unwrap_or_default();
        let local_human = inviter.ends_with(&format!(":{server}")) && !inviter.starts_with(&format!("@{fleet}_"));
        if !(encrypted && invite_only && local_human) {
            eprintln!("palpo approval bot: leaving invite to {room} pending (encrypted={encrypted} invite_only={invite_only} inviter={inviter})");
            continue;
        }
        let joined = reader
            .call_as(bot, reqwest::Method::POST, &["_matrix", "client", "v3", "join", room], &[], Some(json!({})))
            .await;
        eprintln!("palpo approval bot: join {room} invited by {inviter}: {}", if joined.is_ok() { "ok" } else { "failed, retrying" });
    }
}

/// Runs beside the transport lanes until cancelled.
pub(super) async fn run(
    adapter: &Adapter,
    probes: &Probes,
    domain: &DomainStore,
    appservice: &Appservice,
    fleet: &str,
    generation: u64,
    cancel: &CancellationToken,
) {
    let Some(reader) = Reader::new(appservice) else {
        eprintln!("palpo work: invalid homeserver in palpo-appservice.json; probes wait");
        return;
    };
    let server = appservice.representative.split_once(':').map(|(_, s)| s.to_owned()).unwrap_or_default();
    let bot = domain
        .provisioning_registration(fleet.to_owned())
        .await
        .map(|r| r.approval_bot_mxid)
        .unwrap_or_default();
    let mut invites_due = std::time::Instant::now();
    while !cancel.is_cancelled() {
        if !bot.is_empty() && std::time::Instant::now() >= invites_due {
            approval_invites_once(&reader, &bot, fleet, &server).await;
            invites_due = std::time::Instant::now() + Duration::from_secs(15);
        }
        let matrix = matrix_once(adapter, probes, fleet, &appservice.representative, generation).await;
        let work = work_once(adapter, probes, domain, &reader, fleet).await;
        let pause = match (matrix, work) {
            (Outcome::Done, _) | (_, Outcome::Done) => Duration::from_millis(100),
            (_, Outcome::Later) | (Outcome::Later, _) => Duration::from_secs(3),
            _ => Duration::from_secs(1),
        };
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(pause) => {}
        }
    }
}
