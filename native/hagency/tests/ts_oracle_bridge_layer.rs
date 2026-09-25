//! TS test oracle — bridge-layer routing/trust/sync (board #72).
//!
//! The retained TS suite is the parity ORACLE. Each test below names one TS case
//! (`tests/*.test.js:<line>`) and states the exact observable outcome it asserts.
//! Where native has NO surface for the behaviour, the test is `#[ignore]`d with a
//! one-line reason — this task changes no product code (see `.peer/report-72.md`
//! for the full pass/gap/cited/skip map).
//!
//! Out-of-scope skips (declared in the board): legacy group commands
//! `!mkgroup`/`!bindroom`, and tmux probes (`!sessions`/`!mcp`/`!spy`/`!ctl`).
//! Those are reported, not encoded here.

/// TS `tests/room-trust.test.js` — `getRoomTrust`/`markRoomTrusted` classifier
/// (bridge-matrix.js:2841, 2854). Native has no room-trust table, allowlist or
/// `MATRIX_TRUST_MODE` surface: the collector derives membership/privacy facts
/// (`collector.rs`) but never classifies a room trusted/untrusted.

#[test]
#[ignore = "parity gap: no native room-trust classifier (getRoomTrust/markRoomTrusted)"]
fn ts_oracle_room_trust_allowlist_and_managed_and_inviter() {
    // TS asserts: allowlist room -> {trusted:true,reason:'allowlist'};
    // roomGroupMap room -> {trusted:true,reason:'managed'}; dmRooms room ->
    // 'managed'; trusted inviter -> 'trusted_inviter'; unknown -> 'unknown_room'.
    let _ = ();
}

#[test]
#[ignore = "parity gap: no native room-trust classifier (getRoomTrust/markRoomTrusted)"]
fn ts_oracle_room_trust_defaults_to_audit() {
    // TS asserts: MATRIX_TRUST_MODE defaults to 'audit'.
    let _ = ();
}

/// TS `tests/pending-invite-store.test.js` — `createPendingInviteStore`
/// (lib/pending-invite-store.js). Native has no pending-invite store: invited
/// rooms are not a contributor-facing record anywhere in native.

#[test]
#[ignore = "parity gap: no native pending-invite store (lib/pending-invite-store.js)"]
fn ts_oracle_pending_invite_captures_and_derives_server() {
    // TS asserts: upsert captures {room, agent, inviter, state:'pending'};
    // projectServer is DERIVED from the room id, never taken from the caller.
    let _ = ();
}

#[test]
#[ignore = "parity gap: no native pending-invite store (lib/pending-invite-store.js)"]
fn ts_oracle_pending_invite_idempotent_per_agent_key() {
    // TS asserts: re-upsert of the same (room, agent) does not multiply; one room
    // holds several agents keyed (room, agent) each with its own inviter.
    let _ = ();
}

#[test]
#[ignore = "parity gap: no native pending-invite store (lib/pending-invite-store.js)"]
fn ts_oracle_pending_invite_settlement_conflicts_and_never_prunes_pending() {
    // TS asserts: settle records decidedBy+decidedAt; a settled record does NOT
    // reopen on re-report; double-decide throws; deciding an absent record 404s;
    // only accepted/declined are decisions; list is pending-only newest-first; a
    // PENDING record is never pruned; every mutation persists.
    let _ = ();
}

/// TS `tests/representative-sync.test.js` — `startRepresentativeSyncCollector`
/// (lib/representative-sync.js). Native has no representative sync collector.

#[test]
#[ignore = "parity gap: no native representative sync collector (lib/representative-sync.js)"]
fn ts_oracle_representative_sync_gap_and_invite_and_cursor() {
    // TS asserts: an unprovable gap is reported once and preserves blocked
    // recovery; invites are delivered and retried before committing the cursor;
    // a stopped poll never dispatches stale results.
    let _ = ();
}

