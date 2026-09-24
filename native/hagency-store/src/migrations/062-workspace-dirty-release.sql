-- #27: workspace dirty-release observability (TS parity: router/src/store.ts:3297
-- clearWorkspaceDirty). The dirty flag itself already exists (003-task-dispatch.sql
-- workspace_resources.dirty) and blocks dispatch enqueue; this migration adds the
-- two columns the release semantics inspect: `dirty_generation` (the precondition
-- token TS's mark-resource-inspected checks, agent-ops.ts:1179) and `inspected_at`
-- (the release receipt TS writes on clear). The quarantine refusal stays in the
-- store method — a dispatch whose outcome is unknown must be inspected through
-- the stopped-dispatch inspection flow, never released by this route.
ALTER TABLE workspace_resources ADD COLUMN dirty_generation INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workspace_resources ADD COLUMN inspected_at INTEGER;
