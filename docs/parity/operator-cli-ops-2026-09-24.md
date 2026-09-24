# Operator-operations parity audit: TS Hagency vs the Rust `hagency` binary

The Rust binary is well behind TS for work done outside the browser. Out of 42 operator operations, 7 are ported, 3 were changed on purpose (with a cited decision), 15 are partial, 10 are missing and 7 can only be done by hand-editing files. Project-side onboarding is the worst area: you set it up by placing secrets and JSON by hand, and nothing issues or checks an appservice registration.

**Paths used below**
- **TS** = `TS-repo`
- **R** = `TS-repo-rust-migration-finish-20260915`
- **RS** = `R/native/hagency/src`

**Rust command set.** The clap tree in `RS/main.rs:17-166` has these subcommands: `mcp`, `task` (runner-side), `init`, `account {prepare,inspect,retire,login}`, `registration register`, `provision existing`, `intake-refuse-stale-session`, `console-access`, `alerts`, `engagements`, `resources` and `serve`. Of these, `alerts`, `engagements` and `resources` are read-only inspection commands.

**TS command set.** There are 21 commands (`TS/scripts/cli-command-manifest.json`), plus installers, `upgrade.sh`, `services/hagency-services.mjs` and the REST routes in `backend-v2.js`.

---

## Confirmation: onboarding a project side in Rust (confirmed, with evidence)

1. **The credential is a secret file you place by hand. No command or route writes it.**
   - `RS/bootstrap/config.rs:642-649` picks the file by profile:
     ```rust
     let token = if as_namespace.is_some() {
         read(&state.join("matrix.appservice_token"), 4096)?
     } else {
         read(&state.join("matrix.registration_token"), 64)?
     };
     ```
   - Some profiles also need more hand-placed files:
     - `matrix.provisioning_key`: exactly 32 bytes (`config.rs:651`).
     - `matrix.representative_token`: needed when `peer_masters` is set (`config.rs:676`).
     - `matrix.ca.pem`: optional (`config.rs:697`).
   - `read()` goes through `private::open` (`config.rs:300-312`), which refuses any file that is not owner-private (`hagency-store/src/private.rs:56,74,99`, `mode & 0o077`).
2. **`namespace_prefix` is written by hand in `agent-driver.json`.**
   - It is part of the closed profile `appservice_login_home_rooms_enrollment_step_v1` (`config.rs:224-229`).
   - `agent-driver.json` is loaded at `config.rs:403-406`.
   - The prefix must be lowercase letters, digits or `_`, 128 characters or fewer (`R/native/hagency-matrix/src/token_provision/application_service.rs:18-28`).
   - The same JSON also needs a hand-computed `matrix.registration_fingerprint`. It must equal the canonical digest of the registration (`hagency-matrix/src/provisioning.rs:117-119,189`; 64 hex characters, `hagency-matrix/src/config.rs:112`). Nothing prints this digest: `registration register` outputs only `{"ok":true}` (`RS/main.rs:236-240`).
3. **Nothing issues an appservice registration.**
   - Searching all native `.rs` files for `as_token|hs_token|asToken|hsToken|/_matrix/app/` finds nothing outside tests.
   - The only project-side write is the six-field fleet record: `POST /console/api/project-sides` (`RS/console/project_sides.rs:100-140`) and the CLI `registration register` (`RS/bootstrap/registration.rs:22-59`). Neither carries a credential.
   - TS has two routes for this:
     - `POST /api/project-sides/:id/registration-file` (`TS/backend-v2.js:10031-10168`) generates tokens, writes a 0600 YAML under `$HAGENCY_RUNTIME_DIR/registrations`, returns fingerprints and next steps, and holds a reissue as staged until it is verified.
     - `POST /api/project-sides/:id/registration` (`:10170-10220`) returns the YAML once, and returns 409 without `?replace=true`.
