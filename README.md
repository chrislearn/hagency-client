[English](README.md) | [中文](README.zh-CN.md)

# Hagency

**Lend Codex agents to projects on a Palpo Matrix server, with owner approval and token budgets.**

Hagency is one Rust binary, `hagency`, that runs as a service on your machine. The *operator* is the person who runs Hagency and offers its resources to Palpo. The operator publishes resources to a connected Palpo homeserver. A resource is a model, a reasoning effort and a monthly token ceiling. Projects define agents on those resources and request tokens. The operator approves an amount. Hagency then gives the agent its own Matrix identity in the project's room. People @mention the agent to give it work. The owner approves the agent's risky operations from a private encrypted room.

This repository holds the Rust service in [native/](native/) and the console source in [mockup/](mockup/). The console is a Next.js app that is exported to static files at build time and embedded in the binary.

**Running Hagency?** Follow [Set up Hagency](#set-up-hagency).
**Using Hagency through a Palpo server?** Start with the [user guide](docs/user-guide/README.md).
**Changing the service?** Start with the [code walkthrough](docs/architecture-walkthrough.md).

## Contents

| Section | |
| --- | --- |
| [What it does](#what-it-does) | The capability surface |
| [Architecture](#architecture) | One process, its threads and its crates |
| [Set up Hagency](#set-up-hagency) | Sign in to Codex, start Hagency, finish setup in the console |
| [Set up from the command line](#set-up-from-the-command-line) | `hagency setup`, `hagency serve`, the installer, coordinator installs, the operator API |
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
- **One console.** The same binary serves the console on its loopback port. It covers setup, resources, accounts, agents, engagements, project sides, approvals, invitations, tasks, usage and alerts.

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
Palpo homeserver  <── outbound HTTPS ──  hagency start (127.0.0.1:13300)
                                           ├─ Palpo transport: requests, probes, catalog, statuses
                                           ├─ fleet service: provisioning loop, one approval pump per owner
                                           ├─ per agent: a driver thread (Matrix /sync, intake, replies) and an invite poller
                                           ├─ domain + custody SQLite, one writer thread each
                                           ├─ console and operator API
                                           └─ per dispatch: hagency guardian ─> codex app-server ─> hagency mcp
```

- **Outbound only.** The service opens no port to the outside. It polls Palpo's fleet API and polls Matrix `/sync` for each agent, so it can run behind NAT.
- **Single binary.** The same executable is:
  - the daemon, with the console embedded;
  - the per-user service registration;
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

## Set up Hagency

You need:

| | |
| --- | --- |
| Host | macOS, or Linux with systemd. You use the console on the machine that runs Hagency. |
| Coding agent | Codex CLI, installed and on your `PATH` |
| Palpo | A homeserver whose admin can run **Add Hagency** |

Your terminal work is steps 1 to 3. The rest happens in the console.

### 1. Sign in to the coding agent

Sign in to Codex yourself, in a terminal on this machine:

```bash
codex login
```

On a machine without a browser, add `--device-auth`.

Hagency never signs in for you. It only asks Codex whether it is signed in, and how (`codex login status`). It never reads or stores your credentials.

### 2. Get the hagency binary

**A release build.** [release-native.yml](.github/workflows/release-native.yml) builds one binary per platform with the console embedded: macOS arm64 and x86-64, Linux x86-64 and arm64, plus `SHA256SUMS`. It runs on manual dispatch only and publishes no GitHub release yet. Download the binary from the run's artifacts.

**From source.** You need the Rust toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml), and Node.js 22 at build time only.

1. Install the console's build dependencies:

   ```bash
   (cd mockup && npm ci)
   ```

2. Build the console into a new directory:

   ```bash
   node mockup/scripts/build-native-console.mjs --output /abs/path/console
   ```

   The script refuses an existing directory and creates the new one with mode 0700.

3. Build the binary with that console embedded. Use an absolute path:

   ```bash
   HAGENCY_CONSOLE_DIR=/abs/path/console cargo build --release --locked -p hagency
   ```

   The build refuses a directory without the console's `manifest.json`.

Without `HAGENCY_CONSOLE_DIR`, the binary has no console. `hagency start` then refuses to run unless you pass `--console-assets /abs/path/console`. The same flag serves a console folder instead of the embedded one, for console development.

Put the binary where it will stay, for example `~/.local/bin/hagency`. The service in step 3 runs the binary from that path.

### 3. Start Hagency

Choose one:

- **As a service (recommended):**

  ```bash
  hagency service install
  ```

- **In this terminal:**

  ```bash
  hagency start
  ```

  Stop it with Ctrl-C.

`hagency start`:
- uses a default state directory: `~/Library/Application Support/Hagency` on macOS, `~/.local/share/hagency` on Linux (or `$XDG_DATA_HOME/hagency`). Pass `--state-dir DIR` for another one.
- initializes the directory when it is new or empty. It refuses a non-empty directory that is not a Hagency state directory.
- listens on `127.0.0.1:13300`. `--listen` accepts loopback addresses only.
- runs as an imported fleet (`serve --palpo-transport`) with the embedded console. It starts without `fleet-runtime.json` and without a Palpo import; you complete both in the console.
- prints a console sign-in link and opens it in your browser. Pass `--no-open` to only print it.

`hagency service install` registers a per-user service that runs `hagency start --no-open`, and starts it:
- **macOS:** a LaunchAgent, `~/Library/LaunchAgents/io.hagency.plist`. It starts at login and restarts after a crash. Its log is `~/Library/Logs/Hagency/hagency.log`.
- **Linux:** a `systemd --user` unit, `~/.config/systemd/user/hagency.service`. No `sudo` is needed. A user service stops when you log out. To keep it running, run `loginctl enable-linger $USER` once.
- The service runs as you, so it uses your Codex sign-in. It records your current `PATH`, so it finds the same `codex` you do.
- It takes `--state-dir`, `--listen` and `--no-open`, like `start`. When the service answers, the command prints the sign-in link and opens it.
- Run it again to replace the service, for example after you move the binary.
- `hagency service uninstall` stops and removes the service. It keeps the state directory.

### 4. Open the console

Open the printed link in a browser on this machine. The link opens the whole console. Opening it exchanges it for an `HttpOnly` session cookie, and a restart does not sign you out. The link stays valid until you print a new one, so keep it private.

To print a new link:

```bash
# macOS
hagency console-access --state-dir "$HOME/Library/Application Support/Hagency"
# Linux
hagency console-access --state-dir ~/.local/share/hagency
```

Pass the same `--state-dir` and `--listen` you gave `start`, if you changed them. The link is printed only to a terminal. The service log shows only this command.

### 5. Finish setup in the console

Open **Setup** in the console menu. It has three steps. Each step shows a check mark when it is done. Until all three are done, every other console page shows a one-line note, "Setup is not finished", with a link to **Setup**. A coordinator install has no Setup steps: it shows no note, and its Setup page says that its runtime is configured in `agent-driver.json`.

1. **Coding agents.** Hagency finds Codex on the service's `PATH` and shows its path, its version and whether it is signed in.
   - **Not installed:** install Codex, then click **Check again**.
   - **Not signed in:** run `codex login` in a terminal on this machine, then click **Check again**.
   - **Signed in:** nothing to click. When the page loads, Hagency writes and validates `fleet-runtime.json` with the defaults under [Configuration](#configuration). The fleet service picks it up within 5 s, without a restart.
   - The step shows how Codex is signed in: a ChatGPT plan or an API key. With a plan, it notes that plan sign-ins are meant for personal use, and suggests an API key before you offer the agent to other people. It does not block.
2. **Connect Palpo.**
   1. In Palpo web, the Palpo admin runs **Add Hagency**.
   2. In Palpo web, sign in with the account that owns this Hagency. Open **My Hagency access** and click **Download Hagency configuration**.
   3. In this step, pick the file, enter the homeserver's Matrix address and click **Connect**. The Palpo transport starts without a restart. One Hagency runs one Palpo fleet.
   4. In Palpo web, click **Verify connection & create reception**. The fleet service then creates the fleet's representative device and keys. The approval bot gets one device per owner, created when the fleet service first prepares that owner's approved agent (once the owner has a cross-signing key).

   The same import is also under **Project sides → Connect a Palpo project server**.
3. **Offer a resource.** This step needs step 1.
   1. Choose a **Model**. The list holds only the model and reasoning pairs Hagency qualifies ([role-capacity.json](native/hagency-core/role-capacity.json)). For Codex these are `gpt-5.6-sol` with `low`, `medium` or `high`.
   2. Set the **Monthly token ceiling**. The default is 20,000,000.
   3. Click **Offer to Palpo**.

   Hagency creates the resource on the seat of your Codex sign-in and publishes it. Palpo receives it within 15 s, and projects can define agents on it. Offer more models in the same step. Edit or withdraw resources under **My resources**.

### 6. Approve agents

Projects define agents on your resources in Palpo web. Approve each request under **Engagements**. The agent joins the project room when provisioning completes. The [user guide](docs/user-guide/README.md) describes the owner's side.

Everything after setup happens in the console.

## Set up from the command line

Use these for automation, recovery and coordinator installs. The setup above needs none of them.

### Service modes

| Mode | Use it for | How it runs | Configuration |
| --- | --- | --- | --- |
| Fleet (default, recommended) | An imported Palpo fleet, with no coordinator agent (ADR-187) | `hagency start`, or `serve --palpo-transport` | `fleet-runtime.json`, written by the Setup page or `hagency setup`, plus the files the console import writes |
| Coordinator | Existing installs that run a coordinator agent | `serve --agent-driver --palpo-transport` | `agent-driver.json` and its `matrix.*` and `approval.*` files |

`--agent-driver` and `--development-driver` are mutually exclusive. Without `--palpo-transport`, a console import is saved and starts on the next start with the flag. `serve` serves the embedded console when the binary has one; `--console-assets` overrides it.

### Prepare a state directory with `hagency setup`

`hagency setup` does what the Setup page's first step does, with options:

```bash
hagency setup --state-dir /abs/path/state
```

`hagency setup`:
- initializes the directory as `hagency init` does when it is new or empty. It refuses a non-empty directory that has no `operator.token`.
- finds the Codex binary: `--codex PATH`, or else `codex` on `PATH`. If that is the npm launcher script, setup uses the native binary that the npm package ships beside it.
- finds the Codex sign-in folder: `--codex-home DIR`, or else `$CODEX_HOME`, or else `~/.codex`. The folder must exist; if it does not, setup asks you to run `codex login` first. With `--no-local-codex`, setup does not look for this folder, so no `~/.codex` is needed: agents sign in to `<state>/runtime-home` instead, setup reports that folder, and the file gets no `local_codex` block.
- creates `<state>/agent-homes` and writes `fleet-runtime.json` with mode 0600 and the defaults listed under [Configuration](#configuration).
- validates the file with the same loader `serve` uses. A file that fails is renamed to `fleet-runtime.json.rejected`, so the service never starts on it.
- refuses to replace an existing `fleet-runtime.json` unless you pass `--force`. With `--force`, it keeps the old file as `fleet-runtime.json.bak-<seconds>`.

It prints the Codex binary it chose, the file it wrote, and whether Codex is signed in. If Codex is not signed in, it prints the command to run, `CODEX_HOME=<folder> codex login`. It ends with the next commands to run. Pass `--listen` if `serve` will use an address other than `127.0.0.1:13300`. `--console-assets` only fills in the printed `serve` command.

With a `local_codex` block, Codex runs with `HOME` set to `local_codex.home` and `CODEX_HOME` set to `local_codex.codex_home`. Without one, both are `<state>/runtime-home`. Managed accounts and `hagency account login` serve coordinator installs only.

### Run in the foreground with `hagency serve`

```bash
hagency serve --state-dir /abs/path/state --palpo-transport
```

Then print a console link with `hagency console-access --state-dir /abs/path/state`, and continue at [step 5](#5-finish-setup-in-the-console).

### Install a fleet with install-native.sh

[install/install-native.sh](install/install-native.sh) installs a system service from a binary and a console folder. It predates `hagency service install`, which replaces it for fleet installs.

Which user runs it matters, because the service runs as that user and uses that user's Codex sign-in:

- **Linux.** The installer writes the unit to `/etc/systemd/system` and runs `systemctl`, so run it with `sudo`. It renders the unit's `User=` as the user running it, so the service runs as `root`.
  - The `local_codex` binding that setup writes accepts its home folder and Codex folder only when they are owned by the service user (`root`) and have no group or other write permission. `--codex-home` therefore cannot point at another user's `~/.codex`.
  - If `codex` is not on root's `PATH`, pass `--codex /abs/path/to/codex`.
  - The simplest path on Linux is `--no-local-codex`. After the install, sign Codex in to the service's runtime home with `sudo env CODEX_HOME=<state>/runtime-home codex login`.
  - With this installer, run every command that reads the state directory as root (`sudo`), including `hagency console-access`.
- **macOS.** Run it as yourself. It installs a LaunchAgent, `io.hagency.native`, for your user, which uses your Codex sign-in.

```bash
install/install-native.sh \
  --install-dir /abs/path/bin \
  --state-dir /abs/path/state \
  --console-dir /abs/path/console
```

- **Before you run it.** Copy the binary to `<install-dir>/hagency`; the installer refuses to start without it. Build the console folder as in [step 2](#2-get-the-hagency-binary).
- **Steps.** The installer:
  1. runs `hagency init`, which needs an empty state directory and creates `operator.token`;
  2. copies any files from `--config-dir` into the state directory with mode 0600;
  3. runs `hagency setup`, which finds Codex and writes a validated `fleet-runtime.json`. It skips this step when `--config-dir` supplied a `fleet-runtime.json`. If setup fails, the install stops and shows setup's message;
  4. renders [deploy/hagency-native.service](deploy/hagency-native.service) on Linux or [deploy/io.hagency.native.plist](deploy/io.hagency.native.plist) on macOS with the mode's `serve` flags (`serve --palpo-transport` for a fleet), and starts it;
  5. succeeds only when `/ready` answers 200 within 60 s.
- **Codex options.** The installer passes these to `hagency setup`:
  - `--codex PATH`: the Codex binary. The default is `codex` on `PATH`.
  - `--codex-home DIR`: the folder that holds the Codex sign-in. The default is `$CODEX_HOME`, or `~/.codex`.
  - `--no-local-codex`: run agents with `<state>/runtime-home` instead of this machine's Codex sign-in. No `~/.codex` is needed.
- **Your own `fleet-runtime.json`.** Put it in a directory and pass `--config-dir DIR`. The installer copies it and does not run setup. The file must follow [Configuration](#configuration).
- **Refusals.** The installer refuses a missing `--console-dir`, a non-empty state directory, a missing binary, Linux without systemd, and an existing unit unless you pass `--overwrite`.
- **Retrying after a refusal.** Every refusal after step 1 leaves the state directory initialized: Linux without systemd, an existing unit without `--overwrite`, a bad file in `--config-dir`, and a setup failure. Empty the state directory or choose a new one before you run the installer again. After a setup failure you can instead finish with `hagency setup --state-dir …`.

Then print a console link with `hagency console-access --state-dir /abs/path/state`, and continue at [step 5](#5-finish-setup-in-the-console).

### Coordinator install (existing installs)

An existing coordinator install keeps working as configured. `hagency start` and the Setup page's first step do not apply to it. To install one as a service, pass `--mode coordinator` and a `--config-dir` that contains `agent-driver.json`:

```bash
install/install-native.sh --mode coordinator \
  --install-dir /abs/path/bin \
  --state-dir /abs/path/state \
  --console-dir /abs/path/console \
  --config-dir /abs/path/config [--overwrite]
```

- **Configuration.** `agent-driver.json` is required; without it the installer refuses to start. `--config-dir` may also supply `fleet-runtime.json`, `development-driver.json`, `palpo-transport.json` and the private `matrix.*`, `palpo.*` and `approval.*` files. The installer refuses any other file name.
- **Service.** The unit runs `serve --agent-driver --palpo-transport`. `hagency setup` does not run. The other steps and refusals are the same as for a fleet.

### Create a resource with the operator API

The Setup page's **Offer to Palpo** uses the same writer as this call. Replace the state directory, and the listen address if you changed it from the default `127.0.0.1:13300`:

```bash
curl -s -X POST http://127.0.0.1:13300/api/native/v1/resources \
  -H "Authorization: Bearer $(cat /abs/path/state/operator.token)" \
  -H 'Content-Type: application/json' \
  -d '{"presetId":"local_codex","seatId":"local_codex_seat","framework":"codex",
       "model":"gpt-5.6-sol","provider":"openai","reasoning":"medium",
       "ceiling":{"tokens":20000000,"period":"monthly"},"published":true}'
```

With install-native.sh on Linux, only root can read `operator.token`. Use a root shell (`sudo -s`): with `sudo curl …`, the `$(cat …)` still runs as you and cannot read the token. No seat has to be registered first. The answer is the resource's public catalog entry.

- **Qualified pairs only.** Palpo sees a resource only if its `model` and `reasoning` form a pair qualified for at least one role in [native/hagency-core/role-capacity.json](native/hagency-core/role-capacity.json). Any other pair is stored, but not published. The Setup page offers qualified pairs only.
- **Matching the login.** With `local_codex`, `seatId` must equal `local_codex.seat`, `framework` must be `codex`, and `provider` must be `openai` or left out. Setup writes the preset `local_codex` and the seat `local_codex_seat`, which the example uses. The API does not check the match. A mismatched resource is accepted and published, but its agents are refused when Hagency provisions them, after the operator approves.

## Operating

The commands below name the per-user service from `hagency service install`. With install-native.sh, the service is `hagency-native` (Linux) or `io.hagency.native` (macOS), and on Linux you run these as root (`sudo`), because the state directory belongs to the service user.

| Task | Command |
| --- | --- |
| Check liveness | `curl -s 127.0.0.1:13300/health` |
| Check readiness | `curl -s 127.0.0.1:13300/ready`. A 503 names each component that is not ready. |
| Service status (Linux) | `systemctl --user status hagency` · `journalctl --user -u hagency` |
| Logs (macOS) | `~/Library/Logs/Hagency/hagency.log`. With install-native.sh: `<install-dir>/logs/hagency-native.stdout.log` and `…stderr.log`. |
| Set the log level | `RUST_LOG` (default `info`) |
| Restart | `launchctl kickstart -k gui/$(id -u)/io.hagency` (macOS) · `systemctl --user restart hagency` (Linux) |
| Stop and remove | `hagency service uninstall`. It keeps the state directory. The service restarts a crashed process. |
| New console link | `hagency console-access --state-dir <state>` |
| Inspect | `hagency engagements`, `hagency resources`, `hagency alerts` (with `--state-dir`; add `--json` for raw output) |
| Back up while running | `hagency backup --state-dir <state> --out <new dir>` |
| Restore | `hagency restore --state-dir <empty dir> --from <backup>` |
| Rotate the operator token | `hagency rotate --state-dir <state> operator-token`. Restart the service to use the new token. |

**Fleet service progress.** The fleet service logs each stage change as `fleet service stage`. The stages are:

1. `awaiting_runtime_config`: `fleet-runtime.json` is missing. Finish **Setup → Coding agents** in the console, or run `hagency setup`.
2. `awaiting_reception`: Palpo's **Verify connection** has not bound the reception room yet.
3. `identities`: the service is creating the fleet's accounts and keys.
4. `running`: the provisioning loop and the approval pumps run.

The two `awaiting_*` stages are checked again every 5 s. A failed `identities` stage, and a configuration the service refuses (shown as `refused_config`), are retried with a backoff from 1 s to 60 s.

**Owner keys.** The first time Hagency needs an owner's cross-signing master key, it reads the key from the homeserver and pins it in the store. An owner without cross-signing has no key yet, so that owner's agents wait. A pinned key is never replaced by what the homeserver reports later. The console and CLI do not offer a re-pin yet. An owner who resets cross-signing therefore cannot be served until the pin is changed. Even then, a re-pin does not repair agents already enrolled: their frozen key list keeps the old key, so their sends to the owner fail until each agent is provisioned again, and the owner gets a new approval device (ADR-187 amendment; see [known gaps](docs/architecture-walkthrough.md#15-implemented-not-built-yet-and-known-gaps)).

**Restart while an agent waits for its owner.** Restarting the service while a new agent waits for its owner to join the DM strands that agent. See the user guide's [Known limitations](docs/user-guide/README.md#known-limitations).

**Shutdown.** On SIGTERM, the service stops the runners, the Palpo transport and the fleet service. It then closes the Matrix sessions and the databases, and stops its HTTP server last. The installer's systemd unit allows 20 s.

## Configuration

The service reads no environment file. Its configuration is the files in its state directory:

| File | Purpose |
| --- | --- |
| `operator.token` | Operator bearer secret, created by `hagency init` |
| `fleet-runtime.json` | Codex runtime settings for an imported fleet, described below. Written by the Setup page or `hagency setup`. |
| `agent-homes/` | The agents' home directories (`home.root` as `hagency setup` writes it) |
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

The Setup page and `hagency setup` write `fleet-runtime.json` and validate it with the same code; the table below is the file format, for reading it or for supplying your own through the installer's `--config-dir`. Setup writes these values:

- `executable` and `executable_sha256`: the Codex binary it found, and its hash;
- `file_limit` 4194304 (4 MiB), `operation_ms` 300000, `response_ms` 2000, `approval_owner_wait_ms` 180000, `idle_ms` 1200000;
- `send_file` and `receive_file` `true`;
- `home`: `root` is `<state>/agent-homes`, `task_client` is the running `hagency` binary, `projects` is `[]`;
- `local_codex`, unless you pass `--no-local-codex`: preset `local_codex`, seat `local_codex_seat`, `home` set to your `HOME`, `codex_home` set to the Codex sign-in folder (`$CODEX_HOME`, or `~/.codex`; the Setup page always uses this default, `hagency setup --codex-home` chooses another).

To change a value, edit the file (keep mode 0600) and restart the service, or run `hagency setup --state-dir <state> --force` with other options.

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
| `approval_owner_wait_ms` | No | How long a card waits for the owner before it is denied. Without the field the wait is 1000 ms; `hagency setup` writes 180000. With the 5000 ms reply reserve it must fit in 600,000 ms. |
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

What `hagency setup` writes, with each `/srv/hagency/...` path standing in for the paths it found:

```json
{
  "profile": "palpo_fleet_runtime_v1",
  "executable": "/srv/hagency/bin/codex",
  "executable_sha256": "<64 lowercase hex characters: the SHA-256 of the codex binary>",
  "local_codex": {
    "profile": "provider_owned_codex_v1",
    "preset": "local_codex",
    "seat": "local_codex_seat",
    "home": "/srv/hagency/home",
    "codex_home": "/srv/hagency/home/.codex"
  },
  "send_file": true,
  "receive_file": true,
  "file_limit": 4194304,
  "operation_ms": 300000,
  "response_ms": 2000,
  "approval_owner_wait_ms": 180000,
  "idle_ms": 1200000,
  "home": {
    "root": "/srv/hagency/state/agent-homes",
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

- **Loopback only.** The service refuses any listen address that is not loopback. The console checks the `Host` header, and that `Origin` is `http://<listen address>`, on login and on every write ([native/hagency/src/console.rs](native/hagency/src/console.rs)), so it cannot be put behind a reverse proxy. Open the console on the machine where Hagency runs. Reaching it from another machine is not a supported setup.
- **Console requests.** The console requires the exact `Host` header and refuses forwarded headers. Writes need a same-origin `Origin`. The operator API requires the same exact `Host`, refuses `Forwarded`, `X-Forwarded-For`, `Origin` and `Sec-Fetch-Site`, then compares the bearer token's SHA-256 in constant time.
- **Owner approvals.** Approvals come only from the owner's verified device. They arrive in an encrypted room whose only members are the owner and the approval bot. A third member or lost encryption disables the room. A failed delivery or an expired wait counts as a deny.
- **One approval device per owner.** Each owner's approval-bot device trusts only that owner. Owners' approvals stay isolated from each other.
- **Fenced runners.** Each dispatch has a capability and a fence number. A stale runner's calls are refused. A reply is published only after its process tree is proven gone.
- **Codex sandbox.** Codex runs with `workspace-write` and `on-request` approvals, no network, and only its workspace writable. Hagency checks the settings Codex echoes back.
- **Write-only credentials.** No route returns a stored token. The Palpo import's answer carries only public facts.
- **No coding-agent credentials.** The Setup page runs only the Codex binary it found on `PATH`, only with `--version` and `login status`, and records it by path and SHA-256. Hagency never runs a login and never reads or stores the agent's credentials. Every setup write needs the operator's console session.
- **Sign-in link stays out of logs.** `hagency start` prints the console link only to an interactive terminal. A service start writes only the `console-access` command to its log.
- **Hardened unit (installer only).** install-native.sh's systemd unit sets `NoNewPrivileges`, `ProtectSystem=full`, an empty capability set and a syscall filter. The per-user service from `hagency service install` has none of these settings; it runs with your user's rights.

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
