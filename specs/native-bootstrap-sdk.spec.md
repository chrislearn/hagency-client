spec: task
name: "Historical bootstrap SDK configuration validation"
inherits: project
tags: [rust, security]
---

These library-only tests preserve validation of historical configuration without asserting that the production owner executable mounts its old runtime or approves an agent. They call `Bootstrap::open` directly; private fixture construction does not invoke the new CLI.

  Test: native_configured_fleet_profile
  Test: native_bootstrap_config
  Test: native_bootstrap_config_receive_inbox_absent_workspace

Provider path/profile/private-permission validation is Unix scoped and retained in the SDK test `native_bootstrap_local_codex`. Its existing platform-specific spec continues to bind it.
