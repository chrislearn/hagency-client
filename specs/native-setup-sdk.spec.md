spec: task
name: "Setup SDK explicit private profile validation"
inherits: project
tags: [rust, only-macos, only-linux]
---

Library-only setup tests use temporary explicit provider paths and an unsigned synthetic executable. They do not invoke the removed setup CLI or copy daily provider authentication.

  Test: native_setup_sdk_materializes_explicit_private_profile_and_requires_explicit_replace
  Test: native_setup_sdk_refuses_foreign_state_without_import
  Test: native_setup_sdk_can_use_dedicated_unsigned_provider_home