#[test]
#[ignore = "parity gap: no native representative sync collector (lib/representative-sync.js)"]
fn ts_oracle_representative_sync_pending_gap_circuit_break_boundary() {
    // TS asserts: pending gaps persist before cursor commit and recovery retries;
    // poison delivery circuit-breaks without consuming the cursor; gap recovery
    // stops at a recorded event and never guesses a history boundary.
    let _ = ();
}

/// TS `tests/invited-room-routing.test.js` — `onInvitedAgentRoomMessage`
/// (bridge-matrix.js:6873). Native has no invited-room message routing surface.

#[test]
#[ignore = "parity gap: no native invited-room message routing (bridge onInvitedAgentRoomMessage)"]
fn ts_oracle_invited_room_routing_direct_and_group_and_loop() {
    // TS asserts: a direct-room command replies through the admitted agent and a
    // send failure does not wedge sync; a joined agent without a starting device
    // cannot wedge another device; invited group routing wakes only mentioned
    // targets and never loops agent output.
    let _ = ();
}

/// TS `tests/join-backfill.test.js` — `pendingJoinBackfill`
/// (bridge-matrix.js:2940). **Covered since board #79**: the selector is
/// ported 1:1 as `hagency_matrix::join_backfill::pending_join_backfill`, and
/// all 13 TS cases are real green assertions in
/// `native/hagency-matrix/src/join_backfill.rs::tests`
/// (`ts_oracle_backfill_*`). This stub stays only as the map entry; see
/// `.peer/report-79.md`.

#[test]
fn ts_oracle_join_backfill_covered_in_hagency_matrix() {
    use hagency_matrix::join_backfill::{pending_join_backfill, Boundary};
    use serde_json::json;
    // One smoke assertion per TS property group; the full 13-case port is the
    // source module's own test set (same names, same fixtures).
    let bot = "@bot:matrix.test";
    let member = |kind: &str| {
        json!({"type":"m.room.member","sender":"@o:m.test","origin_server_ts":1,
            "state_key":bot,"content":{"membership":kind}})
    };
    // invite..end window (:141) and delivery between invite and join (:123).
    let chunk = json!([
        {"type":"m.room.message","event_id":"$b","sender":"@lin:m.test","origin_server_ts":2,"content":{"body":"b"}},
        member("invite")
    ]);
    let window = pending_join_backfill(Some(&chunk), bot, None);
    assert_eq!(window.events, vec!["$b".to_owned()]);
    assert_eq!(window.boundary, Boundary::InviteToEnd);
    // Fail-closed: no provable invite -> nothing routed (:150).
    let none = json!([{"type":"m.room.message","event_id":"$x","sender":"@lin:m.test","origin_server_ts":1,"content":{"body":"x"}}]);
    assert_eq!(pending_join_backfill(Some(&none), bot, None).boundary, Boundary::Unproven);
}

/// TS `tests/bot-commands-request.test.js` — `cmdRequest` reply half.
/// **Covered since board #79**: `bot_commands.rs::request_reply` renders the
/// TS text 1:1; `dispatch` returns `Dispatched::Request(args)` and
/// `bootstrap/commands.rs` renders it through the production command-notice
/// send path. The submit credential half (requester-token preference) has no
/// native surface: admission is Matrix-event-verified
/// (`authority.rs::verify_request` enforces request-id = authenticated event
/// id, `source.event_id == request.source_event_id`, and requester = sender
/// `source.sender == request.requester_mxid`, authority.rs:222-224), not an
/// agent-token POST — that one row stays a gap.

use hagency::bot_commands::{
    Dispatched, HostObservation, OfferServing, RequestEngagement, RequestOutcome, Acl,
    dispatch, request_reply,
};
use serde_json::json;

/// TS `bot-commands-request.test.js:343` `a malformed token amount is refused
/// without reaching the backend`: the reply names the offending word and no
/// engagement is created — the validation is synchronous (:526-536).
#[test]
fn ts_oracle_request_malformed_token_refused_before_backend() {
    let reply = request_reply(&["coding".into(), "four-hundred-thousand".into()], None);
    assert_eq!(reply.plain, "Not a token amount: four-hundred-thousand");
}

