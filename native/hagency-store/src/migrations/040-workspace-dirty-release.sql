-- #27: workspace dirty-release observability and the per-agent execution policy.
--
-- Dirty release (TS parity: router/src/store.ts:3297 clearWorkspaceDirty). The
-- dirty flag itself already exists (003-task-dispatch.sql workspace_resources.dirty)
-- and blocks dispatch enqueue; this migration adds the two columns the release
-- semantics inspect: `dirty_generation` (the precondition token TS's
-- mark-resource-inspected checks, agent-ops.ts:1180) and `inspected_at`
-- (the release receipt TS writes on clear). The quarantine refusal stays in the
-- store method — a dispatch whose outcome is unknown must be inspected through
-- the stopped-dispatch inspection flow, never released by this route.
ALTER TABLE workspace_resources ADD COLUMN dirty_generation INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workspace_resources ADD COLUMN inspected_at INTEGER;

-- Per-agent execution policy (TS parity: backend-v2.js:10865-10885 GET/PUT
-- /api/agents/:name/execution-policy). TS keeps it on the agent record as
-- `agent.executionPolicy`; native's agent is the engagement, so the policy is
-- keyed by engagement. A missing row is the TS default, `{ yolo: false }`
-- (backend-v2.js:10867 reads `?.yolo === true`), never a stored zero.
CREATE TABLE agent_execution_policies (
 engagement_id TEXT PRIMARY KEY REFERENCES engagements(id),
 yolo INTEGER NOT NULL DEFAULT 0 CHECK(yolo IN (0,1)),
 updated_at INTEGER NOT NULL
) STRICT;
