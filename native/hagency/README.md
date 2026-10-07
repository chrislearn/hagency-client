[English](README.md) | [中文](README.zh-CN.md)

# The owner-client executable

This crate builds `hagency`. Its production entry point is `owner_host::OwnerHost`,
with personal Pasion authorization and explicitly started, local Codex runtimes.
Read the [root quickstart](../../README.md) for build, login and policy steps.

| Command | Actual behavior and flags |
| --- | --- |
| `init --state-dir PATH` | Initialize the new private owner-client format; required state path |
| `start` | Serve the owner console and open it from an interactive terminal; optional `--state-dir`, `--listen`, `--no-open`, `--console-assets` |
| `serve` | Serve without browser opening; optional `--state-dir`, `--listen`, `--console-assets` |
| `open` | Ask the running host for a local console access link over private IPC; optional `--state-dir`, `--no-open` |
| `service install` | Install/start a per-user macOS LaunchAgent or Linux systemd user unit; optional `--state-dir`, `--listen`, `--no-open` |
| `service uninstall` | Stop/remove the per-user service, retaining state |

When no subcommand is provided, `start` is used. The default listener is `127.0.0.1:13300`.
Non-loopback or zero-port listeners are rejected. One process exclusively locks
one state directory; foreground serve and service cannot share it concurrently.
Installing a service runs `start --no-open`, not an automatic model turn. Agents
require an explicit owner start after authenticated configuration.

Fresh state uses `hagency-client-owned-v1.json`. Old Fleet SQLite/configuration,
wrong markers and foreign entries are rejected, without reading or importing
those bytes. Do not change a marker to reuse an old directory. There are no
`setup`, enrollment, authority import, guardian/MCP subcommands or legacy runtime
switches. Old modules retained elsewhere are not production CLI entry points.

The owner console is either embedded at build using `HAGENCY_CONSOLE_DIR` or
provided to start/serve with `--console-assets`. It must be the manifest-checked
owner bundle created by `mockup/scripts/build-native-console.mjs`; missing or
legacy bundles fail rather than being served as a fallback. Service installation
has no `--console-assets` flag: use the correctly embedded release binary.

Console access and server authority are separate. Use your personal Matrix
account in Pasion's browser OAuth/PKCE flow. The server permanently owns the
Agent-to-human mapping; AS credentials never reach the client. Codex onboarding
uses a separate owner-scoped OS keyring/home, not an imported everyday Codex
login, and never uploads provider credentials to the server.

The client requires Matrix sign-in before showing Agent controls. The sign-in
page offers saved account profiles and recent server addresses, with an automatic
device name. Switching stops the previous account's tasks and requires fresh
Pasion authorization. Budgets, ledgers, recovery records and Codex resources are
isolated by account; every Agent retains its original creator. Server history
stores addresses, never passwords or tokens. Sign-in does not start inference.

Agent, Room-binding and requester budgets apply together. Codex only supports
explicitly accepted Estimated reservations/actual usage accounting here; Strict
hard caps fail explicitly. Tools default to disabled. The current host's optional
file surface is list/read/create-new UTF-8 files in the selected Room workspace;
it does not enable shell, MCP, hooks/plugins or replacement of existing files.
AskOwner request/tool decisions use exact local approval of one original request or call; current policy and Room authority are rechecked before execution.

Encrypted Rooms remain deferred. See the server
[capability gap report](../../../hagency-server/docs/CRYPTO_CAPABILITY_GAPS.md).
Unknown execution/usage and ambiguous replies require recovery; restarting or
acquiring a new lease does not authorize rerunning old work or erase its charges.
