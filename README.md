[English](README.md) | [中文](README.zh-CN.md)

# Hagency

**Lend Codex agents to projects on a Palpo Matrix server, with owner approval and token budgets.**

Hagency is one Rust service, `hagency serve`. The *operator* is the person who runs Hagency and offers its resources to Palpo. The operator publishes resources to a connected Palpo homeserver. A resource is a model, a reasoning effort and a monthly token ceiling. Projects define agents on those resources and request tokens. The operator approves an amount. Hagency then gives the agent its own Matrix identity in the project's room. People @mention the agent to give it work. The owner approves the agent's risky operations from a private encrypted room.

This repository holds the Rust service in [native/](native/) and the console source in [mockup/](mockup/). The console is a Next.js app that is exported to static files at build time. `hagency serve` serves those files.

**Using Hagency through a Palpo server?** Start with the [user guide](docs/user-guide/README.md).
**Changing the service?** Start with the [code walkthrough](docs/architecture-walkthrough.md).

## Contents

| Section | |
| --- | --- |
| [What it does](#what-it-does) | The capability surface |
| [Architecture](#architecture) | One process, its threads and its crates |
| [Build and install](#build-and-install) | From source, as a systemd or launchd service |
| [First run](#first-run) | Start the service, connect Palpo, approve an agent |
| [Operating](#operating) | Health, logs, backup, credential rotation |
| [Configuration](#configuration) | The state directory, `fleet-runtime.json` and `agent-driver.json` |
| [Security posture](#security-posture) | What is enforced, and what is assumed |
| [Development](#development) | Tests and CI gates |
| [Documentation](#documentation) | Guides, design records and history |
| [License](#license) | Apache 2.0 and fork provenance |

## What it does

- **Resources and the catalog.** The operator configures resources in the console. Hagency publishes new resources to Palpo. Ceilings, seats and internal ids stay private.
- **Project-defined agents.** A project member defines an agent on a published resource in Palpo web. The definition names the requested tokens and a daily rate. Each definition becomes an *engagement*: the record of one agent for one project. It waits for the operator's decision.
- **Approval with an allocation.** The operator approves an amount, up to "all remaining". The amount must fit the resource's ceiling, seats and pool headroom. When the agent uses up its allocation, it pauses without dropping work. It resumes when the operator adds tokens.
- **Provisioning without a coordinator.** For a Palpo fleet imported in the console, Hagency creates every account itself through the fleet's App Service: the approval bot, each agent, and the agent's encrypted DM with its owner. No separate coordinator agent is needed (ADR-187).
- **Mention-driven work.** In the project room, an agent acts only when a person @mentions it. It reads the surrounding discussion as context and replies in the conversation's thread. In its DM, every message from the owner reaches it.
- **Work in other rooms.** An agent joins rooms its owner invites it to. An invitation from anyone else waits for a decision in the console. In a joined room the agent follows the rules in [Where an agent works](#where-an-agent-works) (ADR-188).
- **Owner approvals.** These go to the owner as cards in a private encrypted approval room:
  - commands and file changes outside the sandbox;
  - coordination tools;
  - file transfers.

  Only a verified verdict from the owner's own device counts. An unanswered card is denied.
- **One console.** The same binary serves the console on its loopback port. It covers resources, accounts, agents, engagements, project sides, approvals, invitations, tasks, usage and alerts.

### Where an agent works

| Room | What wakes the agent |
| --- | --- |
| Its DM with the owner | Every message from the owner |
| The project room (unencrypted) | A human message that @mentions it |
| A joined unencrypted group room | A human message that @mentions it |
| A joined room whose only members are the owner and the agent | Every message from the owner, encrypted or not |
| A joined encrypted room with other people in it | Nothing. The agent does not work there. |

An agent encrypts only for its owner's devices. Other people in an encrypted room could not read its replies. So in an encrypted room shared with other people, the agent stays joined and posts a notice that it cannot work there. It repeats the notice only after someone else posts, and at most once every 15 minutes. Hagency checks the room again on every pass. If the other people leave, the agent starts working there.

Work in a joined room spends the same allocation. Its approval cards still go to the owner's approval room.

## Architecture

```text
Palpo homeserver  <── outbound HTTPS ──  hagency serve (127.0.0.1:13300)
                                           ├─ Palpo transport: requests, probes, catalog, statuses
                                           ├─ fleet service: provisioning loop, one approval pump per owner
                                           ├─ per agent: a driver thread (Matrix /sync, intake, replies) and an invite poller
                                           ├─ domain + custody SQLite, one writer thread each
                                           ├─ console and operator API
                                           └─ per dispatch: hagency guardian ─> codex app-server ─> hagency mcp
```

- **Outbound only.** `hagency serve` opens no port to the outside. It polls Palpo's fleet API and polls Matrix `/sync` for each agent, so it can run behind NAT.
- **Single binary.** The same executable is:
  - the daemon;
  - the guardian that owns each runner's process tree;
  - the MCP helper that gives Codex its task tools;
  - the operator CLI.
- **Fleet service.** When a Palpo fleet is imported, the fleet service runs it. It creates the fleet's accounts, provisions approved agents, and runs one approval pump per owner. Every step retries with backoff. One engagement's failure does not stop the others.
- **Crates.** The Rust workspace lives in [native/](native/). The main crates are:
  - `hagency-core`: the domain types;
  - `hagency-store`: the durable rules;
  - `hagency-matrix`: the Matrix side effects;
  - `hagency-palpo`: the fleet transport;
  - `hagency-execution` and `hagency-runtime`: the Codex runs;
  - `hagency-platform`: process supervision.

  [native/README.md](native/README.md) lists every crate.

The [code walkthrough](docs/architecture-walkthrough.md) follows each flow through the code, with diagrams.

## Build and install

Requirements:

| | |
| --- | --- |
| Rust | The toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml) |
| Node.js 22 | Build time only, to export the console |
| Codex CLI | The runner. The runtime configuration names its path and SHA-256. |
| Host | Linux with systemd, or macOS with launchd |
| Palpo | A homeserver whose admin can run **Add Hagency** |

To build the binary and the console:

1. Build the binary:

   ```bash
   cargo build --release --locked -p hagency
   ```

2. Install the console's build dependencies:

   ```bash
   (cd mockup && npm ci)
   ```

3. Export the console to a new directory:

   ```bash
   node mockup/scripts/build-native-console.mjs --output /abs/path/console-assets
   ```

   The script refuses an existing directory and creates the new one with mode 0700.

### Imported fleet

This is the recommended path for new installs (see [Service modes](#service-modes)). The installer has no imported-fleet mode. Use one of these two ways:

- **In the foreground.** Run `hagency serve` directly, as in [Connect an imported fleet](#connect-an-imported-fleet).
- **As a service.**
  1. Run the installer as in [Coordinator install (existing installs)](#coordinator-install-existing-installs), without `agent-driver.json`. It runs `hagency init`, then writes and enables the unit. Its `/ready` gate then fails, because the unit runs `--agent-driver`. The unit stays in place.
  2. Edit the unit (`/etc/systemd/system/hagency-native.service` on Linux, `~/Library/LaunchAgents/io.hagency.native.plist` on macOS) and remove `--agent-driver`.
  3. Copy `fleet-runtime.json` into the state directory with mode 0600 (see [Configuration](#configuration)).
  4. Restart the service: `systemctl daemon-reload && systemctl restart hagency-native` on Linux, or `launchctl bootout` and then `launchctl bootstrap` the plist on macOS.

  Then continue with [Connect an imported fleet](#connect-an-imported-fleet) from step 4. The walkthrough lists this gap under [known gaps](docs/architecture-walkthrough.md#15-implemented-not-built-yet-and-known-gaps) ("Deploying an imported fleet").

### Coordinator install (existing installs)

The installer sets up coordinator installs only. An existing coordinator install keeps working as configured. To set one up as a service, run the installer:

```bash
install/install-native.sh \
  --install-dir /abs/path/bin \
  --state-dir /abs/path/state \
  --console-dir /abs/path/console-assets \
  [--config-dir /abs/path/config] [--overwrite]
```

- **Steps.** The installer:
  1. runs `hagency init`, which needs an empty state directory and creates `operator.token`;
  2. copies any config files into the state directory with mode 0600;
  3. renders [deploy/hagency-native.service](deploy/hagency-native.service) on Linux or [deploy/io.hagency.native.plist](deploy/io.hagency.native.plist) on macOS, and starts it;
  4. succeeds only when `/ready` answers 200 within 60 s.
- **Inputs.** `--install-dir` must contain the `hagency` binary. `--config-dir` may supply `agent-driver.json`, `development-driver.json`, `palpo-transport.json` and the private `matrix.*`, `palpo.*` and `approval.*` files. It refuses any other file name, including `fleet-runtime.json`.
- **Service mode.** The unit templates run `serve --agent-driver --palpo-transport`. That is the coordinator mode. Without `agent-driver.json` the service does not start, so the `/ready` gate fails and the install fails.
- **Refusals.** The installer refuses an existing unit unless you pass `--overwrite`.

There is no published release yet. [release-native.yml](.github/workflows/release-native.yml) builds per-target binaries and `SHA256SUMS` on manual dispatch only.

## First run

### Service modes

| Mode | `serve` flags | Configuration |
| --- | --- | --- |
| Imported fleet (ADR-187) | `--palpo-transport` | `fleet-runtime.json`, plus the files the console import writes |
| Coordinator install | `--agent-driver --palpo-transport` | `agent-driver.json` and its `matrix.*` and `approval.*` files |

`--agent-driver` and `--development-driver` are mutually exclusive. Without `--palpo-transport`, a console import is saved and starts on the next start with the flag.

### Connect an imported fleet

1. Create the state directory:

   ```bash
   hagency init --state-dir /abs/path/state
   ```

2. Write `fleet-runtime.json` into the state directory with mode 0600. See [Configuration](#configuration). You can also do this later: the fleet service waits for the file.
3. Start the service:

   ```bash
   hagency serve --state-dir /abs/path/state --palpo-transport \
     --console-assets /abs/path/console-assets
   ```

4. Open the console:

   ```bash
   hagency console-access --state-dir /abs/path/state
   ```

   The command prints a link that stays valid until you print a new one. Opening the link exchanges it for an `HttpOnly` session cookie; a restart does not sign you out.
5. In Palpo web, the Palpo admin runs **Add Hagency**.
6. In Palpo web, sign in with the account that owns this Hagency. Open **My Hagency access** and download the Hagency configuration.
7. In the console, the operator opens **Project sides → Connect a Palpo project server**, picks the file and enters the homeserver's Matrix address. The service starts the Palpo transport without a restart. One service runs one Palpo fleet.
8. In Palpo web, the Palpo account that owns this Hagency clicks **Verify connection & create reception**. The fleet service then creates the fleet's representative device and local keys. The approval bot gets one device per owner, created when the fleet service first prepares that owner's approved agent (once the owner has a cross-signing key).
9. Sign Codex in where fleet agents will find it. With `local_codex` in `fleet-runtime.json`, Codex runs with `HOME` set to `local_codex.home` and `CODEX_HOME` set to `local_codex.codex_home`, so sign in with `CODEX_HOME=<local_codex.codex_home> codex login`. Without `local_codex`, Codex runs with both set to `<state>/runtime-home`. After step 8, when the fleet service loads `fleet-runtime.json`, it creates that directory with mode 0700 if it is missing. Sign in there:

   ```bash
   CODEX_HOME=/abs/path/state/runtime-home codex login
   ```

   Managed accounts and `hagency account login` serve coordinator installs only.
10. Create the first resource with the operator API. The console creates further resources only as copies of an existing one, so the first one cannot come from the console. Replace the state directory, and the listen address if you changed it from the default `127.0.0.1:13300`:

    ```bash
    curl -s -X POST http://127.0.0.1:13300/api/native/v1/resources \
      -H "Authorization: Bearer $(cat /abs/path/state/operator.token)" \
      -H 'Content-Type: application/json' \
      -d '{"presetId":"local_codex","seatId":"local_codex_seat","framework":"codex",
           "model":"gpt-5.6-sol","provider":"openai","reasoning":"medium",
           "ceiling":{"tokens":20000000,"period":"monthly"},"published":true}'
    ```

    No seat has to be registered first. The answer is the resource's public catalog entry.

    - **Qualified pairs only.** Palpo sees a resource only if its `model` and `reasoning` form a pair qualified for at least one role in [native/hagency-core/role-capacity.json](native/hagency-core/role-capacity.json). For Codex those are `gpt-5.6-sol` with `low`, `medium` or `high`. Any other pair is stored, but not published.
    - **Matching the login.** With `local_codex`, `seatId` must equal `local_codex.seat`, `framework` must be `codex`, and `provider` must be `openai` or left out. The API does not check this. A mismatched resource is accepted and published, but its agents are refused when Hagency provisions them, after the operator approves.

    A qualified resource then appears in Palpo, where projects can define agents on it. Copy it in the console to offer other models, reasoning efforts or ceilings.
11. Approve requests under **Engagements**. The agent joins the project room when provisioning completes.

The [user guide](docs/user-guide/README.md) describes steps 5 to 8 from the Palpo side.

## Operating

| Task | Command |
| --- | --- |
| Check liveness | `curl -s 127.0.0.1:13300/health` |
| Check readiness | `curl -s 127.0.0.1:13300/ready`. A 503 names each component that is not ready. |
| Service status (Linux) | `systemctl status hagency-native` · `journalctl -u hagency-native` |
| Logs (macOS) | `<install-dir>/logs/hagency-native.stdout.log` and `…stderr.log` |
| Set the log level | `RUST_LOG` (default `info`) |
| Stop (macOS) | `launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/io.hagency.native.plist`. `KeepAlive` restarts a killed process. |
| Inspect | `hagency engagements`, `hagency resources`, `hagency alerts` (with `--state-dir`; add `--json` for raw output) |
| Back up while running | `hagency backup --state-dir <state> --out <new dir>` |
| Restore | `hagency restore --state-dir <empty dir> --from <backup>` |
| Rotate the operator token | `hagency rotate --state-dir <state> operator-token`. Restart the service to use the new token. |

**Fleet service progress.** The fleet service logs each stage change as `fleet service stage`. The stages are:

1. `awaiting_runtime_config`: `fleet-runtime.json` is missing.
2. `awaiting_reception`: Palpo's **Verify connection** has not bound the reception room yet.
3. `identities`: the service is creating the fleet's accounts and keys.
4. `running`: the provisioning loop and the approval pumps run.

A stage that fails is retried with backoff. A configuration the service refuses shows as `refused_config`.

**Owner keys.** The first time Hagency needs an owner's cross-signing master key, it reads the key from the homeserver and pins it in the store. An owner without cross-signing has no key yet, so that owner's agents wait. A pinned key is never replaced by what the homeserver reports later. The console and CLI do not offer a re-pin yet. An owner who resets cross-signing therefore cannot be served until the pin is changed. Even then, a re-pin does not repair agents already enrolled: their frozen key list keeps the old key, so their sends to the owner fail until each agent is provisioned again, and the owner gets a new approval device (ADR-187 amendment; see [known gaps](docs/architecture-walkthrough.md#15-implemented-not-built-yet-and-known-gaps)).

**Shutdown.** On SIGTERM, the service stops the runners, the Palpo transport and the fleet service. It then closes the Matrix sessions and the databases, and stops its HTTP server last. systemd allows 20 s.

## Configuration

`hagency serve` reads no environment file. Its configuration is the files in `--state-dir`:

| File | Purpose |
| --- | --- |
| `operator.token` | Operator bearer secret, created by `hagency init` |
| `fleet-runtime.json` | Codex runtime settings for an imported fleet, described below. You write it by hand. |
| `palpo-transport.json`, `palpo.machine_token`, `palpo-appservice.json` | Written by the Palpo import |
| `representative.identity.json`, `matrix.representative_token`, `matrix.appservice_token`, `matrix.provisioning_key` | The fleet's representative and provisioning credentials. The fleet service creates them once. |
| `approval-<owner>.*`, `approval-sdk-<owner>/` | One approval-bot device per owner: its record, token, identity and key files, and its SDK store. `<owner>` is a slug derived from the owner's Matrix ID. Created when the fleet service first prepares that owner's approved agent (once the owner has a cross-signing key). |
| `agent-driver.json`, `matrix.*`, `approval.*` | Coordinator installs only: runner, Matrix and approval-bot settings |
| `agent-matrix-provision_<engagement>/` | One per agent: its encrypted provisioning and room records, and its Matrix SDK store |
| `runtime-home/` | `HOME` and `CODEX_HOME` for Codex when `fleet-runtime.json` has no `local_codex` block; holds the Codex sign-in |
| `fleet-workspace/` | A private placeholder workspace for the fleet's execution host; no dispatch runs in it |
| `factory-task-contexts/` | Per-dispatch task context files for the warm task bridge |
| `console-logins.json` | SHA-256 hashes of the console access link and logins, so a restart does not sign you out |
| `domain.sqlite3`, `custody.sqlite3` | Durable state (WAL, `synchronous=FULL`). Use one process per state directory. |

Configuration files must be owner-private (mode 0600). `fleet-runtime.json` and `agent-driver.json` refuse unknown fields.

`fleet-runtime.json` fields:

| Field | Required | Meaning |
| --- | --- | --- |
| `profile` | Yes | Must be `palpo_fleet_runtime_v1` |
| `executable` | Yes | The Codex binary: an absolute path with no symlink in it |
| `executable_sha256` | Yes | The binary's SHA-256, as 64 lowercase hex characters |
| `local_codex` | No | Binds the operator's own Codex login (sub-fields below). Without it, Codex uses `<state>/runtime-home`. |
| `file_limit` | Yes | The file size limit for the file tools, in bytes: 1 to 4,194,304. Checked even when both file tools are off. |
| `operation_ms` | Yes | The per-dispatch operation budget, from 100 to 1,200,000 ms. It must be at least `approval_owner_wait_ms` + 5000. |
| `response_ms` | Yes | The per-dispatch response budget, from 10 to 2000 ms |
| `idle_ms` | Yes | How long a warm runtime stays idle, from 100 to 1,200,000 ms |
| `approval_owner_wait_ms` | No | How long a card waits for the owner before it is denied. The default is 1000 ms, so set it. With the 5000 ms reply reserve it must fit in 600,000 ms. |
| `send_file`, `receive_file` | No | Enable the file tools. Default `false`. |
| `coordination_tools` | No | Enable delegation and peer tools. The owner approves each call. Default `false`. |
| `matrix_request_interval_ms`, `matrix_sdk_timeout_ms` | No | Matrix request pacing, and the SDK budget (10 to 60,000 ms) |
| `home` | Yes | Agent homes, with three required sub-fields: `root`, an existing owner-private directory; `task_client`, the absolute path of the `hagency` binary; and `projects`, up to 16 entries with `project_id`, `source` and `mode` (`copy` or `symlink`). `projects` may be `[]`: a project with no entry gets a home without a source copy. |

`local_codex` sub-fields, all required when the block is present:

| Field | Meaning |
| --- | --- |
| `profile` | Must be `provider_owned_codex_v1` |
| `preset` | A resource preset id, recorded on each claim this login serves |
| `seat` | The seat id this login serves. A Codex resource whose `seatId` matches runs on this login. |
| `home` | The directory Codex gets as `HOME` |
| `codex_home` | The directory Codex gets as `CODEX_HOME`, holding the Codex sign-in |

`home` and `codex_home` must be absolute paths with no symlink in them, owned by the service user and not writable by group or others.

A minimal example. Each `/srv/hagency/...` path is a placeholder for your own:

```json
{
  "profile": "palpo_fleet_runtime_v1",
  "executable": "/srv/hagency/bin/codex",
  "executable_sha256": "<64 lowercase hex characters: shasum -a 256 of the codex binary>",
  "local_codex": {
    "profile": "provider_owned_codex_v1",
    "preset": "local_codex",
    "seat": "local_codex_seat",
    "home": "/srv/hagency/codex-user",
    "codex_home": "/srv/hagency/codex-user/.codex"
  },
  "file_limit": 4194304,
  "operation_ms": 600000,
  "response_ms": 2000,
  "approval_owner_wait_ms": 300000,
  "idle_ms": 600000,
  "home": {
    "root": "/srv/hagency/agent-homes",
    "task_client": "/srv/hagency/bin/hagency",
    "projects": []
  }
}
```

Matrix identities do not appear here. They come from the imported fleet.

`agent-driver.json` configures a coordinator install. It shares the Codex, budget and file fields, and adds `workspaces`, the `matrix` and `approval` identity blocks, and `factory_service`.

The authoritative definitions are `FleetRuntimeConfig` and `Config` in [native/hagency/src/bootstrap/config.rs](native/hagency/src/bootstrap/config.rs).

## Security posture

What Hagency **enforces**:

- **Loopback only.** `hagency serve` refuses any listen address that is not loopback. For remote access, use an SSH tunnel or a reverse proxy that sends `Host: 127.0.0.1:13300` and adds no `Forwarded` or `X-Forwarded-*` headers.
- **Console requests.** The console requires the exact `Host` header and refuses forwarded headers. Writes need a same-origin `Origin`. The operator API requires the same exact `Host`, refuses `Forwarded`, `X-Forwarded-For`, `Origin` and `Sec-Fetch-Site`, then compares the bearer token's SHA-256 in constant time.
- **Owner approvals.** Approvals come only from the owner's verified device. They arrive in an encrypted room whose only members are the owner and the approval bot. A third member or lost encryption disables the room. A failed delivery or an expired wait counts as a deny.
- **One approval device per owner.** Each owner's approval-bot device trusts only that owner. Owners' approvals stay isolated from each other.
- **Fenced runners.** Each dispatch has a capability and a fence number. A stale runner's calls are refused. A reply is published only after its process tree is proven gone.
- **Codex sandbox.** Codex runs with `workspace-write` and `on-request` approvals, no network, and only its workspace writable. Hagency checks the settings Codex echoes back.
- **Write-only credentials.** No route returns a stored token. The Palpo import's answer carries only public facts.
- **Hardened unit.** The systemd unit sets `NoNewPrivileges`, `ProtectSystem=full`, an empty capability set and a syscall filter.

What it **assumes**:

- **Shared host trust.** Loopback trust covers the whole machine, so any local process can reach the port. Keep `operator.token` and the state directory private.
- **Owner keys are trusted on first use.** If the homeserver lies the first time Hagency reads an owner's key, Hagency trusts the wrong key. Later answers from the homeserver never replace the pinned key.
- **Group rooms are open to members.** Any joined member who @mentions an agent can give it work. Room membership, the allocation and owner approvals are the controls.
- **Encrypted shared rooms are out of reach.** An agent cannot work in an encrypted room with people other than its owner.
- **No federation.** All members must be on the fleet's own server.
- **Sandbox qualification is open.** Hagency requests and checks the sandbox, but per-OS qualification of its effect is still open.

## Development

Run the same gates as CI:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
node native/scripts/check-rust-spec-bindings.mjs
node --test native/scripts/check-production-callers.test.mjs
node native/scripts/check-production-callers.mjs
cargo test --workspace --all-targets --locked --no-fail-fast -- \
  --skip native_codex_real_app_server \
  --skip native_two_agent_qualification_records_its_evidence
```

- `check-rust-spec-bindings.mjs` checks that every spec selector names a real test.
- `check-production-callers.mjs` checks that every `Production caller:` line resolves in the production call graph. `check-production-callers.test.mjs` tests the checker itself.
- The two skipped tests need a real qualified host (ADR-140, ADR-144). They stay in the specs so the binding gate still checks them.

Task contracts in [specs/](specs/) bind behaviour to tests. [knowledge/decisions/](knowledge/decisions/) records the decisions behind them.

Two workflows run the gates:
- [ci.yml](.github/workflows/ci.yml) runs the gates above on Ubuntu for pull requests and pushes to `main`, and on manual dispatch. It skips changes that touch only `docs/`, `knowledge/` or the two READMEs.
- [rust.yml](.github/workflows/rust.yml) adds macOS and Windows, the console browser tests and a release build. It runs nightly, on manual dispatch, on `nv*` tags, and on pull requests labelled `full-ci` or touching its listed paths.

The walkthrough's [Making a change](docs/architecture-walkthrough.md#16-making-a-change) section says where new rules, migrations, agent tools and console routes go.

## Documentation

| Document | Covers |
| --- | --- |
| [docs/user-guide/README.md](docs/user-guide/README.md) | Connecting Palpo, rooms, who can talk to an agent, tokens |
| [docs/architecture-walkthrough.md](docs/architecture-walkthrough.md) | The code, flow by flow |
| [native/README.md](native/README.md) | The Rust workspace: crates, commands, tests |
| [knowledge/decisions/](knowledge/decisions/) | Architecture decision records |
| [specs/](specs/) | Task contracts bound to tests |
| [docs/guides/](docs/guides/) | Chinese guides: Matrix conversations, files, agent status, the Palpo outbound transport |
| [docs/history/](docs/history/) | Design documents of the removed TypeScript product, kept for history |
| [docs/LICENSING.md](docs/LICENSING.md) | Fork provenance and Apache 2.0 attribution obligations |

## License

**Apache License 2.0**: see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).

Hagency is a fork of [agent-chat](https://github.com/shisuiki/agent-chat), which adopted Apache 2.0 on 2026-07-29. `NOTICE` credits the upstream authors. Keep `NOTICE` and `LICENSE` when you redistribute, and mark any files you change. See [docs/LICENSING.md](docs/LICENSING.md).
