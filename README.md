[English](README.md) | [中文](README.zh-CN.md)

# Hagency

**Lend Codex agents to projects on a Palpo Matrix server, with owner approval and token budgets.**

Hagency is one Rust service, `hagency serve`. A provider publishes resources (model, reasoning effort, monthly token ceiling) to a connected Palpo homeserver. Projects define agents on those resources and request tokens. The provider approves an amount, and Hagency gives the agent its own Matrix identity in the project's room. People then @-mention the agent to give it work, and approve its risky operations from a private encrypted room.

**Using Hagency through a Palpo server?** Start with the [user guide](docs/user-guide/README.md).
**Changing the service?** Start with the [code walkthrough](docs/architecture-walkthrough.md).

## Contents

| Section | |
| --- | --- |
| [What it does](#what-it-does) | The capability surface |
| [Architecture](#architecture) | One process, its threads and its crates |
| [Build and install](#build-and-install) | From source, as a systemd or launchd service |
| [First run](#first-run) | Console access and connecting Palpo |
| [Operating](#operating) | Health, logs, backup, credential rotation |
| [Configuration](#configuration) | The state directory and `agent-driver.json` |
| [Security posture](#security-posture) | What is enforced, and what is assumed |
| [Development](#development) | Tests and CI gates |

## What it does

- **Resources and the catalog.** The provider configures resources in the console. New resources are published to Palpo automatically; ceilings, seats and internal ids stay private.
- **Project-defined agents.** A project member defines an agent on a published resource in Palpo web, with requested tokens and a daily rate. Each definition becomes an engagement awaiting the provider's decision.
- **Approval with an allocation.** The provider approves an amount, up to "all remaining", within the resource's ceiling, seat and pool headroom. When the agent uses its allocation it pauses without dropping work, and resumes when tokens are added.
- **Provisioning.** Approval creates the agent's Matrix account through the App Service and joins it to the project room. It also creates an encrypted DM with the owner, uploading the agent's keys before the owner is invited.
- **Mention-driven work.** In the project room an agent acts only when a person mentions it. It reads the surrounding discussion as context and replies in the conversation's thread. In its DM, every message from the owner reaches it.
- **Owner approvals.** Commands and file changes outside the sandbox, coordination tools and file transfers go to the owner as cards in a private encrypted approval room. Only a verified verdict from the owner's own device counts; an unanswered card is denied.
- **One console.** Resources, engagements, project sides, approvals, invitations, tasks, usage and alerts, served by the same binary on a loopback port.

## Architecture

```text
Palpo homeserver  <── outbound HTTPS ──  hagency serve (127.0.0.1:13300)
                                           ├─ Palpo long poll: requests, probes, catalog, statuses
                                           ├─ per-agent driver threads: Matrix /sync, intake, replies
                                           ├─ domain + custody SQLite, one writer thread each
                                           ├─ console and operator API
                                           └─ per dispatch: hagency guardian ─> codex app-server ─> hagency mcp
```

- **Outbound only.** `hagency serve` opens no port to the outside. It long-polls Palpo's fleet API and polls Matrix `/sync` for each agent, so it runs behind NAT.
- **Single binary.** The same executable is the daemon, the guardian that owns each runner's process tree, the MCP helper that gives Codex its task tools, and the operator CLI.
- **Crates.** The Rust workspace lives in [native/](native/). `hagency-core` holds the domain types, `hagency-store` the durable rules, `hagency-matrix` the Matrix side effects, `hagency-palpo` the fleet transport, `hagency-execution` and `hagency-runtime` the Codex runs, and `hagency-platform` process supervision.

The [code walkthrough](docs/architecture-walkthrough.md) follows each flow through the code, with diagrams.

## Build and install

Requirements:

| | |
| --- | --- |
| Rust | The toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml) |
| Node.js 22 | Build time only, to export the console |
| Codex CLI | The runner; `agent-driver.json` names its path and SHA-256 |
| Host | Linux with systemd, or macOS with launchd |
| Palpo | A homeserver whose admin can run "Add Hagency" |

Build the binary and the console:

```bash
cargo build --release --locked -p hagency
(cd mockup && npm ci)
node mockup/scripts/build-native-console.mjs --output /abs/path/console-assets
```

The console output directory must be new and private to you.

Install as a service:

```bash
install/install-native.sh \
  --install-dir /abs/path/bin \
  --state-dir /abs/path/state \
  --console-dir /abs/path/console-assets \
  --config-dir /abs/path/config
```

- **What it does.** The installer runs `hagency init`, which needs an empty state directory and mints `operator.token`. It copies the config files into the state directory with mode 0600, then renders and starts [deploy/hagency-native.service](deploy/hagency-native.service) on Linux or [deploy/io.hagency.native.plist](deploy/io.hagency.native.plist) on macOS. It succeeds only once `/ready` answers 200.
- **Inputs.** `--install-dir` must contain the `hagency` binary. `--config-dir` supplies `agent-driver.json` and the private `matrix.*`, `palpo.*` and `approval.*` files.
- **Refusals.** It refuses an existing unit unless you pass `--overwrite`.

There is no published native release yet. [release-native.yml](.github/workflows/release-native.yml) builds per-target binaries and `SHA256SUMS` on manual dispatch only.

## First run

1. Open the console:

   ```bash
   hagency console-access --state-dir /abs/path/state
   ```

   It prints a link that is valid for 120 s. Opening it exchanges the link for an `HttpOnly` session cookie.
2. Connect Palpo. Follow the [user guide](docs/user-guide/README.md):
   - The Palpo admin runs **Add Hagency**.
   - The owner downloads the configuration and imports it under **Project sides → Connect a Palpo project server**.
   - The owner then verifies the connection in Palpo web.
3. Configure resources in the console. They appear in Palpo, where projects can define agents on them.
4. Approve requests under **Engagements**. The agent joins the project room once provisioning completes.

## Operating

| Task | Command |
| --- | --- |
| Liveness / readiness | `curl -s 127.0.0.1:13300/health` · `curl -s 127.0.0.1:13300/ready` (503 names the component that is not ready) |
| Service status (Linux) | `systemctl status hagency-native` · `journalctl -u hagency-native` |
| Logs (macOS) | `<install-dir>/logs/hagency-native.stdout.log`, `…stderr.log` |
| Log level | `RUST_LOG` (default `info`) |
| Stop (macOS) | `launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/io.hagency.native.plist` (a killed process is restarted by `KeepAlive`) |
| Inspect | `hagency engagements`, `hagency resources`, `hagency alerts` (`--state-dir`, `--json`) |
| Back up (online) | `hagency backup --state-dir <state> --out <new dir>` |
| Restore | `hagency restore --state-dir <empty dir> --from <backup>` |
| Rotate the operator token | `hagency rotate --state-dir <state> operator-token` |

On SIGTERM the service drains in order (fleet, runners, Matrix sessions, then the databases) and stops its HTTP server last. systemd allows it 20 s.

## Configuration

`hagency serve` reads no environment file. Its configuration is the files in `--state-dir`:

| File | Purpose |
| --- | --- |
| `operator.token` | Operator bearer secret, created by `hagency init` |
| `agent-driver.json` | Runner and Matrix settings, described below |
| `matrix.*`, `approval.*` | Access tokens, SDK store keys and CA bundles for the agent and approval-bot identities |
| `palpo-transport.json`, `palpo.machine_token`, `palpo-appservice.json` | Written when the owner imports the Palpo configuration |
| `domain.sqlite3`, `custody.sqlite3` | Durable state (WAL, `synchronous=FULL`); one process per state directory |

`agent-driver.json` is strict JSON: unknown fields are refused. Its main fields:

| Field | Meaning |
| --- | --- |
| `profile` | Driver profile |
| `executable`, `executable_sha256` | The Codex binary and its expected hash |
| `workspaces` | Workspace id → absolute path |
| `operation_ms`, `response_ms` | Per-dispatch operation and response budgets |
| `approval_owner_wait_ms` | How long a card waits for the owner before it is denied. The default is 1000 ms, so set it |
| `send_file`, `receive_file`, `file_limit` | Enable the file tools, and the received-file size limit |
| `coordination_tools` | Enable delegation and peer tools, each call owner-approved |
| `matrix`, `approval` | Homeserver origin, identities, devices and rooms for the agent and the approval bot |
| `factory_service` | The coordinator that provisions approved agents |

The authoritative definition is `Config` in [native/hagency/src/bootstrap/config.rs](native/hagency/src/bootstrap/config.rs).

## Security posture

What Hagency **enforces**:

- **Loopback only.** `hagency serve` refuses any non-loopback listen address. For remote access, use an SSH tunnel or a reverse proxy.
- **Console requests.** The console requires the exact `Host`, a same-origin `Origin` for writes, and no forwarded headers. The operator API takes a bearer token checked in constant time.
- **Owner approvals.** Approvals come only from the owner's verified device, in an encrypted room whose only members are the owner and the approval bot. A third member or lost encryption disables the room. A failed delivery or an expired wait counts as a deny.
- **Fenced runners.** Each dispatch has a capability and a fence number. A stale runner's calls are refused, and a reply is published only after its process tree is proven gone.
- **Codex sandbox.** Codex runs with `workspace-write` and `on-request` approvals, no network, and no extra writable roots. The settings Codex echoes back are checked.
- **Write-only credentials.** No route returns a stored token; the console shows fingerprints.
- **Hardened unit.** The systemd unit sets `NoNewPrivileges`, `ProtectSystem=full`, an empty capability set and a syscall filter.

What it **assumes**:

- **Shared host trust.** Loopback trust is machine-scoped, so any local process can reach the port. Keep `operator.token` and the state directory private.
- **Group rooms are open to members.** Any joined member who mentions an agent can give it work. Room membership, the allocation and owner approvals are the controls.
- **No federation.** All members must be on the fleet's own server.
- **Sandbox qualification is open.** The sandbox is requested and checked, but per-OS qualification of its effect is still open.

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

Behaviour is bound to tests through task contracts in [specs/](specs/), whose decisions are recorded in [knowledge/decisions/](knowledge/decisions/). CI ([rust.yml](.github/workflows/rust.yml)) also runs:
- `node native/scripts/check-rust-spec-bindings.mjs`: every spec selector names a real test.
- `node native/scripts/check-production-callers.mjs`: every `Production caller:` line resolves in the production call graph.
- The console browser tests.

The walkthrough's [Making a change](docs/architecture-walkthrough.md#14-making-a-change) section says where new rules, migrations, agent tools and console routes go.

## Documentation

| Document | Covers |
| --- | --- |
| [docs/user-guide/README.md](docs/user-guide/README.md) | Connecting Palpo, rooms, who can talk to an agent, tokens |
| [docs/architecture-walkthrough.md](docs/architecture-walkthrough.md) | The code, flow by flow |
| [knowledge/decisions/](knowledge/decisions/) | Architecture decision records |
| [specs/](specs/) | Task contracts bound to tests |
| [docs/LICENSING.md](docs/LICENSING.md) | Fork provenance and Apache 2.0 attribution obligations |

## License

**Apache License 2.0**: see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).

Hagency is a fork of [agent-chat](https://github.com/shisuiki/agent-chat), which adopted Apache 2.0 on 2026-07-29. `NOTICE` credits the upstream authors. Retain it, along with `LICENSE`, when you redistribute, and mark any files you change. See [docs/LICENSING.md](docs/LICENSING.md).