4. **Nothing verifies a registration or a credential.**
   - TS has `POST /api/project-sides/:id/verify` (`TS/backend-v2.js:9918-9992`), which tries the staged credential first and promotes it automatically, and `GET /api/matrix/reach` (`:9698`).
   - Rust has neither. Its project-side view lists `credential_kind`, `has_credential`, `awaiting_install`, `namespace`, `access_state` and `access_detail` as `unavailable` (`RS/console/project_sides.rs:33-47`; ADR-132 lines 23-26 say "Native has no project-side model and no credential anywhere").
   - A bad token only shows up as `/ready` error `"config"` or `"registration"` (`RS/bootstrap.rs:693-700`).
5. **This is not a deliberate change.**
   - `R/specs/task-rust-project-side-registration.spec.md:240-247` lists "registration-**file** issuance (`:10030`)" as out of scope *for that slice*, and calls the narrower record an unresolved divergence (`:131-160`).
   - `R/knowledge/decisions/adr-147-provisioning-verdict-effect-route.md:136-138,248,271,286` says "native HS-authenticated AS transactions/user queries and registration-file generation … remain required".
   - Status: **MISSING**, not CHANGED-ON-PURPOSE.

---

## Inventory

| # | Operation | How in TS | How in Rust | Status | Operator friction |
|---|---|---|---|---|---|
| 1 | First-time host install | `install/bootstrap.sh` → `install-full.sh` / `install/install-macos.sh` (`TS/docs/DEPLOYMENT.md:13-84`); checksum-verified release | `R/install/install-native.sh:20-89` (init → render unit → start → `/ready` gate); units at `R/deploy/hagency-native.service`, `R/deploy/io.hagency.native.plist` | PARTIAL | The unit's start command is plain `serve` with no `--agent-driver`, `--palpo-transport` or `--console-assets` (`hagency-native.service:11`, plist `:28-36`), so the installed service runs no agents and no console. You must hand-edit the unit. The binary must already be built. |
| 2 | Initialize state / operator secret | `API_TOKEN` prompt into `.env` (`TS/install-full.sh:284-296`) | `hagency init --state-dir` writes `operator.token` (`RS/main.rs:216-230`) | PORTED | `init` refuses a non-empty directory, so secrets can only be placed after init, and then the service needs a restart. |
| 3 | Configure the Matrix host / driver | `.env` keys (`TS/docs/E2E-RUNBOOK-macos.md:101-118`; `OPERATOR-WALKTHROUGH.md:265-289`) | Hand-written `agent-driver.json` (`RS/bootstrap/config.rs:393-406`) plus `matrix.access_token` and `matrix.sdk_key` (32 bytes) (`:589-593`) and `matrix.ca.pem` | MANUAL-ONLY | Every mistake collapses into one `Failure::Config` → `"config"` (`bootstrap.rs:693`). Only JSON parse errors log a line and column (`config.rs:407-418`). The fingerprint must be computed by hand (see the confirmation section). The only reference for these files is `R/docs/design/native-execution-parity.md:132`. |
| 4 | Register fleet (registration row) | `POST /api/project-sides` upsert (`TS/backend-v2.js:9862`) | `hagency registration register --file` (`RS/bootstrap/registration.rs:16-59`); console `POST /console/api/project-sides` (`console/project_sides.rs:100`) | PORTED | Six fields only. The CLI takes the exclusive state lock (`hagency-store/src/database.rs:39` → `Locked`), so the service must be stopped. |
| 5 | Onboard project side: set credential | `PUT /api/project-sides/:id/credential` (`TS/backend-v2.js:9881`); console | Hand-place `matrix.registration_token` or `matrix.appservice_token`, plus `matrix.provisioning_key`, `matrix.representative_token`, and the profile and `namespace_prefix` in JSON (`config.rs:213-229,642-681`) | MANUAL-ONLY | See the confirmation section. |
| 6 | Issue appservice registration | `POST …/registration-file` (`:10031`), `POST …/registration` (`:10170`) | none | MISSING | You must write the YAML yourself, generate `as_token`/`hs_token` yourself, and keep them in sync with the state file. ADR-147:136-138 says this is still owed. |
| 7 | Install + verify registration (staged promote, reach diagnostics) | `POST …/verify` (`:9918`); `GET /api/matrix/reach` (`:9698`); `TS/docs/FOR-PROJECT-SIDES.md:40-60` | none; project-side fields reported as `unavailable` (`console/project_sides.rs:33-47`) | MISSING | The only feedback is `/ready` failing with `config`, `registration` or `refresh`. |
| 8 | Appservice inbound (edge / sync) | `bin/hagency-appservice-edge`, `HAGENCY_APPSERVICE_SYNC_*` (`FOR-PROJECT-SIDES.md:95-156`) | No AS receiver. The inline AS profile logs one AS device in and uses SDK sync (`R/specs/task-rust-inline-appservice-account.spec.md:12-24`) | PARTIAL | No edge and no `--check`. ADR-147:248 says the AS receiver is owed. |
| 9 | Per-side token allocation | `PUT /api/project-sides/:id/allocation` (`:9541`), `GET …/budget` (`:9567`) | none (budget exists per resource/seat only) | MISSING | Spec out of scope (`task-rust-project-side-registration.spec.md:248-250`). |
| 10 | Side projects / knock / deactivate / reactivate / delete | `:10359`, `:10375`, `:10402`, `:10464`, `:10474`, `:10584` | none (the collector only watches knock membership, `hagency-matrix/src/collector.rs:565`) | MISSING | Changing a side means editing files and bumping the generation. |
| 11 | Mint agent Matrix identity | `POST /api/agents/:name/matrix-identity` (`:10264`) | Inline token provisioning during approval (`config.rs:642-700`), or `hagency provision existing --file --observer-token --agent-token` (`RS/bootstrap/provision.rs:32-45,525-575`) | PARTIAL | Adoption needs an externally created account, two token files and a strict JSON document. Offline only (state lock). |
| 12 | Configure owner / approver | `HAGENCY_OWNER_MXID` and `HAGENCY_OWNER_DM_ROOM` env, or bridge auto-binding (`OPERATOR-WALKTHROUGH.md:299-332`) | `approval` block in `agent-driver.json` (`config.rs:244-258`), `approval.access_token`, `approval.sdk_key`, `approval.ca.pem` (`:769-790`), and `peer_masters {user_id, master_key}` typed in by hand (`:269-272`) | MANUAL-ONLY | You must look up cross-signing master keys out of band. |
| 13 | Palpo fleet transport | `credential.transport` on the side (`TS/lib/project-side-store.js:146`, `lib/fleet-outbound-config.js:5-14`) | `palpo-transport.json`, `palpo.machine_token`, `palpo.ca.pem` (`RS/bootstrap/palpo.rs:28-60`) and `serve --palpo-transport` | MANUAL-ONLY | The unit doesn't pass the flag (row 1). |
| 14 | Add Codex account / seat | `PUT /api/seats/:seatId` (`:15772`), `POST /api/framework-presets` (`:15843`) | `hagency account prepare\|inspect\|retire` (`RS/bootstrap/accounts.rs:13-60`); console `accounts`, `accounts/{id}/retire`, `accounts/{id}/enrollment` (`console/accounts.rs:29-35`); `POST /api/native/v1/seats` (`RS/resources.rs:19`) | PORTED | Goes beyond TS (ADR-114, Accepted). The CLI needs the service stopped; the console needs `--manage-account-enrollment`. |
| 15 | Log in a provider account | No product step: `codex login` or `hermes auth add` in the agent's HOME (`TS/docs/agent-onboarding.md:93-97`) | `hagency account login --id --login-binary [--device-auth]` (`accounts.rs:27-42,61-111`) | PORTED | New in Rust (ADR-114). CLI only, offline because of the lock; no console route (`console/accounts.rs:42-43`). An unmanaged home can be bound by hand through `local_codex` in JSON (`native-execution-parity.md:132-148`). |
| 16 | Create / edit / delete resource (preset) | `POST`/`PUT`/`DELETE /api/framework-presets` (`:15843`, `:15900`, `:15960`); agent definitions (`:15830-15832`) | Console `POST resources` (`console/resources.rs:11-16`), `PATCH resources/{id}/configuration` (`console/resource_configuration.rs:18-23`); `POST /api/native/v1/resources` (`RS/resources.rs:13-16`) | PARTIAL | No delete. The CLI `hagency resources` is read-only (`main.rs:133-144`). |
| 17 | Publish resource / role | `PUT …/catalog` (`:15833`), `PUT /api/offers/:role` (`:15345`) | Console `POST resources/{id}/publication` (`console/resources.rs:18`); `POST /api/native/v1/roles/{role}/publication` (`RS/resources.rs:22`) | PORTED | Needs a separate `--manage-resource-publication` session. |
| 18 | Create engagement | `POST /api/engagements` (`:14994`); `!request` in Matrix (`OPERATOR-WALKTHROUGH.md:33`) | Matrix provisioning intake in the reception room (`config.rs:605-620`); `provision existing` (`provision.rs:546-549`) | PARTIAL | ADR-147:12-19: an approved request does not yet complete physical provisioning. |
| 19 | Approve engagement | `POST /api/engagements/:id/verdict` (`:15161`), a console click | Owner verdict only through the Matrix approval bot (`hagency-matrix/src/intake.rs:358`); the console can only refuse (`RS/console/agents.rs:235-245`) | PARTIAL | No approve from console or CLI; approval depends on the setup in row 12. |
| 20 | Revoke engagement / retire agent | `POST …/revoke` (`:15222`), `DELETE /api/agents/:name` (`:12165`), `hagency acp-down`, `prune-agents` (`TS/bin/hagency-prune-agents:35-47`) | Console `engagements/{id}/retire`, `engagements/{id}/cleanup-retry` (`console/engagements.rs:26-29`) | PARTIAL | No CLI and no bulk prune; needs a lifecycle-scoped console session. |
| 21 | Start agent | `hagency up` / `acp-up` / `up-v1` (`agent-onboarding.md:17-31,70-78`); `POST /api/agents/:name/start` (`:12713`) | Console start returns 501 `agent_start_unavailable` (`console/agents.rs:172-195`) | MISSING | ADR-130:50-55 says "unavailable until" a real transition exists (a deferral, not a change). Agents run only per dispatch under `--agent-driver`. |
| 22 | Stop agent | `hagency down` (`TS/bin/hagency-down:66-74`), `acp-down`, `POST …/stop` (`:12708`) | Console `POST agents/{id}/stop`, which fences the dispatch and reports `stop_pending` (`console/agents.rs:197-199`; ADR-130:45-48) | PARTIAL | Console only; the stop is not settled by the route. |
| 23 | Restart / recover agent work | The supervisor restarts it (`agent-onboarding.md:23-31`); `hagency service restart` | `recover-dispatch`, plus `stopped-dispatches` inspect / resolve / continue (`console/agents.rs:33-44`; ADR-148, ADR-164, ADR-165) | PARTIAL | Works per dispatch, console only. |
| 24 | Change agent preset | `PUT /api/agents/:name/preset` (`:11484`) | 501 `agent_preset_unavailable` (`console/agents.rs:614-635`) | MISSING | Retire and reprovision instead. |
| 25 | Service start / stop / restart / status | `hagency service pause\|resume\|restart\|status` (`TS/bin/hagency-service:465-485`); `hagency-services.mjs status\|doctor` (`DEPLOYMENT.md:98-106`) | Raw `systemctl` / `launchctl` only (`install-native.sh:69-86`; plist `:9-14`) | PARTIAL | Every offline CLI step (rows 4, 11, 14, 15) needs the service stopped first. |
| 26 | Rotate operator / API token | Edit `API_TOKEN` in `.env` and restart | none: "`operator.token` has no rotation" (`R/knowledge/decisions/adr-135-two-host-cutover-runbook.md:189-197`) | MISSING | Losing the token means a new state directory and re-registering everything. |
| 27 | Rotate Matrix / appservice / Palpo credentials | Staged reissue, then verify promotes it automatically (`TS/backend-v2.js:10054-10124,9934-9953`; `FOR-PROJECT-SIDES.md:40-60`) | Overwrite the private files, bump `transport_generation` / `registration_generation` / `machine_generation` in JSON (`config.rs:196-211`; `palpo.rs:20,46-53`), then restart | MANUAL-ONLY | No staging and no verify, so a mistake is live immediately. |
| 28 | Rotate agent tokens | `hagency maintain gen-agent-tokens` (`TS/bin/hagency-maintain:49`); `acp-up` provisioning (`TS/bin/hagency-acp-up:125-130`) | No persistent agent token; each dispatch gets a runner capability (`RS/task_client.rs:189-210`) | CHANGED-ON-PURPOSE | ADR-041 (Accepted), lines 14-18. |
| 29 | Back up / restore state | Manual `tar` of `data/` (`TS/docs/ROLLBACK.md:113-125`) | none; a plain copy is unsafe because of WAL and no-checkpoint-on-close ("No backup or `VACUUM INTO` path exists", ADR-135:158-167) | MISSING | No safe backup method exists at all. |
| 30 | Import existing TS state | n/a | Deliberately none (ADR-095:16 "No old router, JSON or SDK store is imported"; ADR-135:206-208) | CHANGED-ON-PURPOSE | Cutover is a fresh install; everything is re-registered. |
| 31 | Upgrade / rollback | `./upgrade.sh --to/--list` with health-check auto-revert (`DEPLOYMENT.md:230-237`; `upgrade.sh:9-16`); `hagency update` (`TS/bin/hagency-update:89-104`); auto-deploy watcher | Install version N+1, point the unit at it, restart (ADR-134:102-113) | MANUAL-ONLY | `install-native.sh` refuses a non-empty state directory (`:42-43`), so it can't be used to upgrade. No auto-revert. |
| 32 | Health / readiness | `/health` (`:7586`), `standalone-doctor.mjs`, `verify-remote`, `hagency check-mcp` (`DEPLOYMENT.md:239-246`) | `GET /health`, `GET /ready` (`RS/lib.rs:151-152`) | PARTIAL | No `status`/`doctor` CLI; use `curl /ready`. |
| 33 | View logs | Per-service files in `data/services-local/logs` (`DEPLOYMENT.md:86-106`) | stderr goes to journald or launchd files (plist `:56-59`); `RUST_LOG` filter (`main.rs:187-192`) | PORTED | Fine on systemd. |
| 34 | Log rotation / maintenance | `hagency maintain` (`TS/bin/hagency-maintain:82-112,303-334`) | none; stderr, with the service manager owning rotation (`R/knowledge/decisions/adr-129-native-recovery-artifact-retention.md:49`, Proposed) | CHANGED-ON-PURPOSE | On macOS, logs are plain files "the operator must rotate" (`adr-133-native-launchd-unit.md:22`). |
| 35 | Inspect alerts / engagements / resources / agents / sides | `/api/alerts` (`:16070`), `/api/engagements` (`:14965`), `hagency ls` (`TS/bin/hagency-ls:69-73`), `hagency cli status\|fleet` (`TS/bin/hagency-cli:270,461`) | `hagency alerts\|engagements\|resources` (`main.rs:106-144`); console GET for agents and project-sides | PARTIAL | No CLI for agents or sides (ADR-132:77 "Deferred, named: … any CLI read"). |
| 36 | Alert handling | Transition, notes, patch, delete (`:16115-16151`) | `POST alerts/{key}/transition` (`RS/alerts.rs:14-16`; `console/alerts.rs:26-28`) | PARTIAL | No notes or delete. |
| 37 | Execution grants / policy | `GET`/`PUT …/execution-policy`, `DELETE …/execution-grants/:id` (`:10865-10888`) | Console approvals list, and `DELETE approvals/grants/{id}` (`console/approvals.rs:23-25`) | PARTIAL | No policy edit. |
| 38 | Grant console access | Next.js proxy with `API_TOKEN` held server-side (`E2E-RUNBOOK-macos.md:9`) | `hagency console-access [--manage-…]` prints a 120-second ticket that becomes a 15-minute session (`main.rs:73-105`; `console/authority.rs:16-17`) | PORTED | Only one scope per session (`main.rs:86-104`), and it needs `serve --console-assets`, which the unit doesn't pass. |
| 39 | Revoke console access | Rotate `API_TOKEN` | `DELETE /console/session` revokes only your own session (`RS/console.rs:67,352-358`); a restart clears the in-memory sessions | PARTIAL | No command to revoke every session. |
| 40 | Group / room admin | `hagency cli create-group\|add-member\|tell\|dm` (`TS/bin/hagency-cli:134-183,544-587`); `!mkgroup` / `!bindroom` | none | MISSING | — |
| 41 | Remote agent host / relay | `remote/install-remote.sh` (`DEPLOYMENT.md:195-206`), `verify-remote` | none (`R/docs/design/hagency-rust-migration-plan.md:96` is only a target) | MISSING | — |
| 42 | Uninstall | `TS/uninstall.sh` | Manual `disable --now` and remove the unit (`adr-127-native-systemd-unit.md:81`) | MANUAL-ONLY | — |

