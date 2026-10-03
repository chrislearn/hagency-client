---
spec: task
name: "Guardian stop evidence on macOS"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [active, rust, execution, guardian, diagnostics, only-macos]
---

## Intent

The guardian half of `task-rust-owned-attempt-evidence.spec.md` (ADR-181) whose
tests run only on macOS (`native/hagency-platform/tests/stop_evidence.rs` is
`#![cfg(target_os = "macos")]`). It is split out so the spec-binding check defers
these selectors on other platforms instead of reporting them missing. Same
constraints as the parent spec: the evidence authorizes nothing and changes no
verdict.

## Allowed changes

- native/hagency-platform/**
- specs/task-rust-owned-attempt-evidence-macos.spec.md

## Scenarios

Scenario: The guardian names why a stop after the leader exited did not prove the tree gone
  Test: native_guardian_report_names_the_stop_refusal
  Given a leader that exits leaving one live descendant the stop budget cannot end
  When the guardian reports Stopped with whole_tree_stopped false
  Then the frame carries refusal live_descendants with that row's pid, parent pid and executable name
  And the host reads the guardian's exit status and records both with stop_reported

Scenario: The guardian's stderr reaches the host and nothing else
  Test: native_guardian_stderr_reaches_the_host
  Given a guardian whose stop refusal writes one diagnostic line
  When the host observes the stop
  Then the host's 4 KiB tail holds that line and the work's own stderr holds nothing of the guardian's
  And a guardian started without the pipe still reports exactly as before
