spec: task
name: "Serve implemented agent lifecycle commands behind one finite lifecycle scope"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [active, rust, console, agents, lifecycle]
---

> Desktop-only scope (2026-10-20): CLI and Web page behavior below is historical.
> Shared engine selectors remain active unless explicitly marked Retired-Test.
> See [the desktop contract](native-owner-client.spec.md) for current product entry points.

## Intent

Bind CL-S2 (ADR-130, D-SCOPE in force) on top of CL-S1's landed roster read:
one finite scope, `Scope::AgentLifecycle`, owning the operator's lifecycle
commands. **Stop** is implemented as fence-and-record via the live set or an
unsettled `dispatch_stops` row. **Start** and **preset-apply** retain named API
routes for compatibility but fail closed: native has neither a durable agent
lifecycle record that can re-arm a stopped dispatch nor a safe transition that
can move an engagement's resource, budget, account binding and provision effect
together. The roster renders only controls with a real durable effect.

## Constraints

### Must
- Mint `Scope::AgentLifecycle` only through the operator-authenticated `/api/native/v1/console/access` under ADR-107's issuance rules verbatim: one issuance per second, one outstanding ticket at a time, replacement invalidating the preceding, exchanged before the next is minted; the CLI flag `--manage-agent-lifecycle` is mutually exclusive with both existing management flags, declared and asserted pairwise.
- Add exactly one new public store surface — `stop_dispatch_for_agent` on `DomainStore`/`DomainRepository` (`domain_worker.rs` wrapping `conversation_lifecycle.rs`): the read-plus-stop entry that resolves the named engagement's dispatch (live set or unsettled `dispatch_stops` row) and fences it. It is the **only** store surface this slice adds; `fence_dispatch` stays `pub(super)`, and `settle_conversation_stop` remains uncallable from runtime-facing commands.
- Scope the grant to a session's `Grant` — the existing caps govern (at most 4 concurrent sessions, absolute 15-minute lifetime, no rolling expiry); the scope adds no longer-lived credential.
- Make start refuse with `agent_start_unavailable` (HTTP 501) for every authorized request until a durable host-owned re-arm transition exists; it reads no roster and spawns nothing.
- Make stop idempotent through the resolution predicate — live set (`queued`,`leased`,`started`,`parked`) or an unsettled `dispatch_stops` row, newest by id — fencing only the resolved dispatch, never `retire`'s session cascade, and serving the five-key wire object (`stopped`, `stop_pending`, `dispatch_id`, `fence`, `state`) with refusals in the console's existing `{"ok":false,"code":…}` envelope.
- Make preset-apply refuse with `agent_preset_unavailable` (HTTP 501) for every authorized request until one transaction can preserve the engagement's resource, budget, account and provision-effect invariants; keep no in-memory pseudo-binding.
- Render only implemented lifecycle controls from CL-S1's served `permissions` booleans on the roster: stop is visible to the scoped session; start and preset are absent; a read-only session renders none enabled.
- Add the scope's user-facing strings to both dictionaries (`en` and `zh` in `mockup/lib/i18n.js`).

### Must Not
- Do not report success for a lifecycle command that changed no durable state. Do not grant through this scope: resource publication or configuration, any account act (enrollment, readiness, credential namespace), dispatch or runner capability, workspace access, Matrix path or content, any child process or argv, or the settlement of any stop — `settle_conversation_stop` stays uncallable from runtime-facing commands.
- Do not widen an existing scope or fold lifecycle into configure (D-SCOPE rejected it); do not change the two existing scopes, their flags' `conflicts_with`, or any existing mutation route.
- Do not change `one_live_session` (`003:19`), the `dispatch_stops` DDL, `fence_dispatch`'s uncertain-vs-determinate split, `approval_verdict_receipts`, or the close path.
- Do not touch `browser_boundary`/`authenticate`/`common_authority`/`same_origin`, the five-document exception, the 8-permit semaphore, session rules, or `assets.rs:157`'s required key.
- Do not gate any scenario by OS or feature in the binding set (the former browser selector is retired).

## Boundaries

### Allowed Changes
- crates/hagency/src/main.rs
- crates/hagency/src/console.rs # the lifecycle router + operator issuer wiring and the scope error map — the scope cannot route or refuse without them
- crates/hagency/src/console/authority.rs
- crates/hagency/src/console/agents.rs
- crates/hagency-store/src/domain_worker.rs
- crates/hagency-store/src/domain/conversation_lifecycle.rs
- crates/hagency/tests/console/agents.rs
- crates/hagency/tests/console.rs
- crates/hagency/tests/cli.rs
- crates/hagency/tests/console/browser.rs
- specs/task-rust-console-agent-lifecycle.spec.md
- knowledge/decisions/adr-130-native-agent-lifecycle-authority.md
- docs/progress.md

