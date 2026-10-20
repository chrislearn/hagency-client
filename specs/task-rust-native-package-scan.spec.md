spec: task
name: "Scan the packaged native entrypoints for residual Node references"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [active, rust, release, packaging, audit]
---

## Intent

Bind the M8 item-6 gate (the migration plan's definition of done, line 3):
the packaged native runtime must not invoke Node.js or hide an embedded
JavaScript runtime. Today that property is proven by manually reading the
two unit templates; this slice makes it a **bound test** — a scan over the
native packaging surface as it actually exists: the two deploy unit
templates by path. Service installation uses the binary's own per-user
`hagency-cli service install` command; the redundant shell wrapper is removed.
ADR-134 (versioned release) governs the packaged artifact
set; ADR-134's already-bound staged-tree scan
(`native_release_entrypoints_scan_finds_no_node`,
`tests/release_cutover.rs:295`) covers the staged release tree — a
different surface; this spec binds the **reference-template** surface, so
the two selectors are distinct checks, not the same one twice.

## Constraints

### Must
- Scan the two native unit templates by path: `deploy/io.hagency.native.plist` (launchd) and `deploy/hagency-native.service` (systemd) — exactly the files `deploy/` carries for the native deployment.
- Execute the real CLI's `service install` command against isolated legacy state and prove it rejects that state before any system service registration, without changing the legacy bytes or creating owner credentials.
- Assert none of the scanned invocation lines (ProgramArguments / ExecStart / Exec words) references `node`, `npm`, `npx`, `__NODE_BIN__` or a `.js`/`.mjs` script path.
- Run as a bound test in the native test suite — not a manual reading, not a CI-only grep outside the gate.

### Must Not
- Do not scan or constrain the RETAINED deployment's files (`deploy/com.hagency.supervisor.plist`, the retained `.service` units, `install/install-macos.sh` and its node checks): they run Node by design and are removed at M9 cutover, not by this gate.
- Do not claim "generated hook templates" or "the versioned-artifact ExecStart strings" — the tree carries no hook templates under the native packaging paths, and the current templates' invocation lines carry the unversioned `hagency` placeholder; the scan asserts what exists.
- Do not restore retired Fleet flags or handwritten service renderers. The binary is the sole production unit generator.
- Do not gate any scenario by OS or feature in the binding set.

## Boundaries

### Allowed Changes
- tools/native/ (the scan script or test module)
- crates/hagency/tests/
- specs/task-rust-native-package-scan.spec.md
- docs/progress.md

### Forbidden
- Live services, credentials, deployed state.
- .github/workflows/**; all real deployed services and state.

## Acceptance Criteria

Scenario: The packaged native entrypoints reference no Node runtime
  Retired-Test: native_package_entrypoints_reference_no_node
  Level: integration
  Test Double: the two deploy reference unit templates
  Given deploy/io.hagency.native.plist and deploy/hagency-native.service
  When the scan runs over their invocation lines
  Then none references node, npm, npx or a Node script path
  And the assertion is a bound test whose failure names the offending file and line

## Decisions

**Single unit generator.** The current binary writes escaped per-user systemd/LaunchAgent units. The removed shell wrapper duplicated this command and still expected the old executable name. The reference templates remain scanner and release-fixture inputs. Binary-generated unit escaping and supervisor arguments remain covered by the current service tests.

Scenario: Service installation rejects legacy state before supervisor registration
  Retired-Test: native_service_install_refuses_legacy_state_before_supervisor
  Level: integration
  Given private legacy state in a path containing spaces and the real current binary
  When service install is invoked directly with that state directory and no-open
  Then legacy bytes are unchanged without a new format marker or operator token
  And the command fails before registering or starting a supervisor service

## Out of Scope

The staged-release-tree scan (ADR-134's, already bound), the release
workflow's enablement (operator), and any change to the retained
deployment's files.

Desktop-only retirement (2026-10-20): CLI, static document and browser selectors above are retained as historical records, not passing evidence. Shared engine tests remain active.
