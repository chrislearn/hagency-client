spec: task
name: "Prove the native upgrade procedure continues state and its rollback restores"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [active, rust, release, upgrade, recovery]
---

## Intent

Current scope: two builds of the same OwnerHost schema with only the version constant changed. This proves a same-format artifact switch and stopped private-state snapshot restoration. It does not authorize importing old Fleet/SQLite data, imply arbitrary version rollback, restore remote server credentials, or replay unresolved provider calls. The fixture uses current owner-local Ledger APIs; real binaries load the OwnerHost marker/console, while ledger custody is checked through the shared current SDK. No model is called.

Bind the definition-of-done line "fresh installation, upgrade, recovery and
the selected state-continuity strategy have tested procedures" for the
native half: ADR-134's versioned-artifact procedure (install N → install
N+1 → restart) and ADR-135's rollback step, exercised as a bound test —
state continues across the upgrade and version N runs again on the same
state after rollback. No decision here changes the procedure; the one new
decision — how N and N+1 are produced in a test without the release
workflow — is recorded in this spec and in ADR-134's note.

## Constraints

### Must
- Produce versions N and N+1 in-test WITHOUT the release workflow: two local builds of the same tree with different workspace versions (`hagency --version` derived from the workspace version constant), each built to its own path and named as ADR-134's versioned artifacts are (the binary name carries the version, so both coexist in the install dir exactly as the procedure assumes).
- Install version N by the documented procedure, write live state through it (seed the current owner ledger with permanent owner scope, policy and an unresolved provider charge whose budget remains held), then install version N+1 and restart the unit.
- Assert the upgrade continues state: the current owner marker and ledger remain readable, unresolved provider charges stay unknown and held, and fresh reservation cannot replay them, and the readiness word returns.
- Assert stopped snapshot recovery: restore the complete pre-upgrade owner directory to an empty destination and run N again. Unknown charge holds persist; post-snapshot context metadata is deliberately discarded. This is not arbitrary schema downgrade or a live snapshot.
- Copy and content-address all workspace-root compile-time inputs consumed by
  the native code: the four shared agent-home templates. Read them from the source tree; do not substitute template bytes.
- Build the copied N+1 source as a distinct `hagency-release-next` binary target with a shared dependency cache. Never overwrite or temporarily replace the currently tested `target/debug/hagency`; verify N still reports its original version after N+1 is staged. Only the copied manifest changes.


### Must Not
- Do not invoke the release workflow, a registry, a tag or any network artifact source — the two local builds are the whole fixture.
- Do not modify the store's schema-head or migration chain — the upgrade test consumes migrations as they are; no new migration is licensed here.
- Do not touch the retained JS deployment or its installer.
- Do not gate any scenario by OS or feature in the binding set.

## Boundaries

### Allowed Changes
- native/scripts/ (the two-version build and install harness)
- native/hagency/tests/
- install/install-native.sh and current per-user deploy templates
- specs/task-rust-native-upgrade.spec.md
- knowledge/decisions/adr-134-native-versioned-release.md
- docs/progress.md

### Forbidden
- Live services, credentials, deployed state, the production host.
- .github/workflows/**; native/hagency-store/src/migrations/**; deployed service state.

## Acceptance Criteria

Scenario: The documented upgrade procedure continues live state
  Test: native_upgrade_procedure_continues_state
  Level: integration
  Test Double: two local builds of the same tree at workspace versions N and N+1, installed by the documented procedure over one state dir
  Given version N installed with live state — a current owner marker and owner ledger with an unknown provider charge and held quota
  When version N+1 is installed by the documented procedure and the unit restarted
  Then the same-format owner state remains readable, with unknown charges and their held quota preserved
  And the readiness word returns

Scenario: The procedure's rollback step restores the previous version on the same state
  Test: native_upgrade_rollback_restores_previous
  Level: integration
  Test Double: the same two-version fixture after the upgrade scenario
  Given version N+1 running on the upgraded state
  When the procedure's rollback step is applied and the unit restarted
  Then version N runs again on the stopped same-format snapshot — no re-initialization or replay of unresolved calls
  And the recorded rows from before the upgrade are still readable

Scenario: Shared template changes participate in the real artifact's build inputs
  Test: native_upgrade_shared_templates_are_build_inputs
  Level: unit
  Given the exact root inputs used by the N+1 copy and fingerprint helpers
  When any of the four original shared templates changes in an isolated fixture
  Then that copied input preserves the exact bytes and the source fingerprint changes

## Decisions

**How N and N+1 are produced in a test.** Two local builds of the same
tree with different workspace-version constants — not the restart fixture's
single binary path (it cannot produce two coexisting versioned artifacts)
and not the release workflow (operator-gated, network-sourced). The version
constant is the same one `hagency --version` reads, so the artifacts are
named exactly as ADR-134's procedure installs them; ADR-134's note records
this as the tested form of the procedure. The version-patched copy is
rebuilt into the parent target dir with the inherited crate cache, at a
measured cost of about two minutes cold on a hosted lane and seconds warm;
the cheaper same-binary override (one build reporting a patched version)
was considered and rejected because only a real second artifact proves the
procedure, not merely the report.

## Out of Scope

The release workflow's enablement (operator), real tagged artifacts, the
two-host cutover drill (ADR-135's production exercise, operator), and the
retained JS deployment's upgrade path.
