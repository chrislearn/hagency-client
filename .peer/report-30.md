# Report #30 — Restart: every agent re-attaches like TS

Branch `task/30` from `9775f997`. Status: **done** (acceptance proven by a new three-agent restart test in the configured_fleet harness; one harness bug found and fixed; no production-code change required).

## Step 1 — what is still true on HEAD (verified)

1. **Re-attach path exists and is wired.** `DomainRepository::open` → `inline_factory_engagements` (`domain/provision_runtime.rs:306`, `effects.state='complete' AND engagements.state='active'`) → fleet `reattach_known_agents` (`bootstrap/fleet.rs:338`) → `reattach_completed` (`hagency-matrix/src/provisioning.rs:212`) which uses `reattach_provision_scope` (Complete row, receipt-recomputed proof) — **not** the `claim_effect_for` pending-only gate. The earlier "factory agents blocked by provisioning-only claim" finding applies to *fresh* claims only; reattach no longer goes through it.
2. **Always-grants survive a restart.** `approvals::recover` (`domain/approvals.rs:175-184`, invoked `domain.rs:747` on every open) revokes only grants whose binding is no longer current or whose task is done; `mode='always'` with a live binding is untouched.
3. **Clean-stop vs kill -9 converge on the same durable state.** Both end with a process that never fenced anything (ADR-183); on the next `open`, the same recovery runs for both. One restart test parameterized over {TERM, KILL} exercises both acceptance clauses.
4. **Queued/leased/started reconciliation** lives at claim/expire time (`owned_claim.rs:898-975`: a `started` dispatch left by a killed host is reconciled to `outcome_unknown`, never re-claimed; queued rows untouched).

## TS behaviour (source of truth, quoted)

- `router/src/store.ts:3840-3849` (`reconcileOnStart`): "`A runner lease belongs to this backend process. After a restart no old runner identity is verifiable…` so every unstarted lease must be made runnable again" — `leased → queued`, runner_id/lease cleared, capabilities revoked, resource leases deleted.
- `store.ts:3850-3856`: `started`/`parked` → `settleUnknownInternal(row, 'backend_restart_unverifiable_runner')` → `outcome_unknown`; queued rows untouched.
- `store.ts:3857-3867`: claimed matrix/reply/notice outbox entries re-pended.
- `backend-v2.js:17385-17410` (`shutdown()`): graceful only — `stopServer()` "aborts every live runner, and waits until each dispatch had durably settled or been safely requeued". Nothing fenced on a clean stop.
- `bridge-matrix.js:1519+` (`ensureAgentAccount`): stored token first, whoami to validate, identity check refuses; network failure rethrown, never a dead credential; never re-registers an existing account. `~4288-4296`: one failing agent is skipped and logged, never fatal.

## Rust change (file:line)

**Test harness only — no production change was needed.** The store already has the correct behaviour (proved below).

- `native/hagency/tests/configured_fleet/mod.rs`
  - `Peer::new(..., count)` + `Fixture::three_agents()` (`:114`) — agent count parameterized 2→N; `three_agents` runs the local-Codex profile so the @mention→task→reply chain is the one the acceptance names.
  - `Fixture::restart(signal)` (`:806-856`) — `/bin/kill -TERM|-KILL` the real service process, wait for exit, relaunch the binary over the SAME durable state dir on a fresh ephemeral port.
  - `RestartSignal { Term, Kill }` (`:66-72`).
  - `queue_project_mentions_at(addressed, round)` (`:1076`) — round-parameterized event ids so a restart's re-delivered mentions carry fresh `source_key` (server+room+event_id) and intake cannot dedup them.
  - `assert_project_scope` generation bound `1 + agents.len()` (`:515-525`).
  - `assert_ready` off-by-one fix: snapshot `agents` array = the full entries map (**root backend + n agents**), so `agents.len() == 1 + n`, matching `registered_backends == 1 + n` (`:780`).
  - DM owner-join find: `(0..self.agents.len())` + `!owner` guard (`:1462-1467`).
- `native/hagency/tests/configured_fleet.rs`
  - New `native_configured_fleet_three_agents_reattach_after_restart` (`:259-369`): for TERM and KILL, three agents answer three exact @mentions (round 1), the service restarts over durable state, and the re-attached runtimes answer three fresh @mentions (round 2) with **no account re-registration** (`account_posts` unchanged) and **no key re-upload** (`crypto.writes.len()` unchanged).

## Production wiring path (no test-only seams)

`hagency serve` → `bootstrap.rs:1365` `attach_factory` → `bootstrap.rs:1391` `fleet::Service::new` → `bootstrap.rs:1712` `fleet::Service::run` (`reattach_known_agents` on startup, then the 100 ms `take_next_provisioned_agent` loop). The test exercises this exact path through the real `CARGO_BIN_EXE_hagency` binary against durable state, not an in-process shortcut.

## Tests (exact commands and results — all run this session, verbatim)

- `cargo test -p hagency --test configured_fleet native_configured_fleet_project_mentions` → `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 18.70s`
- `cargo test -p hagency --test configured_fleet native_configured_fleet_three_agents_reattach_after_restart` → `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 45.49s`
- `cargo test -p hagency --test configured_fleet` (whole target) → `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 249.07s`
- `cargo test -p hagency-store --test approvals native_owner_approval_recovery` → `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 2.19s` (incl. `native_owner_approval_recovery_persistent_grant` — always-grant survival)
- `cargo test -p hagency-store --test owned_claim` → `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.77s` (incl. `native_owned_claim_reconciles_started_dispatch_after_host_death`, `native_owned_claim_profile_recovery_report_stays_queued`, `native_owned_claim_profile_done_followup_stays_queued` — queued/leased/started rules)

## A/B against base (LESSONS #2)

- `project_mentions` (n=2) failed at HEAD `25dfa1b2` at the `agents.len()` assertion (`left: 3, right: 2`) and at the stashed-base files (`left: 3, right: 2` at `mod.rs:767`). Root cause: my `25dfa1b2` parameterization changed `agents.len()` from the correct base value `3` (=1+2) to `n` (=2). The snapshot's `agents` array is the full entries map (root + n agents). Fixed to `1 + n`; `project_mentions` then passed.

## Not done / not needed

- No production (non-test) source change: the store and fleet already implement the TS restart semantics; the gap was the missing three-agent acceptance test plus the harness's hard-wired 2-agent plumbing.
- No SQL migration (assigned 064 unused — "if you need none, add none"): this change adds no schema.
- No console UI (board says none).
