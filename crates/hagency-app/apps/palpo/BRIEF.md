# Palpo Operations in Rinx

Implementation of ADR 0011's coordinator workflow, using Design Flow's script-app authoring loop.
The new native host service and backend are implemented in their owning
repositories before the bundle calls them. No new service is invented in Splash.

## Screens and actions

- Inbox: Needs my action, Waiting, History; refresh, inspect current revision,
  approve/reject with a reason, continue approved work, snooze reminders.
- Hagency owns contributions: resource setup is removed from the mini app.
  Existing contribution drafts cannot be resumed through the new workflow.
- Projects: list accessible projects; choose an offered resource, request a
  project, activate it after approval, request a named agent with an allocation.
- Resources: browse server-published roles and resources; keep unavailable or
  stale observations explicit.
- Fleets: owner export through native file UI and connection verification;
  administrators register/install/pause/resume/revoke, inspect outbound queue,
  migrate transport, and manage Matrix agent identities.
- Activity and signup requests: current administrator only. Navigation follows
  the services the server actually grants, so Rust migration does not offer
  unsupported operations. Coordinator status is independent of Matrix admin.
- Agent decisions use the authenticated coordinator and a frozen server request;
  approval is displayed separately from execution and live readiness.
- The Rust service grants Projects and Agents reads with pagination. Agent
  cards distinguish pending allocation, unknown consumption and stale reports.
  Metered usage is a lower bound, not an exact remaining balance. Unimplemented
  creation controls remain hidden even when read access is available.
- Owners can request additional tokens from a current agent allocation. The
  coordinator reviews the exact amount in Inbox. Tokens remain unchanged until
  Hagency reports the applied top-up; stable request IDs survive retries.

## Data and authority

The signed-in Rinx Matrix account is the only identity. Session credentials and
fleet configuration never reach the bundle. Every operation uses an exact
`palpo.*` service from the shared contract. There is no `net` capability and no
network host in the bundle. `storage` keeps only forms and stable operation IDs
in Rinx's account-specific app jail, allowing retries after closing the app.
The live server, not client visibility, authorizes roles, ownership and decisions.

Empty lists explain the next action. Loading prevents duplicate clicks. Failures
retain drafts and operation IDs. Old cards fetch the current record. Opening or
reading a task never completes it. Submitted actions live in Palpo SQLite.
Theme reapply preserves navigation, form text and IDs without submitting work.

## Validation

Drive the production Splash bundle through Makepad remote instrumentation in a
hidden Rinx host example, using the real host adapter and local Palpo backend
with explicit Matrix fixtures. Check manager/coordinator/admin roles, agent decisions,
conflict/denial, empty/error/restart, narrow layout and live
light/dark/custom theme changes. Test credential export separately at the trusted
host boundary. Do not touch the user's profile or deployed Palpo. No mobile
platform or live multi-account acceptance claim without device evidence.

App Hub publication, publisher identity, signing and platform claims are outside
this development delivery; no publisher identity is fabricated.

Current Rust integration test:

```sh
cargo build --profile fast --locked --example palpo_miniapp
python3 tools/wechat-ux/live/native_palpo.py \
  --backend /path/to/palpo/target/debug/palpo-operations \
  --binary target/fast/examples/palpo_miniapp
```

This uses the real Rust backend process and its SQLite store, with fixture
Matrix authentication and an already verified project. It does not claim the
association/profile-download, project-room preparation or live Hagency/chat flow.
