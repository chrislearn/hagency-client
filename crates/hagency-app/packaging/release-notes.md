Projects, Matrix chat and locally executed Agents are available in this release.

- Create Projects and discussion Rooms, create owner-bound Agents and bind them to Rooms.
- Configure Codex, Claude Code or Octos, model/reasoning options, workspaces, budgets and exact-action approvals.
- Configure supported Agent resources on another assigned execution device; offline settings remain read-only.
- Control Codex live web search per Agent, enabled by default.
- Use the Hagency desktop branding and icons.
- Maintain the cross-platform Hagency app and its local owner engine together in [hagency-org/hagency-client](https://github.com/hagency-org/hagency-client), with a single root Cargo.lock and no sibling source checkout.

The client version is 0.6.0, following consolidation of the native app and the local engine.
Existing installations may need to install this version manually.

Agent encrypted-Room execution is deferred; ordinary encrypted Matrix chat remains available.
Windows ARM64, mobile store packages, automatic updates and install-and-launch verification on every OS are not included.
Provider runtimes and accounts must be installed/configured on the execution device.

[Agent setup](https://github.com/hagency-org/hagency-client/blob/main/crates/hagency-app/docs/hagency-quickstart.md)

Dependency audit follow-up remains open for the embedded and bundled Octos runtime,
including PDF/XML parsing. See the [recorded assessment](https://github.com/hagency-org/hagency-client/blob/main/dev-vibe/desktop/release-0.5.0/assessment.zh.md)
for scope and evidence; this release does not claim a clean dependency audit.
