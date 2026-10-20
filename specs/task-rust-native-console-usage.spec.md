spec: task
name: "Serve retained usage components through bounded native browser authority"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [active, rust, console, usage]
---

> Desktop-only scope (2026-10-20): CLI and Web page behavior below is historical.
> Shared engine selectors remain active unless explicitly marked Retired-Test.
> See [the desktop contract](native-owner-client.spec.md) for current product entry points.

## Intent

Implement the accepted ADR107 read-only M7 slice from migration1cb60d7 using the
existing browser page and the original typed native ledger. Root reviewed this
exact twenty-seven-path boundary before any source mutation and approved the
two-path amendment for the existing pinned cap-fs-ext dependency edge and the
thirtieth workflow path for a mandatory separate browser qualification lane and
the thirty-first inventory path to list all feature-gated tests without executing them.

## Decisions

The former Web product and browser CI lane are retired. Shared engine selectors
remain active. Compiled inventories list tests; execution evidence comes from
the engine and desktop test steps in CI.

## Constraints

### Must
- Preserve the existing usage component preferences and English and Chinese presentation.
- Keep native underlying operator and runner APIs closed to browser headers.
- Bound access issuance tickets sessions bodies queries requests assets and deadlines.
- Retain and verify actual nofollow asset snapshots before serving immutable bytes.
- Distinguish public document navigation from authenticated API request authority.
- Recheck original session after awaited reads and retire authority before shutdown.
- Preserve null counts absent periods incomplete history lower bounds and exact failures.
- Discover and select engagements created after the browser asset build.
- Retain same-selection observations with an explicit refresh indicator and mark failed refreshes stale while navigation credentials and logout clear mismatched or retired data.
- Run real Chromium against fresh native state in both languages and the real native executable without Node on PATH.
- Parse and lint before implementation then run exact nonzero tests and strict lifecycle with all changed paths.

### Must Not
- Do not put operator credentials tickets cookies or session secrets in assets or script-accessible browser storage.
- Do not create legacy fleet DTOs fixture success fallbacks billing claims or allocation inferences.
- Do not expose arbitrary proxy routes filesystem paths source records rooms or runtime identities.
- Do not contact live services modify live configuration add dependencies or claim all M7 workflows complete.

## Boundaries

### Allowed Changes
- tools/native/check-rust-spec-bindings.mjs
- .github/workflows/rust.yml
- ./Cargo.lock
- crates/hagency/Cargo.toml
- crates/hagency/src/lib.rs
- crates/hagency/src/main.rs
- crates/hagency/src/bootstrap.rs
- crates/hagency/src/console.rs
- crates/hagency/src/console/assets.rs
- crates/hagency/src/console/authority.rs
- crates/hagency/src/console/usage.rs
- crates/hagency/tests/console.rs
- crates/hagency/tests/console/fixture.rs
- crates/hagency/tests/console/browser.rs
- tests/dashboard-native-usage.test.js
- knowledge/decisions/adr-107-native-console-usage.md
- specs/task-rust-native-console-usage.spec.md
- docs/native-engine.md
- docs/agent-knowledge.md
- docs/progress.md

## Acceptance Criteria

Scenario: Lock contention cannot extend ticket or session authority
  Test: native_console_clock_after_lock
  Level: unit
  Test Double: clock callback inspects the held original mutex and separate real held-lock expiry regression
  Given a request waiting on the original session mutex
  When absolute expiry passes before that mutex becomes available
  Then the production clock is sampled after acquisition and the credential is refused

Scenario: A console link is reusable and its login does not expire on a timer
  Test: native_console_login_is_reusable
  Level: unit
  Given one issued console link
  When it is exchanged more than once and the logins are used later
  Then every exchange yields a working login and no rate limit or session cap refuses it

Scenario: Console logins survive a restart until revoked
  Test: native_console_logins_survive_a_restart
  Level: unit
  Given a link and a login persisted in the state directory
  When the authority is reopened, the login is revoked, and the authority is reopened again
  Then the login works after the first restart and is refused after revocation
  And a login made from the same link before revocation still works

Scenario: Browser authority has finite isolated scope
  Test: native_console_authority
  Level: integration
  Test Double: Salvo request transport over actual console authority and fresh writers
  Given an operator issuer and finite browser tickets and sessions
  When requests replay expire exceed bounds change authority or attempt native API access
  Then only the original current session reads the fixed console facade

Scenario: Assets retain verified original file evidence
  Retired-Test: native_console_assets


Scenario: Native usage remains truthful and bounded
  Test: native_console_usage
  Level: integration
  Test Double: fresh canonical engagement and actual typed ledger fixtures
  Given missing partial regressed and complete aggregate observations
  When bounded session reads or post-await retirement occur
  Then evidence optional counts periods and failures remain explicit without private fields

Scenario: Retained browser works against native service
  Retired-Test: native_console_browser


Scenario: Native executable serves without deployed Node
  Retired-Test: native_console_executable


## Out of Scope

Other console pages full Agent routing fleet usage aggregation health alerts
operations maintenance remote roles packaging production configuration and cutover.

Scenario: The console engagements read requires a browser session
  Test: native_console_engagements_read
  Given the console engagements read behind the console authenticate hoop with three seeded engagements including one carrying an astral project name truncated to 255 scalar values
  When a request arrives without a session, with a forged or duplicated cookie, with foreign host origin sec-fetch-site or forwarded headers, or with a foreign zero above-cap or repeated limit
  Then it is refused exactly as the usage console reads are, the default read publishes every wire key including requestedTokens, and paging at limit one through the opaque id cursor serves two content pages then an empty page with a null cursor while the astral name agrees on both bounds

Scenario: The console engagements document serves with a no-query rule
  Retired-Test: native_console_engagements_document


Scenario: The native origin hoops match the executed retained proxy rules
  Test: native_console_origin_matches_retained_vectors
  Level: integration
  Test Double: regenerated oracle vectors computed by the retained proxy and backend
  Given the regenerated console-origin vectors produced by executing the retained proxy and backend rules
  When the native console origin, canonicalisation and one-origin CORS predicates replay each request row
  Then allow, refuse and canonical verdicts agree with the retained result for every row, and the three named divergences where native is stricter are asserted rather than left implicit

## Owner-only browser qualification (2026-10-07)

The former browser qualification and Web console have been retired. Independent SDK HTTP/library selectors in this mixed contract remain active. Current desktop and OAuth qualification is defined in `native-owner-client.spec.md`; historical browser selectors are not passing evidence.

Desktop-only retirement (2026-10-20): CLI, static document and browser selectors above are retained as historical records, not passing evidence. Shared engine tests remain active.
