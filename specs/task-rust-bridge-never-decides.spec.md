---
spec: task
name: "The bridge never decides done: retry before fence, trust the anchor, heal the room, notify on budget"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [active, rust, matrix, approval, fleet, containment]
---

## Intent

Third slice of the closing order (review gap G5, and the operator's rule of
2026-09-23: hagency is a bridge; only the user in the homeserver or Codex
decides that anything is done). Decided in ADR-183: no component refusal
exits the process; a Matrix refresh failure is retried, not fatal; recipients
are the owner's devices signed by the pinned anchor, and the list may change;
a fenced approval room heals on a good observation and a delivery failure
fences nothing; the execution budget notifies and never cuts; the warm
runtime's idle re-qualification records and never stops.

## Constraints

- The retained product's retry shape: 1 s doubling to a 60 s cap, reset by
  the first success; only an authentication rejection (401/403, whoami
  naming another account) or a retired transport generation is not retried,
  and those park the worker with the reason instead of ending it.
- The owner's cross-signing master key pinned at provisioning stays the trust
  anchor (ADR-137); no card is encrypted to a device that key has not signed;
  an unsigned device is excluded and counted, never a refusal; a recipient
  set with no verified device refuses that card only.
- `approval_rooms.available=0` is written only by negative evidence about the
  room's state, never by a delivery or recipient failure; a good observation
  at the same generation with the same snapshot restores it.
- `operation_ms` elapsing kills nothing: status `over_budget`, one thread
  notice, one attempt event; the turn ends when Codex ends it or a human
  stops it (console `agents/{id}/stop`, SIGTERM).
- The warm runtime's idle re-qualification never stops the child; a failed
  check is recorded and re-checked.
- ADR-181's evidence is recorded on every path this slice changes; the
  status words are fixed labels, never free text.

## Allowed changes

