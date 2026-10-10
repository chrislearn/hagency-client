[English](README.md) | [中文](README.zh-CN.md)

# Hagency client

Run your own Codex Agents in Matrix Rooms through `hagency-server`. You sign in
with your personal Matrix account through Pasion. The server creates each Agent's
puppet account and records its permanent owner; your local client controls model
credentials, budgets, requester rules and tool permissions. Server administrators
control where members may create Agents, without approving your local resource use.

A Project corresponds to a Matrix Space; each Room has independent membership.
An Agent can have multiple authorized Room bindings and can never be transferred
to another owner. The integrated server Appservice is required. Clients do not
receive its secret or provision puppet accounts themselves.

## Quickstart

Use a new private state directory and a current binary containing the **owner
console**, not a retained Fleet console build. You need an accessible configured
`hagency-server`/Palpo/Pasion deployment, your personal account, and an installed
Codex executable compatible with the current app-server 0.160 adapter and an
available OS credential keyring. The listener must be a nonzero loopback address.

```sh
hagency init --state-dir "$HOME/.local/share/hagency-owned-client"
hagency start --state-dir "$HOME/.local/share/hagency-owned-client"
```

`start` runs the local OwnerHost and opens the console from an interactive terminal.
It does **not** automatically start Agents or call a model. If the browser did not
open, run this in another terminal:

```sh
hagency open --state-dir "$HOME/.local/share/hagency-owned-client"
```

1. In the console, choose your Hagency server and sign in through Pasion's official
   browser authorization flow using your own Matrix account. A local console
   access link is not server authorization. Do not paste AS credentials or use
   another person's account.
2. In Project management, create a private Space, then select its Project and create a discussion Room. You may also register existing Spaces/Rooms you have joined and can administer. The client uses your Matrix authorization to create and link them; Room membership remains independent. If a creation outcome is uncertain, inspect and resume its original command. Select or create your Agent and binding; the server checks current membership and creation policy. An
   identity may be pending until durable Matrix provisioning finishes. Select an
   **active** binding before starting its runtime.
3. Use **Sign in to my ChatGPT / Codex account** and complete the official provider
   login. Codex stores this owner's credentials in its separate OS keyring/home.
   The client does not import your ordinary `~/.codex` login or send provider
   credentials to Hagency server. Use the verified credential reference in the
   local model profile, choose a model and an existing private workspace directory.
4. Configure budgets at all three levels: **Agent**, **Room binding**, and **Room
   requester**. Unset is not unlimited. Choose explicit token limits or Unlimited,
   and a Lifetime, UTC day or UTC month period. Set whether each requester is
   allowed, denied or requires an owner decision.
5. Explicitly accept **Estimated** mode and specify a positive token reservation.
   Current Codex does not expose an enforceable hard token bound; **Strict is
   unavailable**. Reservations and actual-usage accounting can prevent subsequent
   work but cannot guarantee one model call never overruns its estimate.
6. Start Codex for the selected Room explicitly. Keep restricted file tools off
   unless you want them. Review runtime state and stop the selected Room when done.
   An Agent can explicitly run multiple Rooms under one device lease. Start/stop,
   budgets and context remain separate; unstarted Rooms do not execute.

The shipped host supports three optional Room-workspace tools:
`hagency_file_list`, `hagency_file_read` and `hagency_file_create`. They are disabled
by default. Create writes a new UTF-8 file; it cannot replace an existing file.
Native shell, MCP, plugins/hooks and general file-editing are not exposed by this
host. Allowing a tool does not enable these unsupported capabilities. Owner AskOwner request/tool decisions use local exact approval. Approval binds the original request or full arguments, policy revisions and expiry; live Room authority is rechecked before execution.
Denied or unanswered authorization is never permission to run.

Encrypted Rooms are explicitly deferred in the new architecture; there is no
plaintext downgrade. Existing SDK crypto code is retained for adaptation. See the
server [E2EE capability gap report](../hagency-server/docs/CRYPTO_CAPABILITY_GAPS.md)
for the required restricted proxy, reliable crypto sync and interoperability work.

## Process and service commands

```sh
# Foreground listener without opening a browser.
hagency serve --state-dir "$HOME/.local/share/hagency-owned-client" --listen 127.0.0.1:13300
# Foreground start without browser opening.
hagency start --state-dir "$HOME/.local/share/hagency-owned-client" --no-open
# Print a running console access link without opening it.
hagency open --state-dir "$HOME/.local/share/hagency-owned-client" --no-open
# Install and start a per-user LaunchAgent or systemd --user service.
hagency service install --state-dir "$HOME/.local/share/hagency-owned-client" --no-open
# Stop/remove that service; state is retained.
hagency service uninstall
```

Run only one host against a state directory. Stop a foreground instance before
installing its service. The service runs `start --no-open` as your OS user; Agents
still require explicit owner start. Without `--state-dir`, macOS uses
`~/Library/Application Support/HagencyOwnedClient`; Linux uses
`$XDG_DATA_HOME/hagency` when set, otherwise `~/.local/share/hagency-owned-client`.
Use the same explicit directory for Init, Start, Open and Service when in doubt.

The new marker is `hagency-client-owned-v1.json`. An old or foreign directory,
missing/wrong marker or unexpected entries is refused without importing old
SQLite/config/credentials. Keep old data separately if needed; do not copy it into
new state, edit the marker or follow old migration/import instructions. There is
no compatibility runtime switch.

## Build from source

From this repository root, with Rust at least the declared 1.98 and Node/npm:

```sh
npm ci --prefix mockup
console_parent="$(mktemp -d)"
node mockup/scripts/build-native-console.mjs --output "$console_parent/owner-console"
HAGENCY_CONSOLE_DIR="$console_parent/owner-console" cargo build --locked --release -p hagency
export PATH="$PWD/target/release:$PATH"
```

The build output directory must be new. This script stages only the owner console
routes; `npm run dev` in `mockup` is not the production OwnerHost. For development,
`start`/`serve` accept `--console-assets /absolute/path/to/owner-console`; service
installation expects a binary with the correct embedded console. A binary without
an owner console fails explicitly rather than falling back to old assets.

See [the executable guide](native/hagency/README.md) for command boundaries and
[the implementation report](docs/design/2026-10-07-server-appservice-client-refactor.zh-CN.md)
for the reviewed refactor. Retained Fleet/Engagement/resource approval design
records and old architecture guides describe historical implementations; they are
not setup instructions for this executable.

## License

Apache 2.0; retain [LICENSE](LICENSE) and [NOTICE](NOTICE) on redistribution.
Hagency is a fork of agent-chat; [licensing](docs/LICENSING.md) records attribution.

See [local development](docs/local-development.md) and [personal server login](docs/server-login.md).
