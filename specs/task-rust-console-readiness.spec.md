spec: task
name: "Serve the console readiness and version strip from the existing boundaries"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [retired, rust, console, readiness, version]
---

> Desktop-only scope (2026-10-20): CLI and Web page behavior below is historical.
> Shared engine selectors remain active unless explicitly marked Retired-Test.
> See [the desktop contract](native-owner-client.spec.md) for current product entry points.

## Intent

Bind ADR-145: a readiness and version strip on every native console page, reading
the existing unauthenticated `GET /ready` as-is (never `/health`, never a console
route, never proxied) and two build-time constants from a generated module. No
Rust file changes; the readiness contract (ADR-096's brief-19/21 amendments)
stays frozen.

## Constraints

### Must
- Read `GET /ready` from the page same-origin and consume the payload as-is — exactly `{"status","implementation","components[].{name,state}}`, no new wire keys.
- Generate `status-constants.js` in the staged tree before `next build`, carrying `HAGENCY_NATIVE_VERSION` (parsed from the root Cargo.toml's `[workspace.package]`) and `HAGENCY_NATIVE_SCHEMA_HEAD` (a `--schema-head` argument copied from the migration registry). The staged tree is the build's `mkdtemp` workspace — **no repo path for the generated file is committed or expected in Allowed Changes**; if Next excludes an unbundled generated module, the constants are appended to an existing staged `.js` instead (same values, same test).
- Render `status === "ok"` as ready; 503/`unavailable` as not ready with failing components' names and words — a failing component never renders ready; an unreachable `/ready` renders unknown, never ready.
- Render the sweep-tick cell's raw word unstyled by outcome — `refused_*` is a ready word (`lib.rs:207-210`); only the sweep's liveness participates in the colour.
- Render the strip as a `PageHead` child on every native console page.
- Validate the payload by exact key set and count only; unknown state words render as text, never error.

### Must Not
- Do not read `/health`, add any console route for readiness, or proxy `/ready` through `/console/api/*`.
- Do not add any field to `manifest.json` or touch `crates/hagency/src/console/assets.rs` — the constants travel as a bundled `.js`, and "no Rust change" holds by construction.
- Do not enumerate readiness state words in the client — one vocabulary, the server's.
- Do not add restart or stop controls — explicitly out of scope until the service wrapper exists.
- Do not change the readiness contract: `/health` 200-while-live, `/ready` the 503 boundary, the `ComponentState` vocabulary and its enumeration test, diagnostic-never-authority.

## Boundaries

### Allowed Changes
<!-- Two paths beyond the original fourteen: mockup/lib/i18n.js carries the
strip's en/zh words (the console renders every visible word through t()),
and mockup/app/resources/new/page.jsx mounts the strip at the fifth native
page's own PageHead sites — the one page-file exception to Forbidden. -->
- crates/hagency/tests/console/status_strip.rs
- crates/hagency/tests/console.rs
- specs/task-rust-console-readiness.spec.md
- knowledge/decisions/adr-145-console-readiness-version-strip.md
- docs/progress.md

### Forbidden
- Live services, credentials, deployed state.
- crates/hagency/src/** (no Rust source file — the strip's own rule); mockup/app/** page files; the manifest schema.

## Acceptance Criteria

Scenario: The strip renders not-ready on a 503 component
  Retired-Test: native_console_status_strip_503_renders_not_ready


Scenario: The strip renders unknown when /ready is unreachable
  Retired-Test: native_console_status_strip_unreachable_renders_unknown


Scenario: The built assets carry the workspace version after a rebuild
  Retired-Test: native_console_status_strip_version_matches_workspace


## Out of Scope

Restart and stop controls (the service wrapper's slice), any Rust change, the
retained console's pages, and the readiness payload itself (frozen by
ADR-096's amendments).

The former browser status strip and its release evidence are retired. Current desktop/OAuth qualification is defined in `native-owner-client.spec.md`.
