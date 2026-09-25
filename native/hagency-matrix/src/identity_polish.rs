//! Agent identity polish (board #11, parity with the retained bridge):
//! display-name reconciliation, approval-DM power levels, the owner-absent
//! warning text, and the send-retry warning — the exact retained shapes as
//! pure functions, so the wiring points stay one-liners.
//!
//! TS anchors: lib/matrix-agent-profile.js:1-30 (display name),
//! bridge-matrix.js:2561-2576 (`approvalRoomPowerLevels`),
//! :8861-8887 (`ensureApprovalDmRestricted` normalize+compare),
//! :9267-9297 (`warnIfOwnerCannotSeeApprovalRoom`),
//! :10888-10950 (membership failure → re-invite, rejoin, resend).

use crate::{CancellationToken, Error, http::Http};
use serde_json::{Value, json};

/// The machine-generated names a reconciliation may overwrite
/// (lib/matrix-agent-profile.js:22): a user's custom profile always wins.
pub fn is_machine_generated(current: &str, mxid: &str, agent_name: &str) -> bool {
    let localpart = &mxid[1..mxid.find(':').unwrap_or(mxid.len())];
    [mxid, localpart, agent_name, &format!("🤖 {agent_name}")].contains(&current)
}

/// The desired display name (lib/matrix-agent-profile.js:6-8): trimmed,
/// capped at 128 characters, never empty.
pub fn desired_display_name(display_name: &str) -> Option<String> {
    let desired = display_name.trim().chars().take(128).collect::<String>();
    (!desired.is_empty()).then_some(desired)
}

/// The approval-room power levels (bridge-matrix.js:2561-2576), verbatim:
/// everything that mutates the room sits at 100, only the room's own
/// creator-actor holds it, and message events stay open (events_default 0)
/// so the owner can still talk.
pub fn approval_room_power_levels(actor_mxid: &str) -> Result<Value, Error> {
    if !actor_mxid.starts_with('@') || !actor_mxid.contains(':') {
        return Err(Error::Config);
    }
    Ok(json!({
        "ban": 100,
        "events_default": 0,
        "invite": 100,
        "kick": 100,
        "notifications": {"room": 100},
        "redact": 100,
        "state_default": 100,
        "users": {actor_mxid: 100},
        "users_default": 0,
    }))
}

/// The normalized comparison subset (bridge-matrix.js:8878-8886): exactly the
/// ten keys `approvalRoomPowerLevels` owns — a current state carrying any
/// other key compares equal on these and is left alone; a difference on any
/// of them is a difference.
pub fn normalized_power_levels(current: &Value) -> Value {
    json!({
        "ban": current.get("ban").cloned().unwrap_or(Value::Null),
        "events_default": current.get("events_default").cloned().unwrap_or(Value::Null),
        "invite": current.get("invite").cloned().unwrap_or(Value::Null),
        "kick": current.get("kick").cloned().unwrap_or(Value::Null),
        "notifications": current.get("notifications").cloned().unwrap_or(Value::Null),
        "redact": current.get("redact").cloned().unwrap_or(Value::Null),
        "state_default": current.get("state_default").cloned().unwrap_or(Value::Null),
        "users": current.get("users").cloned().unwrap_or(Value::Null),
        "users_default": current.get("users_default").cloned().unwrap_or(Value::Null),
    })
}

/// Whether the room's power levels must be (re)written
/// (bridge-matrix.js:8877-8887): absent state, or a normalized difference.
pub fn power_levels_differ(current: Option<&Value>, expected: &Value) -> bool {
    match current {
        None => true,
        Some(current) => normalized_power_levels(current) != *expected,
    }
}

/// The owner-absent warning (bridge-matrix.js:9279-9283), verbatim words.
pub fn owner_absent_warning(agent: Option<&str>, room_id: &str, owner: &str) -> String {
    format!(
        "approval request for {} was delivered to {}, but its owner {} is NOT in that room \
         — invited and never joined, or since departed. Nobody who can decide will see it. \
         Remedy: have the owner accept the invitation to that room, or point \
         HAGENCY_OWNER_DM_ROOM (or the binding) at a room they are actually in.",
        agent.unwrap_or("an agent"),
        room_id,
        owner
    )
}

/// The send-retry warning (bridge-matrix.js:10946-10948), verbatim words.
pub fn send_retry_warning(room_id: &str, reason: &str) -> String {
    format!("sendAsAgent failed in room {room_id} (after auto-join retry): {reason}")
}

