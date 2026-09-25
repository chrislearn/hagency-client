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
/// (bridge-matrix.js:2940). Native has no join-backfill window selector.

#[test]
#[ignore = "parity gap: no native join-backfill selector (bridge pendingJoinBackfill)"]
fn ts_oracle_join_backfill_timeline_order_and_window() {
    // TS asserts: an older-in-timeline command with a NEWER timestamp is not
    // admitted; commands sharing a millisecond keep timeline order; a command
    // between invite and join is delivered; after-join is left to sync; no-join
    // page = invite..end window.
    let _ = ();
}

#[test]
#[ignore = "parity gap: no native join-backfill selector (bridge pendingJoinBackfill)"]
fn ts_oracle_join_backfill_fails_closed_and_filters() {
    // TS asserts: no provable invite -> boundary 'unproven' and NOTHING routed;
    // an earlier membership cycle is not replayed on re-invite; a foreign invite
    // is not the bot's; a JOIN is not an invite; the bot's own messages are never
    // fed back; seen events skipped; non-message/no-id events ignored; a missing
    // or malformed page yields nothing rather than throwing.
    let _ = ();
}

/// TS `tests/bot-commands-request.test.js` — `cmdRequest` submit path and the
/// requester credential. Native has no `cmdRequest` renderer (bot_commands.rs
/// `dispatch` returns `Unrenderable` for `!request`), and no HTTP
/// `/api/engagements` submit with a requester-token preference: admission is
/// Matrix-event-verified (`authority.rs` `verify_request`), not an agent-token
/// POST. The request-id = authenticated-event-id half IS enforced natively at
/// `verify_request` (`source.event_id == request.source_event_id`, authority.rs:222).

#[test]
#[ignore = "parity gap: no native cmdRequest submit (requestId/requester body) renderer"]
fn ts_oracle_request_id_is_event_id_and_requester_is_sender() {
    // TS asserts: requestId = the authenticated event id (never an argument);
    // with no event id NO requestId is sent; roomId from the room; requester from
    // the authenticated sender. Native enforces these at verify_request, not in a
    // cmdRequest renderer (bot_commands.rs dispatch -> Unrenderable for !request).
    let _ = ();
}

#[test]
#[ignore = "parity gap: no native requester-token submit credential (no /api/engagements POST)"]
fn ts_oracle_requester_token_preferred_over_operator() {
    // TS asserts: HAGENCY_REQUESTER_TOKEN is preferred over API_TOKEN for the
    // submit. Native has no HTTP submit path to carry either token.
    let _ = ();
}

#[test]
#[ignore = "parity gap: no native cmdRequest reply renderer (framework/model/reasoning, malformed-token, pending-why, room-name)"]
fn ts_oracle_request_reply_tells_what_the_project_got() {
    // TS asserts: the reply names framework/model/reasoning; a failure does NOT
    // leak the provider config (HAGENCY_*); unknown config degrades to the agent;
    // a malformed token amount is refused before the backend; a pending request
    // is told WHY; the room NAME labels the engagement. Native has no cmdRequest
    // reply path (the offer() renderer covers the offer-side disclosure only).
    let _ = ();
}
