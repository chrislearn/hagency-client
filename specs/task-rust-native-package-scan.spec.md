spec: task
name: "Retired standalone client packaging scan"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [retired, rust, release, packaging, audit]
---

The standalone CLI, service installer and reference service templates were
removed on 2026-10-20. Their packaging scenarios are retired, not successful
desktop checks. The original contract remains available in Git history.

Current desktop packaging is verified by the app CI and app build/release
workflows. See [the desktop contract](native-owner-client.spec.md).

Retired-Test: native_package_entrypoints_reference_no_node
Retired-Test: native_service_install_refuses_legacy_state_before_supervisor
