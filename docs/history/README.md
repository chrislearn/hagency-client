# Historical documents

These files describe the TypeScript product (`backend-v2.js`, `bridge-matrix.js`, `lib/` and its Node services) that was removed in hagency-rs #17. They are kept for design history only and do not describe the current native service. For the current service see [the architecture walkthrough](../architecture-walkthrough.md) ([中文](../architecture-walkthrough.zh-CN.md)).

## Architecture

- [Agent Lifecycle Architecture](architecture/agent-lifecycle.md): agent types, provisioning, session and supervisor lifecycle, the task system, agent home layout and the removed subconscious hooks.
- [Agentchat Authentication & Trust Model](architecture/auth-trust-model.md): bearer, per-agent and bridge-secret tokens, trust levels and the Matrix bridge room-trust rules.
- [Group & Room System Architecture](architecture/group-room-system.md): how groups, DM rooms, SPY rooms and Matrix rooms were created, mapped and reconciled, with the API routes, cursors and mention delivery.
- [Agentchat Operational Patterns](architecture/ops-patterns.md): autodeploy, monitoring, the subconscious system, remote server management and runbooks.
- [Owner-scoped Matrix UI approval](architecture/owner-ui-approval.md): owner bindings, the `com.agentchat.approval.*` event contract, `data/approvals.json` and the Claude Code / Codex approval relays.
- [System Components Overview](architecture/system-components.md): the Node services, library modules, scripts, data flows and configuration model.

## Guides

- [项目方申请、Hagency 批准、Agent 完成任务：人工验证](guides/hagency-borrower-walkthrough.zh.md): the 2026-09-06 manual verification script for `!offer` / `!request` in a project room.
- [在 Palpo 定义 Agent，向 Hagency 申请资源](guides/resource-agents.zh.md): defining agents in Palpo against Hagency resource pools, through the TypeScript console and the reverse-tunnel callback.
