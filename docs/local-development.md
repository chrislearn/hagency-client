[English](local-development.md) | [中文](local-development.zh-CN.md)

# Local owner-client/server development

Keep `hagency-client`, `hagency-server` and optionally `hagency-desktop` as sibling
checkouts. Each selects its own Rust toolchain. The owner console requires Node.js
22+ and npm; the local HTTP host is Rust. All client commands below run in this repo.

## Server prerequisites

Follow [server setup](../../hagency-server/README.md), then use `just db-up` and
`just dev` in the server checkout. Default origin is `http://127.0.0.1:8088`;
web management is `/login`. Pasion delegated Matrix authentication and the
integrated Agent Appservice are required. There is no Fleet enrollment or
`[fleet_access]` configuration. See [deployment diagnostics](../../hagency-server/docs/LOCAL_DEPLOYMENT.md)
and [isolated tests](../../hagency-server/docs/TESTING.md).

## Build the current owner host

```sh
npm ci --prefix mockup
mkdir -p .run
console_parent="$(mktemp -d "$PWD/.run/owner-console.XXXXXX")"
node mockup/scripts/build-native-console.mjs --output "$console_parent/assets"
HAGENCY_CONSOLE_DIR="$console_parent/assets" cargo build --locked -p hagency
# First initialization only: choose a fresh owner-format state directory.
target/debug/hagency init --state-dir "$PWD/.run/owner-dev-state"
target/debug/hagency start --state-dir "$PWD/.run/owner-dev-state" \
  --listen 127.0.0.1:13300 --console-assets "$console_parent/assets"
```

The assets output path must not already exist. Keep the assets while this host
runs. For later starts skip `init`; in another terminal obtain a private access link:

```sh
target/debug/hagency open --state-dir "$PWD/.run/owner-dev-state"
```

`open` talks to the running host through local IPC; it does not need a listener
argument. Do not save private access links in logs. For terminal-only operation
use `start --no-open` or `serve` with the same state/listener/assets. Only one host
may own a state directory. The listener must be nonzero loopback.

The retained `just dev` / `just console` wrappers are not current owner-host
instructions: `native/scripts/dev.mjs` still passes removed `--palpo-transport`
and checks the old `operator.token`; `just console` calls removed
`console-access`. Use the explicit commands above until those wrappers are ported.
`just init-dev` remains an npm dependency-install shortcut. `npm run dev` in
`mockup` is a design preview, not a live OwnerHost.

Rebuild exported owner assets after console edits and restart the host with the
new bundle; rebuild Rust after engine edits. For an existing validated font cache,
pass `--font-cache /absolute/path/to/cache` to the console builder if font fetching
is unavailable. That cache is build input, not authentication or runtime state.

## Sign in and configure

Enter the server origin and complete personal Pasion PKCE login. Project maps to
one Space; discussion Rooms retain independent membership. Create/adopt authorized
Spaces/Rooms, select an Agent and active binding, then configure owner-scoped Codex
credentials, model, private workspace, Agent/Room/requester budgets and exact tool
policy. Explicitly accept Estimated accounting and a positive reservation before
starting a scope. Login/server connection alone does not invoke a model.

The CLI host uses a separate owner-scoped Codex home/keyring. Desktop can also
explicitly associate an already signed-in local Codex account. Neither sends model
credentials to the server. Read [server login](server-login.md), the
[root guide](../README.md) and [Desktop development](../../hagency-desktop/docs/local-development.md).

Fresh CLI state has `hagency-client-owned-v1.json`. Keep old Fleet state separate;
do not copy old config/SQLite/tokens or edit markers to bypass initialization.
Unknown execution/costs retain their recovery holds. Encrypted Agent scopes are
currently deferred; ordinary Desktop human Matrix encryption is separate.

## Verification

```sh
cargo test --locked -p hagency --lib
cargo clippy --locked -p hagency --all-targets -- -D warnings
git diff --check
```

These are local engine checks. Complete-host Pasion/Matrix fixtures and native
Desktop checks have their own guides above. Real Codex execution requires explicit
start and consumes provider resources; do not report protocol fixtures as inference.