The Rust-only recovery command `hagency intake-refuse-stale-session` (`main.rs:66-72`) has no TS counterpart and isn't counted.

---

## (1) Counts per status (42 rows)

| Status | Count |
|---|---|
| PORTED | 7 |
| PARTIAL | 15 |
| MISSING | 10 |
| MANUAL-ONLY | 7 |
| CHANGED-ON-PURPOSE | 3 (ADR-041, ADR-095/135, ADR-129) |

## (2) Top 15 gaps, ranked by operator friction

1. **Issuing an appservice registration is missing** (row 6): you must write the YAML and generate the tokens yourself; ADR-147:136-138 says this is still owed.
2. **The project-side credential is hand-placed** (row 5): `matrix.appservice_token` or `matrix.registration_token`, plus `provisioning_key`, `representative_token` and `namespace_prefix` in JSON (`config.rs:642-681`).
3. **Registration and credential verification are missing** (row 7): the only signal is an opaque `/ready` `"config"` or `"registration"` error.
4. **`agent-driver.json` is hand-authored** (row 3): the fingerprint must be hand-computed and all errors collapse into `"config"` (`bootstrap.rs:693`).
5. **The installed unit runs plain `serve`** (row 1): no agent driver, Palpo or console flags, so you must hand-edit it.
6. **Approver setup needs cross-signing master keys typed into JSON** (row 12), along with separate approval token and key files.
7. **There is no safe backup or restore** (row 29): a plain copy is unsafe with WAL (ADR-135:158-167).
8. **The operator token can't be rotated** (row 26): losing it means rebuilding the state directory (ADR-135:189-197).
9. **Credential rotation is manual** (row 27): overwrite files, bump generations, restart; no staging or verify step.
10. **Offline CLI commands need the service stopped** (rows 4, 11, 14, 15, 25): the state-directory lock (`database.rs:39`) means every account login or registration costs downtime.
11. **Agents can't be started** (row 21): 501 `agent_start_unavailable`; preset change is also 501 (row 24).
12. **Engagements can't be approved from console or CLI** (row 19): it depends on the Matrix approval bot, and approval doesn't complete provisioning (ADR-147:12-19).
13. **Upgrades are manual with no auto-revert** (row 31), and the installer refuses existing state.
14. **Per-side allocation and side lifecycle are missing** (rows 9 and 10): allocation, knock, deactivate, reactivate, delete, projects.
15. **Day-2 CLI coverage is thin** (rows 25, 32, 35, 38): no `status`/`doctor`, no agent or side listing, one console scope per session, and no remote-host (row 41) or group-admin (row 40) commands.