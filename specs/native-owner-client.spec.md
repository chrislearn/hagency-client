spec: task
name: "OwnerHost executable release qualification"
inherits: project
tags: [active, rust, security]
---

## Product scope

The native product constructs OwnerHost, requiring a fresh private owned-client state and user Pasion authorization. It never restores the historical Fleet/operator/custody CLI. The historical SDK specifications and their library tests remain compiled and checked by the Rust binding inventory. Their historical `Production caller:` claims no longer qualify this executable. CI explicitly selects this spec for the current production call graph; running the checker without `--spec` still diagnoses all historical claims rather than hiding them.

## Executable entry points

- Initialize private state without importing old domain databases; repeated initialization is safe.
  Test: native_owner_init_is_private_idempotent_and_has_no_legacy_store
  Production caller: hagency::owner_host::initialize
- Start/serve constructs the owner console, and never starts inference automatically.
  Test: native_owner_start_serves_only_owner_routes
  Production caller: hagency::owner_host::OwnerHost::open
- Access is issued over owner-private OS IPC. A browser cookie alone never grants Pasion rights.
  Test: native_owner_open_uses_private_ipc_single_use_ticket_without_pasion_rights
  Production caller: hagency::service::open_owner_link
- Install/uninstall configures only the per-user supervisor, preserves private state, and uses the owner start command.
  Test: native_owner_service_install_uninstall_uses_only_per_user_supervisor
  Production caller: hagency::service::run
- Removed commands/options and old data are refused, without migration or transfer.
  Test: native_owner_rejects_retired_commands_and_options
  Test: native_owner_rejects_legacy_data_without_import
- Crash recovery retains the owner format; restart never restores Pasion/device/provider authorization.
  Test: native_binary_survives_crash_without_node
  Test: native_service_unit_restart_preserves_owner_state
  Test: native_launchd_restart_preserves_owner_state
- A clean supervisor stop finishes within the existing 20-second bound.
  Test: native_service_unit_stops_cleanly_within_timeout_budget
  Test: native_launchd_agent_stops_cleanly
- Real bundled owner navigation is served by the actual native executable; retired Fleet documents are absent.
  Test: native_console_rail_pages_are_shipped_and_served

## Browser and release artifact gates

Switching an account revokes every old browser grant and pending entry ticket, without permanently retiring the local host.

  Test: account_switch_revokes_all_browser_grants_and_pending_ticket_without_retiring_host

Client pages verify the current browser Matrix session before exposing Agent controls. Anonymous entry and expired sessions return to Matrix sign-in. Shared login status cannot substitute for authenticated owner API validation. Server history contains only bounded canonical origins; the default device name requires no input. Browser artifact verification covers these UI flows with explicit API fixtures, without provider inference.

OAuth callbacks return to `/console/`, never a retired Fleet document. Public owner HTML documents accept same-site and cross-site navigation after Pasion authorization; API reads still enforce their origin/session checks, static assets do not gain cross-origin access, and document query strings remain refused.

  Test: native_owner_oauth_return_documents_allow_external_navigation_but_not_api_reads

`rust.yml` builds the owner bundle, runs the real binary navigation test, and runs `check-owner-console.mjs` against actual browser components with explicit mocked API responses. This replaces historical Fleet browser walks, whose pages are no longer packaged. SDK route tests continue in the unfiltered workspace suite. No gate uses personal provider credentials or paid inference.

`release-native.yml` remains dispatch-only. Its embedded-binary smoke tests login and owned-Agent documents, retired route 404s, owner authorization 401s and private-IPC opening. This does not claim real model, encrypted-room or hosted launchd/systemd qualification. CLI install tests use isolated HOME and controlled supervisor helpers; foreground stop/restart tests use actual binary processes and OS signals.

Unix IPC and supervisor installation gates run the real IPC and an isolated per-user helper on Unix hosts. Non-Unix diagnostic runs assert the explicit unavailable/invalid-listener refusal; they do not claim supervisor or private IPC qualification.

## Bootstrap replacement coverage

The historical bootstrap product tests required `--development-driver`, client-side AS/registration secrets and encrypted private Matrix approval bots. Those production features were explicitly removed. New executable bootstrap tests reject those old config/secret files, deny anonymous model/tool/provider controls and require fresh Pasion authorization after restart. Independent SDK configuration validation remains in `tests/bootstrap.rs` using direct library calls and private fixtures, and is separately bound by `native-bootstrap-sdk.spec.md`; no old runner is restored to the executable.

  Test: native_owner_bootstrap_refuses_old_driver_and_matrix_secret_files
  Test: native_owner_bootstrap_no_anonymous_provider_model_tool_or_approval_access
  Test: native_owner_bootstrap_restart_does_not_restore_owner_session

  Test: native_owner_palpo_client_is_not_a_fleet_publisher_or_appservice
  Test: native_owner_setup_command_cannot_import_daily_provider_credentials
  Test: native_owner_start_invalid_console_creates_no_credentials
  Test: native_console_assets_refusal_names_field_and_fix

Private bundle refusals name the console-assets field and an actionable static fix without printing credential bytes. Invalid bundle startup creates no owner credentials or Agent ledgers.

Browser login qualification: `mockup/scripts/check-owner-login.mjs` validates canonical bounded server history, unavailable browser storage, current-cookie authorization, default device naming and explicit logout. `check-owner-console.mjs` additionally checks anonymous Agent navigation and expired-session redirection. OAuth returns to `/console/`, and protected APIs continue requiring current owner authority.

Account profile switching keeps each server/issuer/subject/MXID identity separate. The native switch route revokes old browser/device authority before stopping running owner work, issues a finite local login bridge, and never restores Pasion or provider credentials from saved identity metadata. Concurrent old-cookie switches have at most one winner; old-profile logout cannot stop the newly selected owner.

  Test: native_account_profiles_switch_revoke_old_tabs_and_pin_pasion_identity

## Final review regression coverage

  Test: private_ipc_accepts_ipv6_loopback_access_link

  Test: native_owner_space_candidates_use_own_oauth_and_paginate_partial_failures