/// The agent's own rejoin (bridge-matrix.js:10936-10943, the join half of the
/// retained invite-then-join pair): POST /join/{roomId} as the agent. A kicked
/// member needs a fresh invite no agent can mint for itself — the caller
/// surfaces the warning and keeps the failure non-terminal when this refuses.
pub async fn agent_rejoin(http: &Http, room_id: &str, cancel: &CancellationToken) -> Result<(), Error> {
    let response = http
        .post(
            &["_matrix", "client", "v3", "join", room_id],
            "{}".to_owned(),
            cancel,
        )
        .await?;
    response.success()?;
    Ok(())
}

/// Display-name reconciliation (lib/matrix-agent-profile.js:3-30): GET the
/// current name, overwrite only a machine-generated one, PUT, read back. A
/// custom name wins silently (`changed: false, custom: true`); a mismatched
/// readback is an error, never a silent no-op.
pub async fn reconcile_display_name(
    http: &Http,
    mxid: &str,
    agent_name: &str,
    display_name: &str,
    cancel: &CancellationToken,
) -> Result<bool, Error> {
    let Some(desired) = desired_display_name(display_name) else {
        return Ok(false);
    };
    let read = http
        .request(
            &["_matrix", "client", "v3", "profile", mxid, "displayname"],
            None,
            cancel,
        )
        .await?
        .success()?;
    let current = read
        .get("displayname")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if current == desired {
        return Ok(false);
    }
    if !current.is_empty() && !is_machine_generated(current, mxid, agent_name) {
        return Ok(false);
    }
    http.put(
        &["_matrix", "client", "v3", "profile", mxid, "displayname"],
        json!({"displayname": desired}).to_string(),
        cancel,
    )
    .await?
    .success()?;
    let readback = http
        .request(
            &["_matrix", "client", "v3", "profile", mxid, "displayname"],
            None,
            cancel,
        )
        .await?
        .success()?;
    if readback.get("displayname").and_then(Value::as_str) != Some(desired.as_str()) {
        return Err(Error::Wire);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_generated_names_are_exactly_the_retained_four() {
        let mxid = "@agent_project_request:example.test";
        assert!(is_machine_generated(mxid, mxid, "UsageWorker"));
        assert!(is_machine_generated("agent_project_request", mxid, "UsageWorker"));
        assert!(is_machine_generated("UsageWorker", mxid, "UsageWorker"));
        assert!(is_machine_generated("🤖 UsageWorker", mxid, "UsageWorker"));
        assert!(!is_machine_generated("Ada's agent", mxid, "UsageWorker"));
        assert!(!is_machine_generated("", mxid, "UsageWorker"));
    }

    #[test]
    fn desired_name_trims_and_caps_at_128_scalars() {
        assert_eq!(desired_display_name("  Vivian  "), Some("Vivian".into()));
        assert_eq!(desired_display_name("   "), None);
        assert_eq!(
            desired_display_name(&"𝕏".repeat(200)).map(|s| s.chars().count()),
            Some(128)
        );
    }

    #[test]
    fn power_levels_match_the_retained_shape() {
        let expected = approval_room_power_levels("@agent:example.test").unwrap();
        assert_eq!(
            expected,
            json!({
                "ban": 100, "events_default": 0, "invite": 100, "kick": 100,
                "notifications": {"room": 100}, "redact": 100, "state_default": 100,
                "users": {"@agent:example.test": 100}, "users_default": 0,
            })
        );
        assert!(approval_room_power_levels("agent:example.test").is_err());
        // Equal on the ten owned keys: no rewrite.
        assert!(!power_levels_differ(Some(&expected), &expected));
        // Any owned-key drift or absent state: rewrite.
        let mut drifted = expected.clone();
        drifted["invite"] = json!(0);
        assert!(power_levels_differ(Some(&drifted), &expected));
        assert!(power_levels_differ(None, &expected));
        // A foreign key compares on the owned subset only.
        let mut extra = expected.clone();
        extra["history_visibility"] = json!("shared");
        assert!(!power_levels_differ(Some(&extra), &expected));
    }

    #[test]
    fn owner_absent_warning_keeps_the_retained_words() {
        assert_eq!(
            owner_absent_warning(Some("worker"), "!dm:example.test", "@owner:example.test"),
            "approval request for worker was delivered to !dm:example.test, but its owner \
             @owner:example.test is NOT in that room — invited and never joined, or since \
             departed. Nobody who can decide will see it. Remedy: have the owner accept the \
             invitation to that room, or point HAGENCY_OWNER_DM_ROOM (or the binding) at a \
             room they are actually in."
        );
        assert!(owner_absent_warning(None, "!dm:example.test", "@owner:example.test")
            .starts_with("approval request for an agent was delivered to"));
    }

    #[test]
    fn send_retry_warning_keeps_the_retained_words() {
        assert_eq!(
            send_retry_warning("!room:example.test", "not in room"),
            "sendAsAgent failed in room !room:example.test (after auto-join retry): not in room"
        );
    }
}
