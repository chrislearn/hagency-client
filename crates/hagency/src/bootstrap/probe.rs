//! Fleet connection probe (TS parity: `lib/fleet-protocol.js:98-153`).
//!
//! The probe is the runtime reception-room binding: a `com.hagency.connection.probe.v1`
//! event in a room, re-read and verified, binds that room as the fleet's reception
//! room — replacing the offline step. This module ports the TS `recordEvent` +
//! `POST /api/fleet/v1/probe` decision logic 1:1, with the Matrix read and the
//! durable write injected so the decision logic is pure and testable.

use hagency_core::authority::Registration;
use serde_json::Value;

/// Event type used by the shared fleet connection protocol.
pub const PROBE_EVENT: &str = "com.hagency.connection.probe.v1";

/// A verified probe delivery receipt (`lib/fleet-protocol.js:113-115`).
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeReceipt {
    pub source_room_id: String,
    pub source_event_id: String,
    pub challenge: String,
}

/// TS failure vocabulary (`lib/fleet-protocol.js` `fail(code, message, status)`).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProbeError {
    #[error("fleet_unavailable: Fleet registration is unavailable.")]
    FleetUnavailable,
    #[error("wrong_fleet: Fleet scope does not match.")]
    WrongFleet,
    #[error("probe_pending: The matching authenticated Palpo delivery has not been received.")]
    ProbePending,
    #[error("probe_mismatch: Probe event does not match the delivery receipt.")]
    ProbeMismatch,
    #[error("representative_absent: The representative has not joined the reception.")]
    RepresentativeAbsent,
    #[error("reception_conflict: This registration is already bound to another reception.")]
    ReceptionConflict,
    #[error("room_policy: Reception and project rooms must be invite-only and unencrypted.")]
    RoomPolicy,
}

/// Validate the probe event shape exactly as TS `recordEvent` does
/// (`lib/fleet-protocol.js:105-108`): sender is the representative, content
/// carries the fleet id and a 16-128 char `[a-zA-Z0-9_-]` challenge, the room
/// id and event id are well-formed.
pub fn receipt_from_event(registration: &Registration, event: &Value) -> Option<ProbeReceipt> {
    let sender = event.get("sender")?.as_str()?;
    let room_id = event.get("room_id")?.as_str()?;
    let event_id = event.get("event_id")?.as_str()?;
    let content = event.get("content")?;
    let fleet_id = content.get("fleetId")?.as_str()?;
    let challenge = content.get("challenge")?.as_str()?;
    if event.get("type")?.as_str()? != PROBE_EVENT
        || sender != registration.representative_mxid
        || fleet_id != registration.fleet_id
        || !valid_room_id(room_id, &registration.server_name)
        || !valid_event_id(event_id)
        || !valid_challenge(challenge)
    {
        return None;
    }
    Some(ProbeReceipt {
        source_room_id: room_id.to_owned(),
        source_event_id: event_id.to_owned(),
        challenge: challenge.to_owned(),
    })
}

