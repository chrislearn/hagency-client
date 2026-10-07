spec: task
name: "Isolate owner profiles and stop prior execution before account switching"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [active, rust, accounts, security, only-macos, only-linux]
---

## Intent

One local OwnerHost can select different personal Matrix accounts and servers.
Every authenticated origin, Pasion issuer, subject and MXID tuple owns separate
ledger, inbox, provider home, Codex keyring locator and execution contexts. Agent
ownership is permanent and selection never transfers or imports another account.

## Constraints

- Derive directory and runtime keys from the complete verified identity.
- Pin the SQLite profile identity permanently, even if MXIDs happen to match.
- Leave old prototype directories untouched; do not adopt populated unscoped data.
- Preserve settled charges and unresolved holds when returning to a profile.
- Revoke capabilities before registry drain; reject an old queued registration
  after it obtains the lock, and drain/cancel pending provider login children.
- Pin all lease/tool/approval transport to its original identity. A worker must
  never use the newly selected profile's bearer, even when IDs collide.
- Keep account homes and credentials in place when switching; require owner
  login before starting again. Do not call a model during the tests.

## Acceptance Criteria

Scenario: A copied database cannot become another subject's ledger with the same MXID
  Test: profile_identity_survives_restore_and_rejects_same_mxid_foreign_profile_copy

Scenario: Switching away and back preserves spent budget while roots remain independent
  Test: multi_profile_directories_preserve_local_budget_and_reject_foreign_copy

Scenario: A revoked in-flight registration cannot reenter the drained runtime registry
  Test: multi_profile_revoked_inflight_registration_cannot_reenter_cleared_registry

Scenario: Switching cancels and kills pending provider children without deleting account homes
  Test: multi_profile_switch_stops_pending_provider_children_without_deleting_account_homes

Scenario: Credential homes and OS keyring locators differ by subject as well as MXID
  Test: dedicated_provider_paths_are_private_canonical_and_owner_scoped

Scenario: Pending approval cannot be consumed by a different subject or issuer with all other IDs unchanged
  Test: approval_registry_fences_owner_device_expiry_digest_and_single_consumption

## Final review regression coverage

  Test: simultaneous_fresh_profile_openers_share_one_initialized_schema
  Test: profile_marker_accepts_long_identity_and_rejects_oversized_metadata
  Test: profile_marker_rejects_symlink_and_nonprivate_file
