spec: task
name: "Manage native resource catalog visibility in the retained console"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [active, rust, console, resources]
---

> Desktop-only scope (2026-10-20): CLI and Web page behavior below is historical.
> Shared engine selectors remain active unless explicitly marked Retired-Test.
> See [the desktop contract](native-owner-client.spec.md) for current product entry points.

## Intent

Implement accepted ADR108 from the clean dfba95b checkpoint. Root reviewed the
concrete scope, original 31 paths, lock order and transition table before mutation.
Root approved NativeUsage as the exact 32nd path for shared truthful logout retry.
This is the next step toward full resource management; the retained wizard's
missing native fields and discovery contracts remain explicit follow-up work.

## Decisions

The former Web product and browser CI lane are retired. Shared SDK tests
remain active. Listing tests never counts as execution evidence.

## Constraints

### Must
- Retain the existing resource page controls preferences and both languages.
- Keep native operator and runner APIs closed to browser authority.
- Keep ordinary console sessions read only and grant publication only through an explicit finite management ticket.
- Retain a concrete opaque non-Clone non-Deserialize command from the exact original management session.
- Acquire SQLite IMMEDIATE before the per-session nonblocking mutation gate and never acquire the global session map from that gate.
- Check original session and request deadlines after SQLite waiting and before mutation and commit without renewing them.
- Keep unrelated reads and sessions independent of a mutation gate held through disk IO.
- Compare actual configuration revision and change only publication and derived roles.
- Preserve missing pool and shared-account values and periods without inventing legacy DTO fields.
- Bound requests queries bytes pages assets and deadlines using the existing console limits.
- Keep Busy and unknown logout visible with an explicit retry and no claimed revocation.
- Keep mutation conflict and unknown visible with read-only current-state reconciliation and no automatic write retry.
- Test actual dynamic resource selection and local catalog publication in English and Chinese browsers and the native executable with an empty runtime PATH.
- Parse and lint the contract then verify exact nonzero selectors and all changed paths.

### Must Not
- Do not put credentials auth-home paths internal preset IDs or full resource configurations in browser DTOs.
- Do not claim native catalog changes establish remote Palpo publication.
- Do not expose a generic browser-authority callback or accept a browser-supplied scope assertion.
- Do not replace the retained console with a separate minimal product.
- Do not invent runtime readiness seat members rate caps execution policy or provider accounting.
- Do not call live services change live configuration widen deadlines introduce dependencies or claim M7 complete.

## Boundaries

### Allowed Changes
- knowledge/decisions/adr-108-native-console-resource-publication.md
- specs/task-rust-native-console-resource-publication.spec.md
- docs/native-engine.md
- docs/agent-knowledge.md
- docs/progress.md
- crates/hagency-store/src/lib.rs
- crates/hagency-store/src/domain.rs
- crates/hagency-store/src/domain/resource_publication.rs
- crates/hagency-store/src/domain_worker.rs
- crates/hagency-store/tests/resource_publication.rs
- crates/hagency/src/console.rs
- crates/hagency/src/console/authority.rs
- crates/hagency/src/console/resources.rs
- crates/hagency/src/console/assets.rs
- crates/hagency/src/console/usage.rs
- crates/hagency/src/main.rs
- crates/hagency/tests/console.rs
- crates/hagency/tests/console/fixture.rs
- crates/hagency/tests/console/browser.rs
- crates/hagency/tests/console/resources.rs
- tests/dashboard-native-resources.test.js

## Acceptance Criteria

Scenario: Publication compares the actual configuration atomically
  Test: native_resource_publication_cas
  Level: integration
  Test Double: original private SQLite repository and resource edits
  Given a native configuration and a revision read from its actual stored fields
  When current or stale publication commands arrive
  Then only the matching revision changes publication without overwriting profile or budget

Scenario: Original finite publication authority survives queued work
  Test: native_resource_publication_current
  Level: integration
  Test Double: original SQLite write lock and concrete session gate with real expiry
  Given an exact session command with original deadlines
  When queue or SQLite waits caller loss expiry retirement or gate contention occurs
  Then stale commands cannot mutate and possible commits remain unknown without retry

Scenario: Browser publication needs its exact management scope
  Test: native_console_resource_authority
  Level: integration
  Test Double: actual finite console authority and native writer behind Salvo
  Given read-only and management sessions from the native issuer
  When publication or logout races requests and expiry
  Then only current management scope admits writes and Busy never claims revocation

Scenario: Resource observations preserve native meanings
  Test: native_console_resource_observations
  Level: integration
  Test Double: canonical configurations seats roles and commitments in fresh native state with one unmeasured resource one carrying a bound usage source and observations and a second preset sharing the first pool's seat
  Given published withdrawn missing and partially bounded resources
  When bounded resource and budget reads complete or lose authority
  Then safe DTOs preserve current native facts and absent values without private fields and the budget draw object carries drawn measured consumed the binding draw named as the over-commit refusal names it and remaining before the ceiling with every unknown figure null never zero while the console wire omits the always-zero reserved key and the draw's committed equals the pool's committed across the shared seat with the seat figure deliberately larger and a clock fault refuses as console unavailable never busy

Scenario: Retained resource controls work in both languages
  Retired-Test: native_console_resources_browser


Scenario: Resource management runs without deployed Node
  Retired-Test: native_console_resources_executable


## Out of Scope

Resource creation full profile editing friendly-name persistence rate caps execution
policy host discovery Agent detail routing seat declaration edits role publication
writes remote Palpo delivery production packaging and complete M7 parity.

## Owner-only browser qualification (2026-10-07)

The former browser qualification and Web console have been retired. Independent SDK HTTP/library selectors in this mixed contract remain active. Current desktop and OAuth qualification is defined in `native-owner-client.spec.md`; historical browser selectors are not passing evidence.
