spec: task
name: "Native console workflow for inspected stopped tasks"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION, REQ-LOCAL-THREE-RUNNER-QUALIFICATION]
tags: [active, rust, console, recovery]
---

> Desktop-only scope (2026-10-20): CLI and Web page behavior below is historical.
> Shared engine selectors remain active unless explicitly marked Retired-Test.
> See [the desktop contract](native-owner-client.spec.md) for current product entry points.

## Intent

Expose the existing ADR165 inspection and three explicit operator actions in the
native console, with bounded discovery of stopped dispatches per engagement.

## Constraints

- Keep the existing seven-field roster unchanged. Separate stopped-work reads
  require lifecycle authority and return only sixteen scalar rows per page.
- Show absence of original inspection explicitly. A listed receipt is historical
  evidence, not a promise that current custody permits resolution.
- Bind inspection, replacement and result to the selected original dispatch.
  Display task, workspace, inventory hashes and expiry before any decision.
- Keep the one-use secret in memory only. Never render, log, persist or place it
  in URLs. Request/body limits remain finite; only inspection permits up to 2 MiB.
- Require an operator note and a distinct instruction for continuation. Never
  issue mutations from effects or auto-retry a lost decision. Preserve the exact
  serialized pending decision for an explicit same-request retry on uncertainty.
- English and Chinese labels, accessible inputs and wrapped identifiers.
- Offline tests and real local browser fixtures; no live service from tests.

## Allowed changes

- crates/hagency-store/src/domain/stopped_inspection.rs
- crates/hagency-store/src/domain_worker.rs
- crates/hagency/src/console/agents.rs
- crates/hagency/tests/console/**
- tests/dashboard-native-recovery.test.js
- specs/task-rust-console-outcome-workflow.spec.md
- specs/task-console-outcome-protocol.spec.md
- knowledge/decisions/adr-170-console-outcome-workflow.md
- docs/**

## Scenarios

The browser-side decision protocol is a JavaScript module with a Vitest
selector, so its scenario lives in `task-console-outcome-protocol.spec.md`,
which the Node bindings check resolves. This spec binds Rust selectors only.

Scenario: Stopped-work discovery remains private and bounded
  Test: native_console_stopped_dispatch_list
  Given stopped dispatches across separate engagements
  When an operator pages the selected engagement
  Then only sixteen bounded rows and an exact cursor are returned
  And read-only sessions, malformed queries and foreign inspection are refused

Scenario: Lifecycle operators can review and resolve through the native browser
  Retired-Test: native_console_agent_lifecycle_browser


## Owner-only browser qualification (2026-10-07)

The former browser qualification and Web console have been retired. Independent SDK HTTP/library selectors in this mixed contract remain active. Current desktop and OAuth qualification is defined in `native-owner-client.spec.md`; historical browser selectors are not passing evidence.
