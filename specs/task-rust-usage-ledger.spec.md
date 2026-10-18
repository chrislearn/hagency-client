spec: task
name: "Persist host-attributed native usage observations without inventing measurement authority"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION, REQ-CONTRIBUTION-CONSOLE]
tags: [active, rust, metering, custody]
---

## Intent

Preserve bounded transcript observations across restart and source disappearance
without duplicate growth, guessed attribution or unknown-to-zero accounting.

## Constraints

### Must
- Initially bind one source to the exact current Started dispatch scope and derive historical attribution from that writer scope.
- Keep counts untrusted and separate from provider-authenticated measurement execution permission allocations and quota enforcement.
- Preserve optional latest counts full parser diagnostics and sticky historical incomplete evidence separately from high-water lower bounds.
- Apply per-kind high-water growth both UTC granularities and content-bound receipt in one writer transaction after queue and lock.
- Preserve existing source identities and receipts at finite global per-source and per-engagement capacity limits.
- Check all known subtotals and aggregates even when another contributing field is unknown.
- Keep source regression visible and refuse backward or out-of-range writer clocks without changing period attribution.
- Permit identical historical receipt replay after restart retirement and capacity refusal without renewing execution authority.

### Must Not
- Do not derive attribution from transcript path workspace model display name or runtime-supplied Agent metadata.
- Do not infer source authenticity from completed tasks matching paths or a claimed source identifier.
- Do not add HTTP runner source setters automatic archival eviction quota decisions or live transcript discovery.
- Do not turn missing or rejected evidence into zero or count a reappearing source twice.

## Boundaries

### Allowed Changes
- crates/hagency-metering/Cargo.toml
- crates/hagency-metering/src/lib.rs
- crates/hagency-metering/src/observation.rs
- crates/hagency-metering/tests/observations.rs
- crates/hagency-store/Cargo.toml
- crates/hagency-store/src/domain.rs
- crates/hagency-store/src/domain/usage.rs
- crates/hagency-store/src/domain/usage/**
- crates/hagency-store/src/domain_worker.rs
- crates/hagency-store/src/lib.rs
- crates/hagency-store/src/migrations/017-usage-ledger.sql
- crates/hagency-store/tests/usage.rs
- crates/hagency-store/tests/usage/**
- crates/hagency-store/tests/common/mod.rs
- crates/hagency-store/tests/approvals.rs
- crates/hagency-store/tests/conversations.rs
- crates/hagency-store/tests/replies.rs
- crates/hagency-store/tests/verified_ingress/notice_custody.rs
- crates/hagency-store/tests/workflows/custody.rs
- crates/hagency-store/tests/workflows/mod.rs
- crates/hagency-store/tests/owned_completion.rs
- ./Cargo.lock
- tools/native/usage-vectors.mjs
- crates/hagency-store/tests/fixtures/usage-vectors.json
- .github/workflows/rust.yml
- specs/task-rust-usage-ledger.spec.md
- knowledge/decisions/adr-063-native-usage-ledger.md
- docs/agent-knowledge.md
- docs/progress.md

## Acceptance Criteria

<!-- lint-ack: bdd-rule-grouping — Each fixture exercises an independent boundary of this bounded repository slice. -->

Scenario: Source attribution is exact host history
  Test: native_usage_source_authority
  Given exact current Started and foreign or unstarted scopes
  When the host binds restores or replays a usage source
  Then only the original immutable execution attribution is admitted and source data cannot choose an Agent

Scenario: Normalized evidence remains incomplete when unknown
  Test: native_usage_observation_evidence
  Given missing malformed regressing or explicitly zero parser observations
  When bounded snapshots are normalized and recorded
  Then optional counts full diagnostics and incomplete history survive without authenticated measurement claims

Scenario: Growth and UTC buckets match retained JavaScript arithmetic
  Test: native_usage_high_water_vectors
  Given exact positive JavaScript-derived ledger vectors
  When repeated cumulative snapshots span UTC day and month boundaries
  Then only per-kind growth is credited and both granularities match the oracle

Scenario: Source disappearance and restart do not erase identity
  Test: native_usage_restart_and_reappearance
  Given recorded usage and its immutable execution source
  When source bytes disappear reappear or change after writer restart and retirement
  Then historical marks remain and only new growth is credited with exact receipt replay

Scenario: Capacity and overflow refusal are atomic
  Test: native_usage_capacity_and_rollback
  Given exhausted finite row limits or overflowing known subtotals
  When another observation or source is attempted
  Then existing source receipts marks and periods survive and unknown fields cannot hide overflow

Scenario: Observation receipts fence changed bodies and clocks
  Test: native_usage_receipts_and_clock
  Given exact and changed duplicate calls rejected parse results and backward clocks
  When observations cross the writer transaction boundary
  Then identical replay returns original evidence and changed or unsupported commands cannot rebucket growth

Scenario: Writer timing and lost receipts preserve exact observation identity
  Test: native_usage_worker
  Given a real writer queue SQLite lock and an expired caller response
  When usage is committed or its command never begins
  Then observation time follows the acquired lock and same-call replay counts the snapshot exactly once

Scenario: Usage migration does not invent observations
  Test: native_usage_migration
  Given a valid older database or incomplete current schema
  When schema seventeen opens
  Then valid history remains and missing structure fails closed without fabricated sources

## Out of Scope

Actual descriptor-to-process association, filesystem discovery, transcript scan,
live provider or Matrix services, service integration, quota enforcement,
provider-authenticated metering, monetary billing and automatic retention pruning.
