spec: task
name: "Retired CLI and Web project-side registration plan"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [retired, rust, registration]
---

This historical plan proposed standalone CLI and Web entry points. Both products
were removed on 2026-10-20. The deleted bootstrap command is no longer a production
caller. The original plan is available in Git history.

The store registration rules, generation fences and shared SDK route implementation
remain unchanged. Current product qualification is defined in
[the desktop contract](native-owner-client.spec.md).

These historical obligations had no bound test when retired. They are recorded as
unimplemented obligations, not as executed, passing or removed tests:

- native_registration_bootstrap_writes_the_fleet_row
- native_registration_route_writes_the_fleet_row
- native_registration_identical_reregistration_is_a_noop
- native_registration_stale_generation_is_refused
- native_registration_generation_advance_reconciles
- native_registration_refuses_an_invalid_record
- native_registration_must_match_the_hosts_engagement
- native_registration_route_requires_operator_authority
