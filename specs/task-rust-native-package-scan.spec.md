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
templates by path, plus the native installer wrapper that delegates to the binary's current per-user `service install` implementation. ADR-134 (versioned release) governs the packaged artifact
set; ADR-134's already-bound staged-tree scan
(`native_release_entrypoints_scan_finds_no_node`,
`tests/release_cutover.rs:295`) covers the staged release tree — a
different surface; this spec binds the **templates-and-wrapper** surface, so
the two selectors are distinct checks, not the same one twice.

## Constraints

### Must
- Scan the two native unit templates by path: `deploy/io.hagency.native.plist` (launchd) and `deploy/hagency-native.service` (systemd) — exactly the files `deploy/` carries for the native deployment.
- Scan the actual installer wrapper and assert delegation to `service install`; separately execute it with an isolated argv-capture binary, preserve its error status, and prove rejected legacy arguments cannot reach the binary. Real binary service-format checks reject legacy state before any system service registration.
- Assert none of the scanned invocation lines (ProgramArguments / ExecStart / Exec words) references `node`, `npm`, `npx`, `__NODE_BIN__` or a `.js`/`.mjs` script path.
- Run as a bound test in the native test suite — not a manual reading, not a CI-only grep outside the gate.

### Must Not
- Do not scan or constrain the RETAINED deployment's files (`deploy/com.hagency.supervisor.plist`, the retained `.service` units, `install/install-macos.sh` and its node checks): they run Node by design and are removed at M9 cutover, not by this gate.
- Do not claim "generated hook templates" or "the versioned-artifact ExecStart strings" — the tree carries no hook templates under the native packaging paths, and the current templates' invocation lines carry the unversioned `hagency` placeholder; the scan asserts what exists.
- Do not restore retired Fleet flags or handwritten service renderers. The binary is the sole production unit generator.
- Do not gate any scenario by OS or feature in the binding set.

## Boundaries

### Allowed Changes
- native/scripts/ (the scan script or test module)
- native/hagency/tests/
- specs/task-rust-native-package-scan.spec.md
- docs/progress.md

### Forbidden
- Live services, credentials, deployed state.
- .github/workflows/**; all real deployed services and state.

## Acceptance Criteria

Scenario: The packaged native entrypoints reference no Node runtime
  Test: native_package_entrypoints_reference_no_node
  Level: integration
  Test Double: the two deploy unit templates as shipped, plus the actual delegating installer wrapper
  Given the native packaging surface — deploy/io.hagency.native.plist and deploy/hagency-native.service, and the actual service-install wrapper
  When the scan runs over their invocation lines
  Then none references node, npm, npx or a Node script path
  And the assertion is a bound test whose failure names the offending file and line

## Decisions

**Single unit generator.** The current binary writes escaped per-user systemd/LaunchAgent units. The installer delegates to it instead of duplicating unit rendering or accepting legacy Fleet configuration. The reference templates remain scanner inputs. Binary-generated unit escaping is covered by the current service tests.

Scenario: Installer preserves refusal and passes current arguments exactly
  Test: native_installer_delegates_current_service_and_refuses_legacy_state
  Level: integration
  Given a private legacy state and the real current binary, then a private argv-capture executable
  When the actual shell installer is invoked
  Then legacy bytes are unchanged without a new format marker or operator token
  And current service install argv are preserved including spaces and its failure exit code
  And retired arguments are rejected before delegation

## Out of Scope

The staged-release-tree scan (ADR-134's, already bound), the release
workflow's enablement (operator), and any change to the retained
deployment's files.
