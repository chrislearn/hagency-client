[English](architecture-walkthrough.md) | [中文](architecture-walkthrough.zh-CN.md)

# Desktop architecture walkthrough

The native desktop application is the product entry point. Its former CLI and
Web console have been removed. The retired Fleet/Engagement walkthrough remains
available in Git history; current setup starts with the
[desktop quick start](../crates/hagency-app/docs/hagency-quickstart.md).

## Login and account identity

The application login code lives in `crates/hagency-app/src/login/`. Matrix and
Pasion/OAuth establish the signed-in account. The desktop engine adapter in
`crates/hagency-app/src/hagency/backend.rs` passes that account and its token source
to `hagency::native_owner::open_with_matrix`.

The owner facade pins the Matrix account and validates server identity. Account
switching revokes the previous account's local grants. OAuth redirects return to
the callback handler on an ephemeral loopback port; that listener serves no
management pages or management APIs. See [server login](server-login.md).

## Desktop management and Agent execution

`crates/hagency/src/native_owner.rs` exports the shared owner facade. The underlying
`console/native.rs` dispatches authenticated requests to an in-process Salvo
service. `owner_host.rs` owns the private state directory and exclusive process
lock. No static assets or local Web management server are created.

The shared `console` modules implement owner Projects, Agent lifecycle, provider
settings, execution approvals and budgets. The execution, runtime, platform and
Matrix libraries retain their original authority and custody checks. Removing the
presentation layer does not replace these checks with desktop-only shortcuts.

## Files and build entry points

Use the [workspace map](workspace.md) and [shared engine map](native-engine.md).
Windows builds run through `tools/dev.ps1`, which places outputs, Cargo caches and
temporary files under `D:/BuildCache/hagency-client` and verifies Cargo metadata.
Desktop packaging and releases are defined in the `app-*` GitHub workflows.

Historical design records, parity vectors and attribution retain their original
provenance. They are evidence of previous implementations, not current installation
or browser-console instructions.
