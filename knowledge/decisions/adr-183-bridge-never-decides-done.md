---
kind: decision
id: ADR-183
title: "Matrix and approval faults are contained and recoverable: retry before fence, trust the anchor not the device list, a fenced room heals on a good observation"
status: Proposed
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [native, matrix, approval, fleet, containment, recovery]
---

## Context

The containment slice's binary ran live on 2026-09-23 (instance
`local-native-palpo-live-20260923T063338Z`). The fleet did what ADR-182 says
for its own faults, and then died of three faults ADR-182 left to this slice —
the review's G5 ("fleet SPOFs and the persisted Matrix fence"):

1. **13:52:39 UTC — one Matrix refresh timeout ended all three workers.**
   `run_continuous` returns `Failure::Refresh` on any refresh error
   (`driver.rs:513-521`); the owner loop records it as the agent's failure
   (`driver.rs:186`), the fleet reads `failed`, readiness is 503, and nothing
   retries. The endpoint answered again 61 ms later. The retained product
   retries its sync with backoff and marks nothing.
2. **The owner logging in on a second Matrix client refused the next startup.**
   The approval enrollment keeps a byte-for-byte copy of the owner's
   `/keys/query` response and refuses `Recipients` when any of `device_keys`,
   `master_keys`, `self_signing_keys`, `user_signing_keys` differs
   (`sdk/enrollment.rs:603-618`), and its device acceptance refuses the whole
   set when any one device is unverified (`sdk/keys.rs:80-84`). The port
   already pins the owner's cross-signing master key (`check_anchors`,
   `sdk/enrollment.rs:809-833`) — the retained product's trust anchor — and
   then adds the stricter list equality on top. A new Robrix login is an
   unverified device: outage.
