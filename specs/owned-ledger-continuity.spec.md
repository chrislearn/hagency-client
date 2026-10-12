spec: task
name: "Owner Agent execution-device ledger continuity"
inherits: project
tags: [active, rust, security]
---

## Execution history and accounting evidence

The current owner client checks the Agent's entire started execution history before acquiring its device lease. Server history contains immutable execution metadata and no message body, model credential or charge. Every page is bounded to 128 records and belongs to one snapshot. Acquire atomically checks that snapshot; the client checks again before enabling new model calls. Failed, truncated or changed history is never treated as empty.

- The operator's 2026-10-12 accounting rule supersedes the historical hold policy: missing usage or historical receipts count as zero and do not block new work. Server started-execution custody still prevents replay; all available local identity and accounting proofs must match.
  Test: continuity_missing_charge_is_zero_but_execution_identity_is_retained
  Test: history_missing_receipt_does_not_block_with_retained_execution_identity
  Test: missing_historical_receipt_allows_new_work_without_owner_reconciliation
- Unknown estimates are released as zero. Genuine late usage can replace provisional zero once. Settled usage and all three accounting layers survive restart; contradictory known balances do not qualify.
  Test: continuity_complete_unknown_hold_and_settled_cost_survive_restart_but_reset_balance_fails
  Test: continuity_scope_digest_and_compacted_reply_require_exact_original_cost_proof
- A normal rejection may atomically seal explicit evidence that no provider call began. Missing accounting counts as zero while original execution identity is retained; a rejection cannot overwrite an existing settlement.
  Test: continuity_rejection_seals_only_proven_no_call_and_preserves_existing_unknown_charge
- Pagination and post-acquire races fail closed.
  Test: history_changed_between_pages_after_acquire_and_missing_page_are_never_empty
  Test: history_wrong_witness_digest_is_not_an_empty_or_complete_ledger
  Test: fresh_history_acquire_sends_snapshot_and_post_check_keeps_models_allowed
- Contradictory available execution/accounting evidence requires explicit known-reply recovery. Missing historical receipts alone permit acquisition and new requests. Recovery of known original output performs no poll, ACK or inference.
  Test: conflicting_history_requires_explicit_known_reply_recovery
  Test: recovery_required_reconciles_known_reply_without_poll_ack_or_model
  Test: binding_selection_filters_before_limit_for_known_reply_and_unknown_backlogs

## Product boundary

`ledger_recovery_required` represents conflicting identity or available accounting evidence. Missing recorded usage is handled silently as zero without owner intervention. Estimated mode, takeover, administrator consent or another owner's ledger cannot bypass contradictory proofs. There is no old Fleet import, Agent ownership transfer or server budget approval.

The browser gate in `check-owner-console.mjs` checks the runtime failure, HTTP 409 error and known-reply-only state. It displays recovery instructions without claiming readiness for new model work. These tests use protocol peers and real SQLite; real PostgreSQL/API and Docker release evidence are recorded separately in the design report. They do not qualify paid provider inference.

## Final review regression coverage

  Test: terminal_rejections_free_payload_and_active_slots_without_losing_started_witnesses
  Test: only_settled_failed_cost_can_compact_unknown_and_unsent_results_stay_retained