- native/hagency/src/bootstrap.rs
- native/hagency/src/bootstrap/driver.rs
- native/hagency/src/bootstrap/fleet.rs
- native/hagency/src/bootstrap/approval.rs
- native/hagency/src/lib.rs
- native/hagency/src/main.rs
- native/hagency/tests/**
- native/hagency-matrix/src/approval_delivery.rs
- native/hagency-matrix/src/approval_delivery/**
- native/hagency-matrix/src/approval_intake.rs
- native/hagency-matrix/src/enrollment.rs
- native/hagency-matrix/src/sdk/enrollment.rs
- native/hagency-matrix/src/sdk/keys.rs
- native/hagency-matrix/src/sdk/encrypted_message.rs (decision B: an excluded device's withheld share is skipped, not refused)
- native/hagency-matrix/src/sdk.rs (decision B: the enrollment poison applies only to a ledger left mid-mutation)
- native/hagency-matrix/src/collector.rs (decision A: a refused read fences nothing)
- specs/task-rust-matrix-approval-cleanup-observation.spec.md (decision C: the identity-refusal scenario's wording and binding)
- native/hagency-matrix/src/lib.rs
- native/hagency-matrix/tests/**
- native/hagency-store/src/domain/approvals.rs
- native/hagency-store/src/domain/attempt_events.rs
- native/hagency-store/src/domain_worker.rs
- native/hagency-store/src/lib.rs
- native/hagency-store/src/domain.rs (decision D only: the migration registry line and version 39)
- native/hagency-store/src/migrations/039-attempt-over-budget.sql (decision D: the `over_budget` phase widens the 037 CHECK, which SQLite cannot alter in place)
- native/hagency-store/tests/**
- native/hagency-execution/src/operation.rs
- native/hagency-execution/src/warm.rs
- native/hagency-execution/src/factory.rs (decision D, warm rule only: the `idle_status` accessor the host projects)
- native/hagency-execution/src/lib.rs
- native/hagency-execution/tests/**
- knowledge/decisions/adr-183-bridge-never-decides-done.md
- knowledge/decisions/adr-047-native-matrix-transport.md
- knowledge/decisions/adr-137-private-approval-send-fail-closed.md
- knowledge/decisions/adr-138-bounded-native-approval-observation.md
- knowledge/decisions/adr-174-matrix-read-rate-limit.md
- knowledge/decisions/adr-182-one-fact-one-blast-radius.md
- specs/task-rust-bridge-never-decides.spec.md
- docs/**

## Scenarios

Scenario: A Matrix refresh failure is retried and the worker stays up
  Retired-Test: native_refresh_failure_is_retried_not_fatal
  Production caller: hagency::bootstrap::driver::run_continuous
  Given a continuous worker whose homeserver stops answering its refresh
  When the refresh fails
  Then the worker is still up, its status reads refresh_refused with the failure count, the fleet is not failed and readiness is 503
  And when the homeserver answers again the next attempt runs, the status clears and readiness is 200

Scenario: A refresh retry follows the retained product's backoff
  Retired-Test: native_refresh_retry_backoff
  Production caller: hagency::bootstrap::driver::run_continuous
  Given a worker whose refresh keeps failing
  When it retries
  Then the pauses double from 1 s to a 60 s cap and the first success resets them

Scenario: An authentication rejection parks the worker with the reason
  Retired-Test: native_refresh_identity_rejection_parks
  Production caller: hagency::bootstrap::driver::run_continuous
  Given a worker whose whoami names another account
  When the refresh fails
  Then the worker parks as awaiting_operator with matrix_error identity, re-checks on each pass, and the process keeps serving

Scenario: A new device signed by the anchor joins the recipients
  Test: native_new_signed_device_joins_recipients
  Production caller: hagency_matrix::approval_delivery::ApprovalCollector::send_private_approval_card
  Given an approval enrollment completed with one verified owner device
  When the owner adds a second device cross-signed by the pinned identity and a card is sent
  Then the card is encrypted to both devices without a restart

Scenario: An unsigned device is excluded, not fatal
  Test: native_unsigned_device_is_excluded_not_fatal
  Production caller: hagency::bootstrap::approval::Pump::initialize
  Given an approval enrollment completed with one verified owner device
  When the owner adds a device the pinned identity has not signed
  Then the enrollment still completes, the card goes to the verified device only, the status counts unverified_devices=1, and nothing is fenced

Scenario: No verified device refuses the card only
  Test: native_no_verified_device_refuses_the_card_only
  Production caller: hagency_matrix::approval_delivery::ApprovalCollector::send_private_approval_card
  Given an owner whose only devices are unsigned
  When a card is sent
  Then the card is refused with Recipients and the request is denied fail-closed, the approval room stays available, and the worker continues

Scenario: A changed anchor is still refused
  Test: native_changed_anchor_is_refused
  Production caller: hagency::bootstrap::approval::Pump::initialize
  Given an owner whose cross-signing master key differs from the pinned one
  When the enrollment verifies
  Then it refuses with Recipients, as ADR-137 requires

Scenario: A fenced approval room heals on a good observation
  Test: native_fenced_approval_room_heals_on_a_good_observation
  Production caller: hagency_store::domain_worker::DomainStore::observe_approval_room
  Given an approval room fenced by an earlier refusal
  When the room is observed again at the same generation with the same members, privacy and encryption
  Then the row is available again and the next startup admits it
  And an observation whose snapshot differs at the same generation stays fenced with Conflict

Scenario: A delivery or recipient failure fences nothing
  Test: native_delivery_failure_fences_nothing
  Production caller: hagency_matrix::approval_delivery::ApprovalCollector::send_private_approval_card
  Given an approval room whose send fails
  When the card is refused
  Then the request is denied fail-closed, the room row is untouched, and the next card is attempted

Scenario: The execution budget notifies and the turn continues
  Test: native_budget_expiry_notifies_and_the_turn_continues
  Production caller: hagency::bootstrap::driver::run
  Given an operation whose budget elapses while Codex is still working
  When the budget elapses
  Then nothing is killed, the status reads over_budget with the elapsed time, one notice is queued for the thread, one attempt event is recorded, and the turn completes when Codex ends it

Scenario: The over-budget notice is queued once for the dispatch's thread
  Test: native_over_budget_notice_queued_once
  Production caller: hagency::bootstrap::driver::run
  Given a started dispatch answering an addressed request in a verified thread session
  When the budget elapses and the host queues the notice, twice
  Then one pending task notice of kind over_budget exists in that thread with fixed words around the elapsed time, the second queue finds the first, the driver's notice delivery can claim it, and a dispatch with no verified thread queues nothing

Scenario: The idle re-qualification records and never stops
  Test: native_idle_qualification_failure_is_recorded_not_fatal
  Production caller: hagency_matrix::provisioning::factory::TokenProvisioningHost::finish_factory
  Given a warm child whose local check fails while idle
  When the check fails
  Then the child is not stopped, the failure is recorded, and once the check passes again the next dispatch is admitted to the same child

Scenario: A component refusal does not exit the process
  Retired-Test: native_component_refusal_does_not_exit_the_process
  Production caller: hagency::bootstrap::Bootstrap::serve
  Given a service whose approval SDK refuses at startup
  When the service starts
  Then the process keeps serving, readiness is 503 naming the refusing component and its reason, the capabilities carry the reason, and when the refusal clears the component is admitted without a restart

## Out of scope

Configurable ceiling period and daily rate cap (the next small slice); the
activity notice (its own slice); the owner-facing fence card in the DM; the
approval leg's environmentId proof (G4); authority decoupling (G3).

## Owner bootstrap product replacement (2026-10-07)

The `Retired-Test` selectors above depended on removed client-side Fleet/provisioning/encrypted approval-bot production switches. They are no longer executable product requirements. New `native-owner-client.spec.md` binds actual bootstrap secret/config refusal, anonymous provider/model/tool denial, fresh Pasion authorization after restart and independent Room runtimes. It does not claim encrypted private approval, factory or delegated-task behavior. Direct SDK configuration tests remain bound by `native-bootstrap-sdk.spec.md`, and all other SDK Test/Filter bindings remain active.