### Forbidden
- Live services, live agents, credentials and deployed state.
- crates/hagency/src/console/resources.rs; mockup/app/api/**; every other hagency-store file (the two licensed store paths carry exactly one new public entry — `stop_dispatch_for_agent`, F1's fix — and the fence kernel and settlement stay the store's own).

## Acceptance Criteria

Scenario: One console login carries every action class
  Test: native_console_one_login_full_authority
  Level: unit
  Given one issued console link exchanged for a login
  When the login is checked for read, lifecycle and configuration actions and then logged out
  Then it is admitted for every action class
  And logout revokes all of them at once

Scenario: Start fails closed and stop is at-most-once over the store's own state
  Test: native_console_agent_start_stop_is_at_most_once
  Level: integration
  Test Double: an engagement with a dispatch in state started and a real domain writer
  Given an agent whose dispatch is live and then fenced by a first stop
  When start is called twice and stop is called again
  Then start refuses with agent_start_unavailable and spawns nothing
  And the second stop resolves the same dispatch id and fence through the unsettled stop row writing no second row and still reports stop_pending
  And stopped stays false because no production path settles

Scenario: Operator recovery resumes an orphaned dispatch exactly once
  Test: native_console_agent_recover_dispatch_recovers_orphan
  Level: integration
  Test Double: a settled orphan dispatch with held lease quarantined session and dirty workspace
  Production caller: hagency::console::agents::recover_dispatch
  Given an engagement whose dispatch was settled to the orphan state with its lease held session quarantined and workspace dirty
  When a read-only session posts recovery and then a lifecycle operator posts recovery with evidence
  Then the read-only post is refused before any row changes
  And the operator post clears the lease clears the quarantine clears the dirty flag supersedes the orphan and enqueues the replacement
  And the recovery row records the operator evidence unchanged
  And the same recovery posted again is refused with recovery_conflict and writes no second recovery row

Scenario: Recovery refuses a dispatch already fenced by a stop
  Test: native_console_agent_recover_dispatch_refuses_stopped_dispatch
  Level: integration
  Test Double: a dispatch carrying a dispatch_stops row and a real domain writer
  Given an engagement whose dispatch has an unsettled stop row from the conversation-stop flow
  When a lifecycle operator posts recover-dispatch for that dispatch
  Then the route refuses with the named not-recoverable word and no lease quarantine dirty or recovery row changes

Scenario: Preset-apply refuses until its durable transition exists
  Test: native_console_agent_preset_apply_refuses_without_durable_transition
  Level: integration
  Test Double: an active engagement bound to a native resource
  Given the engagement's resource-derived framework budget and provision effect
  When preset-apply is called through a lifecycle session
  Then it refuses with agent_preset_unavailable
  And the engagement projection remains unchanged
  And no browser-memory pending marker is created

Scenario: The roster page renders lifecycle controls only from served permissions
  Retired-Test: native_console_agent_lifecycle_browser


## Decisions

**Start's refusal word is `agent_start_unavailable` (HTTP 501).** Native has no
agent lifecycle record independent of an engagement and no host-owned transition
that can settle a stop and re-arm work. The route refuses before any store read
or spawn; a successful no-op is forbidden.

**Preset-apply's refusal word is `agent_preset_unavailable` (HTTP 501).** An
engagement's resource id determines its budget, account binding, qualification
and provision effect. An in-memory preset pointer changes none of those facts and
is not an apply. The route therefore keeps no pending marker and returns no
success until a complete durable retire/reprovision transition is designed.

**The flag's pairwise exclusivity is declared AND asserted.**
`--manage-agent-lifecycle` declares `conflicts_with_all` against both
management flags, and the CLI selector asserts each combination is refused
before any ticket issues — a declaration is not a test.

**Retired browser driver.** The former lifecycle browser scenario and its driver
were removed with the Web product.

**Stop's widening to `retire`'s session cascade is a later slice.** The route
fences only the resolved dispatch — the one the named engagement resolves —
and does not reuse `retire`'s session-keyed walk, which also closes child
conversations (`conversation_lifecycle.rs:125-150`). Widening is a deliberate
future decision with its own review, not a default; the cost (sibling
dispatches keep running) is accepted now.

**One new public store entry, named.** `stop_dispatch_for_agent`
(`DomainStore` wrapper in `domain_worker.rs`, over the domain file) is the
slice's entire store surface (F1's fix): the selector's resolution and the
fence travel through it, and nothing else on the store becomes reachable from
the console crate.

**Superseded: console scopes.** The console now issues one login that carries every
action class (read, agent lifecycle, configuration, accounts), so the scoped tickets,
their mutual exclusivity and the one-per-second / one-outstanding issuance rules this
section described no longer exist. The single-login behaviour is pinned by
`native_console_one_login_full_authority`, `native_console_login_is_reusable` and
`native_console_logins_survive_a_restart`.

## Out of Scope

CL-S1's roster read itself (landed), durable start/re-arm, resource rebinding or
reprovisioning, the stop-widening to `retire`'s session cascade, the host
settlement path (the host's own), the agent-lifecycle CLI read, and every other
console page.

## Owner-only browser qualification (2026-10-07)

The former browser qualification and Web console have been retired. Independent SDK HTTP/library selectors in this mixed contract remain active. Current desktop and OAuth qualification is defined in `native-owner-client.spec.md`; historical browser selectors are not passing evidence.
