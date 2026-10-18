Projects, Matrix chat and locally executed Agents are available in this release.

- Create Projects and discussion Rooms, create owner-bound Agents and bind them to Rooms.
- Configure Codex, Claude Code or Octos, model/reasoning options, workspaces, budgets and exact-action approvals.
- Configure supported Agent resources on another assigned execution device; offline settings remain read-only.
- Control Codex live web search per Agent, enabled by default.
- Use the Hagency desktop branding and icons.
- Maintain Desktop and its local owner engine together in [hagency-org/hagency-client](https://github.com/hagency-org/hagency-client), with a single root Cargo.lock and no sibling source checkout.

The version is intentionally reset from the inherited 1.1.x series to 0.5.0.
Existing installations may need to install this version manually.

Agent encrypted-Room execution is deferred; ordinary encrypted Matrix chat remains available.
Windows ARM64, mobile store packages, automatic updates and install-and-launch verification on every OS are not included.
Provider runtimes and accounts must be installed/configured on the execution device.

[Agent setup](https://github.com/hagency-org/hagency-desktop/blob/main/docs/hagency-quickstart.md)