fn valid_room_id(id: &str, server: &str) -> bool {
    id.starts_with('!')
        && id
            .split_once(':')
            .is_some_and(|(_, s)| !s.is_empty() && s.eq_ignore_ascii_case(server))
}
fn valid_event_id(id: &str) -> bool {
    id.starts_with('$') && id.len() > 1 && !id.chars().any(char::is_whitespace)
}
fn valid_challenge(challenge: &str) -> bool {
    (16..=128).contains(&challenge.len())
        && challenge
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// The `/api/fleet/v1/probe` decision (`lib/fleet-protocol.js:132-153`):
/// match the body against the receipt, re-verify the source event, require the
/// representative joined in an invite-only unencrypted room, refuse a conflict
/// with an already-bound reception, then return the room to bind.
///
/// `event` and `room_state` are the just-read Matrix authority facts; the
/// caller must have fetched them fresh (TS `readRoom`).
pub fn decide(
    registration: &Registration,
    receipt: Option<&ProbeReceipt>,
    body: &Value,
    event: &Value,
    room_state: &Value,
) -> Result<ProbeReceipt, ProbeError> {
    // TS:132-139 — body.fleetId must scope to this fleet, and the receipt must
    // match the body's sourceEventId / challenge / sourceRoomId.
    if body.get("fleetId").and_then(Value::as_str) != Some(registration.fleet_id.as_str()) {
        return Err(ProbeError::WrongFleet);
    }
    let receipt = receipt.ok_or(ProbeError::ProbePending)?;
    let body_event = body.get("sourceEventId").and_then(Value::as_str);
    let body_challenge = body.get("challenge").and_then(Value::as_str);
    let body_room = body.get("sourceRoomId").and_then(Value::as_str);
    if body_event != Some(receipt.source_event_id.as_str())
        || body_challenge != Some(receipt.challenge.as_str())
        || body_room != Some(receipt.source_room_id.as_str())
    {
        return Err(ProbeError::ProbePending);
    }
    // TS:140-143 — re-read the source event and require it to match the receipt.
    let observed = receipt_from_event(registration, event).ok_or(ProbeError::ProbeMismatch)?;
    if observed != *receipt {
        return Err(ProbeError::ProbeMismatch);
    }
    // TS:144-145 — the room must be invite-only, unencrypted, and the
    // representative must remain joined.
    if room_policy(room_state) {
        return Err(ProbeError::RoomPolicy);
    }
    let joined = room_state
        .get("joined")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    if !joined
        .iter()
        .any(|m| m == &registration.representative_mxid)
    {
        return Err(ProbeError::RepresentativeAbsent);
    }
    // TS:146-147 — an already-bound registration refuses a different reception.
    if !registration.reception_room_id.is_empty()
        && registration.reception_room_id != receipt.source_room_id
    {
        return Err(ProbeError::ReceptionConflict);
    }
    Ok(receipt.clone())
}

/// TS `plaintextPrivate` (`lib/fleet-protocol.js:89-95`): invite-only and not
/// megolm-encrypted.
fn room_policy(room_state: &Value) -> bool {
    let join_rule = room_state
        .get("join_rules")
        .and_then(|r| r.get("join_rule"))
        .and_then(Value::as_str);
    let encryption = room_state
        .get("encryption")
        .and_then(|e| e.get("algorithm"))
        .and_then(Value::as_str);
    join_rule != Some("invite") || encryption.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registration() -> Registration {
        serde_json::from_value(json!({
            "fleetId": format!("hf_{}", "a".repeat(32)),
            "generation": 1,
            "serverName": "example.test",
            "receptionRoomId": "",
            "representativeMxid": format!("@hf_{}_representative:example.test", "a".repeat(32)),
            "approvalBotMxid": "@approval:example.test",
        }))
        .unwrap()
    }
    fn probe_event(reg: &Registration, room: &str, event: &str, challenge: &str) -> Value {
        json!({
            "type": PROBE_EVENT,
            "sender": reg.representative_mxid.clone(),
            "room_id": room,
            "event_id": event,
            "content": {"fleetId": reg.fleet_id.clone(), "challenge": challenge},
        })
    }
    fn invite_room(reg: &Registration) -> Value {
        json!({
            "joined": {reg.representative_mxid.clone(): {}},
            "join_rules": {"join_rule": "invite"},
            "encryption": null,
        })
    }
    const ROOM: &str = "!reception:example.test";
    const EVENT: &str = "$probe-event";
    const CHALLENGE: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn probe_event_shape_accepts_valid_and_rejects_wrong_sender_or_challenge() {
        let reg = registration();
        let good = probe_event(&reg, ROOM, EVENT, CHALLENGE);
        let receipt = receipt_from_event(&reg, &good).expect("valid probe");
        assert_eq!(receipt.source_room_id, ROOM);
        // Wrong sender.
        let mut bad = good.clone();
        bad["sender"] = json!("@intruder:example.test");
        assert_eq!(receipt_from_event(&reg, &bad), None);
        // Short challenge.
        let bad = probe_event(&reg, ROOM, EVENT, "short");
        assert_eq!(receipt_from_event(&reg, &bad), None);
        // Wrong event type.
        let mut bad = good.clone();
        bad["type"] = json!("m.room.message");
        assert_eq!(receipt_from_event(&reg, &bad), None);
    }

    #[test]
    fn probe_decision_binds_the_reception_round_trip() {
        let reg = registration();
        let event = probe_event(&reg, ROOM, EVENT, CHALLENGE);
        let receipt = receipt_from_event(&reg, &event).unwrap();
        let body = json!({"fleetId": reg.fleet_id, "sourceEventId": EVENT,
            "challenge": CHALLENGE, "sourceRoomId": ROOM});
        let bound = decide(&reg, Some(&receipt), &body, &event, &invite_room(&reg)).unwrap();
        assert_eq!(bound.source_room_id, ROOM, "probe binds the reception room");
    }

    #[test]
    fn probe_decision_refuses_pending_mismatch_absent_and_conflict() {
        let reg = registration();
        let event = probe_event(&reg, ROOM, EVENT, CHALLENGE);
        let receipt = receipt_from_event(&reg, &event).unwrap();
        let body = json!({"fleetId": reg.fleet_id, "sourceEventId": EVENT,
            "challenge": CHALLENGE, "sourceRoomId": ROOM});
        // No matching receipt.
        assert_eq!(
            decide(&reg, None, &body, &event, &invite_room(&reg)),
            Err(ProbeError::ProbePending)
        );
        // Re-read event does not match the receipt.
        let other = probe_event(&reg, ROOM, "$other", CHALLENGE);
        assert_eq!(
            decide(&reg, Some(&receipt), &body, &other, &invite_room(&reg)),
            Err(ProbeError::ProbeMismatch)
        );
        // Representative absent.
        let absent = json!({"joined": {}, "join_rules": {"join_rule": "invite"}});
        assert_eq!(
            decide(&reg, Some(&receipt), &body, &event, &absent),
            Err(ProbeError::RepresentativeAbsent)
        );
        // Already bound elsewhere.
        let mut bound = registration();
        bound.reception_room_id = "!other:example.test".into();
        assert_eq!(
            decide(&bound, Some(&receipt), &body, &event, &invite_room(&bound)),
            Err(ProbeError::ReceptionConflict)
        );
    }
}