3. **That refusal fenced the approval room, and nothing can unfence it.**
   `fence_approval_candidates` sets `approval_rooms.available=0`
   (`approvals.rs:1011-1031`); a later good observation at the same
   generation is refused (`approvals.rs:237-250`: "restoration still needs a
   new generation"); no operator command produces a new generation; every
   restart refuses `Generation`. The operator reversed exactly this rule for
   the close-time fence on 2026-09-21 (ADR-047 amendment: "a same-generation
   positive observation cannot restore an unavailable generation, and so the
   only way back was an operator editing the generation") — the recipients
   fence kept it.

The operator asked on 2026-09-23 for all three to be fixed, and stated the
rule that governs them: "hagency is just a bridge between Codex and the
homeserver; Rust code cannot decide 'we are done' — only the user in the
homeserver or Codex can."

## Principle

The bridge never ends an agent or the service on its own. Every fault the
bridge sees is exactly one of two things: **transient** — retried with
backoff, forever, shown as a state word and recorded per attempt (ADR-181);
or **needs a human** — parked with the reason, shown in status and readiness,
cleared by the human's act or by the fact going away. There is no third
class. The only exits are the operator's SIGTERM and Codex's own turn
outcomes. The ADR-182 agent fence is the one allowed bridge-side stop, because
it parks and the operator's settlement clears it.

## Decision

0. **No component refusal exits the process.** `serve` starts every
   component it can and runs; a component that refuses (the approval SDK, a
   transport) is shown as refusing in status and readiness with its reason,
   and is retried on the transient class or parked on the human class. The
   process exits on SIGTERM only. (`Error: Startup` after "approval startup
   refused" was seen live tonight, twice.)


A. **A Matrix refresh failure is retried, not fatal.** `run_continuous`
   treats `Failure::Refresh` (and the `Startup` of the SDK enrollment when
   it is a transport refusal) like a refused handoff: record it in the
   status (`matrix_error`, `refresh_failures`, `refresh_since_ms`), keep the
   worker, and retry with the retained product's backoff: 1 s doubling to a
   60 s cap, reset to 1 s by the first success (`lib/appservice-sync.js:236-237,
   :46, :297`); the SDK's own sync loop retries forever at 5–15 s
   (`matrix-bot-sdk/lib/MatrixClient.js:619-632`), and only 401/403 are
   non-retryable (`bridge-matrix.js:1647-1649`, "a network failure is NOT a
   dead credential"). The agent is `refresh_refused`, a
   live state word: the fleet is not failed, readiness reports the component
   as refusing (503 while it lasts, 200 when a refresh succeeds), and the
   failures are counted in the status and named in the log (a refresh
   precedes any claim, so there is no attempt row to hang them on). Nothing
   is fenced by a refresh failure. The only fatal Matrix outcomes remain the ones that are evidence
   about the transport itself, and even those **park, they do not end**:
   `Identity` (whoami names another account) and `Generation` (the store
   retired this transport) park the worker as `awaiting_operator` with the
   reason and re-check on each pass; `OutcomeUnknown` on a write whose
   response was lost keeps ADR-064's uncertainty word and parks the same way.

B. **Recipients are the owner's devices signed by the pinned anchor; the
   list may change.** Keep `check_anchors` (the master key pinned at
   provisioning is the trust anchor, ADR-137). Replace the byte-equality of
   the recorded `/keys/query` with: for each recipient user the fresh
   response must carry the same master and self-signing keys as the SDK's
   accepted identity (already checked, `keys.rs:57-64`); every device that
   is cross-signed by that identity is a recipient; a device that is not is
   **excluded** and counted (`unverified_devices`), never a refusal; a
   recipient set with no verified device for the owner refuses the *card*
   (`Recipients`, fail-closed as ADR-137 says) and fences nothing. New
   verified devices join the next card's recipient set without a restart.
   The status shows `recipients` and `unverified_devices` per approval
   room. The retained product pins nothing and encrypts to every joined
   device with a fresh `/keys/query` per send (`RustEngine.js:64-105`,
   `onlyAllowTrustedDevices` never set); the operator chose on 2026-09-23 to
   keep the port's anchor and drop only the list equality (option 2 of two):
   a card that names commands and files is not readable by a device the
   owner has not verified, and an unverified device is not an outage.

B-1. **The card's own recipient proof is the recorded set, not the device
   list.** The private card's attempt record is validated before every write
   by comparing the Olm messages actually sent against the devices in the
   `/keys/query` response (`approval_delivery/state.rs`, "actual_recipients
   != expected_recipients" → `Storage`). That equality IS the device-list
   rule again, one layer down: with an unsigned device excluded (B), the
   actual set is a strict subset and every card refuses. The attempt record
   therefore carries the recipient set the SDK decided on at encryption
   time, and the validator compares the sent messages against THAT set,
   with the record's set required to be a subset of the response's devices.
   The proof a card went only where it was meant to is unchanged; what it is
   measured against stops being "every device the server listed".

C. **A fenced approval room heals on a good observation.** `available=0`
   stays the fence word. `observe_approval_room` at the same generation with
   `available: true` restores the row when the fresh snapshot (joined,
   invite_only, encrypted) equals the snapshot the fence was written over —
   the same rule the operator chose for the transport on 2026-09-21. A
   changed snapshot at the same generation is still `Conflict` (negative
   evidence), and a new generation still works as before. The startup
   refresh therefore unfences a room whose state is intact, and an operator
   sees in the status why a room is fenced (`fenced_reason`, from the
   failure that fenced it) while it lasts. No console route is needed for
   the healthy case; a room whose state really changed is the next
   provisioning's matter, as today. The retained product has no room fence:
   a failed delivery denies that request only (`approval-store.js:681-697`),
   "observing membership asserts nothing about permission … an unreachable
   room must not look like a withdrawn binding" (`:264-290`), and a binding
   is reactivated by the next good observation (`:315`, on owner rejoin and
   on every bridge start, `bridge-matrix.js:4376`). Decision C therefore
   also removes the fence *write* on a recipient or delivery failure: the
   card is refused, the room row is untouched.

D. **The execution budget notifies; it does not cut.** Operator decision,
   2026-09-23: "make it notify-only and cut only when the turn is done."
   When `operation_ms` elapses the attempt is not killed: the status reads
   `over_budget` with the elapsed time, the thread gets one notice ("still
   running after N minutes; say stop to end it"), an attempt event is
   recorded (ADR-181), and the turn continues until Codex ends it or a human
   stops it — the owner's stop through Matrix, or the operator's SIGTERM.
   The retained product's `executionTimeoutMs` SIGTERM (`router/src/runner.ts:362`)
   is deliberately not ported: only the user in the homeserver or Codex
   decides. The same rule retires the warm runtime's re-qualification kill
   (`lost_authority` every 100 ms while idle): a failed local check is
   recorded and re-checked, never a stop. The human stop paths this relies
   on: the console `agents/{id}/stop` route (exists, `console/agents.rs:31`,
   store `stop_dispatch_for_agent`) and the retained product's
   `POST /api/agents/:name/stop` (`backend-v2.js:12708`); whether the
   retained product also takes an owner "stop" in the thread is checked
   during the build and ported if so.

## Amends
- ADR-047 (fence on incomplete collection): a refresh failure retries before
  it fences; the close-time reversal of 2026-09-21 now applies to the
  approval room too.
- ADR-137/138: the recipient rule is the anchor plus cross-signed devices,
  not the recorded list; `unverified_devices` is a status word.
- ADR-174: the transport redial stays; above it, the worker-level retry.
- ADR-182: `Failure::Refresh` leaves the list of worker-ending failures.

## Scenarios

Bound in `specs/task-rust-bridge-never-decides.spec.md`.
