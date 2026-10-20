spec: task
name: "Desktop owner client and retained engine boundary"
tags: [active, rust, desktop]
---

The desktop application is the only product entry point. The engine is a local
library dependency. There is no CLI Web console, static page bundle, user daemon
installer, or headless product release. Historical CLI and browser selectors in
other contracts are retired evidence, never relabelled as passed.

## Desktop engine and OAuth

- Native management uses the authenticated in-process facade, pinning the Matrix account.
  Test: native_in_process_facade_pins_matrix_account_without_exposing_credentials
  Production caller: hagency::native_owner::open_with_matrix
- The only actual listener serves the loopback OAuth callback; management APIs and documents are absent.
  Test: native_desktop_listener_exposes_only_oauth_callback
- Account switching revokes old grants and pending tickets.
  Test: account_switch_revokes_all_browser_grants_and_pending_ticket_without_retiring_host
- Account profiles keep Pasion identity and policy isolated.
  Test: native_account_profiles_switch_revoke_old_tabs_and_pin_pasion_identity
- Space discovery and creation retain owner authorization and bounded pagination.
  Test: native_owner_space_candidates_use_own_oauth_and_paginate_partial_failures
- Private owner state still refuses legacy data and permits only one process owner.
  Test: old_state_and_foreign_marker_are_refused_without_reading_or_rewriting_old_data
  Test: fresh_format_is_reopenable_and_has_one_process_owner

The desktop OAuth integration tests also live in hagency-app/src/login/oauth_tests.rs.
The normal desktop CI runs them with all application library tests. No test uses
personal provider credentials or paid model inference.
