spec: task
name: Windows desktop automatic updates
satisfies: [REQ-WINDOWS-UPDATES]
tags: [active, rust, desktop, updates]
---

## Intent

Add signed background updates and explicit restart installation to the Windows
desktop without canceling existing Agent work or admitting new work during
installation preparation.

## Constraints

### Must
- Use the existing NSIS packaging identity and update the complete application.
- Verify signatures with the embedded key before using the manifest or installer.
- Fence installation and task admission atomically, retaining the fence through exit.
- Let the user postpone installation and keep downloaded packages across restarts.
- Keep machine preferences independent of Matrix login.
- Keep all local builds and test fixtures on D drive.

### Must Not
- Force terminate a user Agent task or the GUI to install an update.
- Automatically downgrade or install a prerelease from the stable feed.
- Change schemas, read production credentials, publish releases or change hosted apps.

## Boundaries

### Allowed Changes
- crates/hagency-updater/**
- crates/hagency-app/src/updates/**
- crates/hagency-app/src/app.rs
- crates/hagency-app/src/main.rs
- crates/hagency-app/src/lib.rs
- crates/hagency-app/src/hagency/backend.rs
- crates/hagency-app/src/login/login_screen.rs
- crates/hagency-app/src/settings/**
- crates/hagency-app/resources/i18n/**
- crates/hagency-app/Cargo.toml
- crates/hagency-app/packaging/**
- crates/hagency-app/tools/sign-windows-update.py
- crates/hagency/src/update_guard.rs
- crates/hagency/src/lib.rs
- crates/hagency/src/console/native.rs
- crates/hagency/src/console/native/speech.rs
- crates/hagency/src/console/owned_runtime.rs
- Cargo.lock
- typos.toml
- .github/workflows/app-*.yml
- tools/test-windows-updater.ps1
- knowledge/requirements/req-windows-updates.md
- specs/task-windows-updates.spec.md
- dev-vibe/**

## Acceptance Criteria

Scenario: Existing Agent work postpones installation and blocks racing new work
  Test: update_waits_for_tasks_and_fences_new_admission
  Level: unit
  Test Double: isolated real process admission gate
  Given two admitted tasks
  When installation is requested
  Then installation stays deferred until both task guards finish
  And an installation guard prevents new task admission until released

## Verification

Run the portable hagency-updater signature, size, storage and preferences tests;
the Windows file-lock test; the native narrow-layout update controls test; strict
Clippy; and the app library suite. Run tools/test-windows-updater.ps1 against real
NSIS to verify parent-exit ordering, restart at a path containing spaces,
cancellation without a shutdown commit, and rejection of a corrupted package.
These installer fixtures use only disposable D drive data and ephemeral keys.
