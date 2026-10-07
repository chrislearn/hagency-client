spec: task
name: "Owner Agent execution-device ledger continuity"
inherits: project
tags: [active, rust, security]
---

## Execution history and accounting evidence

The current owner client checks the Agent's entire started execution history before acquiring its device lease. Server history contains immutable execution metadata and no message body, model credential or charge. Every page is bounded to 128 records and belongs to one snapshot. Acquire atomically checks that snapshot; the client checks again before enabling new model calls. Failed, truncated or changed history is never treated as empty.

- Only a genuinely empty remote history permits a fresh local ledger. Local prepared entries that never reached remote start may be extra entries.
  Test: continuity_empty_history_allows_fresh_but_remote_start_requires_local_charge
  Test: history_fresh_zero_full_settled_and_missing_cost_gate_real_local_ledger
- Complete unknown reservations stay held and continue to block new calls in that binding. Settled usage and all three accounting layers survive restart; reset or missing balances do not qualify.
  Test: continuity_complete_unknown_hold_and_settled_cost_survive_restart_but_reset_balance_fails
  Test: continuity_scope_digest_and_compacted_reply_require_exact_original_cost_proof
- A normal rejection may atomically seal explicit evidence that no provider call began. A pre-reserve backup cannot synthesize zero cost, and existing unknown charges cannot be overwritten.
  Test: continuity_rejection_seals_only_proven_no_call_and_preserves_existing_unknown_charge
- Pagination and post-acquire races fail closed.
  Test: history_changed_between_pages_after_acquire_and_missing_page_are_never_empty
  Test: history_wrong_witness_digest_is_not_an_empty_or_complete_ledger
  Test: fresh_history_acquire_sends_snapshot_and_post_check_keeps_models_allowed
- A missing ledger without an explicitly requested, locally known reply recovery never acquires or disrupts another device. Recovery with known original output performs no poll, ACK or inference.
  Test: incomplete_ledger_never_acquires_without_explicit_known_reply_recovery
  Test: recovery_required_reconciles_known_reply_without_poll_ack_or_model
  Test: binding_selection_filters_before_limit_for_known_reply_and_unknown_backlogs

## Product boundary

`ledger_recovery_required` requires the same owner's complete current-schema ledger. Estimated mode, takeover, administrator consent or another owner's ledger cannot bypass accounting evidence. There is no old Fleet import, Agent ownership transfer or server budget approval.

The browser gate in `check-owner-console.mjs` checks the runtime failure, HTTP 409 error and known-reply-only state. It displays recovery instructions without claiming readiness for new model work. These tests use protocol peers and real SQLite; real PostgreSQL/API and Docker release evidence are recorded separately in the design report. They do not qualify paid provider inference.

## Final review regression coverage

  Test: terminal_rejections_free_payload_and_active_slots_without_losing_started_witnesses
  Test: only_settled_failed_cost_can_compact_unknown_and_unsent_results_stay_retained