/// TS `:365` `a pending request is told WHY, in the project's own terms` —
/// `notWhitelisted` vs `overCeiling` are different next steps (:619-628).
#[test]
fn ts_oracle_request_pending_told_why() {
    let pending = |route: &str| RequestOutcome {
        engagement: Some(RequestEngagement {
            role: "coding".into(),
            requested_tokens: 400_000,
            allocated_tokens: 0,
            agent: "claude-agent".into(),
            auto_joined: false,
            route: Some(route.into()),
        }),
        ..RequestOutcome::default()
    };
    assert_eq!(
        request_reply(&["coding".into(), "400000".into()], Some(&pending("notWhitelisted"))).plain,
        "Requested coding for 400000 tokens — awaiting a decision, because this room is not on the contributor's whitelist."
    );
    assert_eq!(
        request_reply(&["coding".into(), "400000".into()], Some(&pending("overCeiling"))).plain,
        "Requested coding for 400000 tokens — awaiting a decision, because the amount is above what the serving agent has left."
    );
}

/// TS `:295` `the reply names the framework, model and reasoning level`;
/// `:312` a failure does NOT hand the project the provider's configuration
/// (`HAGENCY_*` never appears); `:330` an unknown configuration degrades to
/// the agent alone, not to a fabricated one (:596-612).
#[test]
fn ts_oracle_request_auto_join_disclosure() {
    let base = RequestEngagement {
        role: "coding".into(),
        requested_tokens: 400_000,
        allocated_tokens: 400_000,
        agent: "claude-agent".into(),
        auto_joined: true,
        route: None,
    };
    let disclosed = RequestOutcome {
        engagement: Some(base.clone()),
        serving: Some(OfferServing {
            framework: Some("claude".into()),
            model: Some("claude-opus-5".into()),
            reasoning: Some("high".into()),
            tier: Some("strong".into()),
            provisioning_required: true,
        }),
        ..RequestOutcome::default()
    };
    let reply = request_reply(&["coding".into(), "400000".into()], Some(&disclosed));
    assert_eq!(
        reply.plain,
        "Joined automatically as coding — 400000 tokens, served by claude-agent running claude · claude-opus-5 (high) · strong."
    );
    // `serving: null` degrades to the agent alone, never a fabricated config.
    let unknown = RequestOutcome {
        engagement: Some(base),
        serving: None,
        ..RequestOutcome::default()
    };
    let reply = request_reply(&["coding".into(), "400000".into()], Some(&unknown));
    assert_eq!(
        reply.plain,
        "Joined automatically as coding — 400000 tokens, served by claude-agent."
    );
    assert!(!reply.plain.contains("running"));
    assert!(!reply.plain.contains("HAGENCY_"));
}

/// The dispatch half: a parsed `!request` line is authorized and handed to the
/// caller with its arguments (TS `:365`).
#[test]
fn ts_oracle_request_dispatch_hands_args_to_caller() {
    let acl = Acl::default();
    let observed = HostObservation::default();
    assert_eq!(
        dispatch("!request coding 400000 20000", "@a:example.test", &acl, false, &observed),
        Dispatched::Request(vec![
            "coding".to_owned(),
            "400000".to_owned(),
            "20000".to_owned()
        ])
    );
    assert_eq!(
        json!(true),
        json!(matches!(
            dispatch("!request coding 400000", "@a:example.test", &acl, false, &observed),
            Dispatched::Request(_)
        ))
    );
}

#[test]
#[ignore = "parity gap: no native requester-token submit credential (no /api/engagements POST)"]
fn ts_oracle_requester_token_preferred_over_operator() {
    // TS asserts: HAGENCY_REQUESTER_TOKEN is preferred over API_TOKEN for the
    // submit. Native has no HTTP submit path to carry either token; admission
    // is Matrix-event-verified instead.
    let _ = ();
}
