---
kind: decision
id: ADR-189
title: "Hagency ships as one binary, starts with one command and is set up from its web app; coding-agent sign-in stays with the user"
status: Accepted
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
amends: [ADR-187]
tags: [native, onboarding, console, installer, runtime, release]
---

## Context

An operator who wants agents on an imported Palpo fleet (ADR-187) today has to:

1. build Hagency and its console, which ship as a binary plus a separate folder of 163 files (6.6 MB);
2. run the installer, or `hagency init`, `hagency setup` and `hagency serve` with the right flags;
3. run `hagency console-access` to get a link into the console;
4. import the Palpo configuration in the console;
5. create the first resource with a `curl` call to the operator API, because the console can only copy an existing resource.

`hagency setup` removed the hand-written `fleet-runtime.json`, but the operator still works in a terminal for most of these steps. The console already does the rest: it imports Palpo, approves agents, manages tokens, resources and invitations. The fleet service already waits for `fleet-runtime.json` (stage `awaiting_runtime_config`, polled every 5 s), so writing that file while the service runs takes effect without a restart.

Hagency runs on the same machine as the coding agent and the operator uses the console on that machine.

Hagency talks to Codex by starting `codex app-server` and speaking its JSON-RPC protocol over stdio (`hagency-execution/src/host.rs`, `hagency-runtime/src/codex/`). Codex reads its sign-in from its own folder (`CODEX_HOME`, `~/.codex` by default). Signing in is between the user and the coding agent; it is out of Hagency's scope.

## Decision

1. **One binary.** The console build is embedded in the `hagency` binary at compile time. `--console-assets DIR` stays as an override for console development. Releases ship one binary per platform (macOS arm64 and x86-64, Linux x86-64 and arm64).

2. **One start command.** `hagency start [--state-dir DIR] [--listen ADDR] [--no-open]`:
   - uses a default state directory (`~/Library/Application Support/Hagency` on macOS, `~/.local/share/hagency` on Linux) when none is given;
   - initializes it when it is new or empty;
   - serves as an imported fleet (`serve --palpo-transport`, no `--agent-driver`) with the embedded console;
   - starts with no `fleet-runtime.json` and no Palpo import. Both are completed from the web app.

   Coordinator installs keep `hagency serve` with their existing flags.

3. **Service registration from the binary.** `hagency service install` registers a per-user service that runs `hagency start` at login (macOS LaunchAgent) or as a `systemd --user` unit (Linux), and starts it. `hagency service uninstall` removes it. The service runs as the user, so it sees the user's coding-agent sign-in. `install/install-native.sh` is retired for fleet installs.

4. **First access without a second command.** At start, the server issues a console sign-in link and prints it. When started from a terminal on a desktop session, it also opens the link in the default browser; `--no-open` turns this off, and a service start never opens a browser. `hagency console-access` stays, to print a new link.

5. **A setup page in the console**, shown until setup is complete and reachable later from the console menu:
   1. **Coding agents.** Hagency detects the coding agents installed on this machine and shows each one's path, version and whether it is signed in. Codex is first; Claude Code and Octos are added later, each through its own ADR.
      - Detection runs only binaries found on `PATH` (or chosen by the operator in the page), only with their version flag and their own read-only sign-in status command (`codex login status`), and records them by path and SHA-256.
      - Hagency only asks the agent whether it is signed in, and how. It never runs a login, never reads credential files and never stores credentials. Asking the agent also covers sign-ins kept in the system keychain. If the agent is not signed in, the page asks the user to sign in themselves (for Codex: `codex login` in a terminal) and click **Check again**. If it is not installed, the page asks the user to install it.
      - When a signed-in agent is found, Hagency writes and validates its runtime configuration itself (the `hagency setup` code and defaults, checked by the service's own loader). There is no settings step; the CLI remains for changing the defaults.
      - The page shows how Codex is signed in, a ChatGPT plan or an API key, as `codex login status` reports it. With a plan sign-in, it notes that plan sign-ins are meant for personal use before the operator offers the agent to other people. It never blocks.
   2. **Connect Palpo.** The existing import page: upload the configuration file downloaded from Palpo's web admin, then verify the connection in Palpo.
   3. **Offer a resource.** A button creates the first resource from the configured agent, through the same writer as `POST /api/native/v1/resources`, with the agent's seat. It offers only model and reasoning pairs Hagency qualifies (`role-capacity.json`), so the resource is published to Palpo.

6. **Everything after setup happens in the web app.** No operator script or CLI step is needed for normal operation.

7. **The CLI stays for automation and recovery.** `hagency setup`, `hagency serve`, `hagency console-access` and the operator API keep working; nothing requires them.

## Security

- The console stays local-only, with its existing `Host` and `Origin` checks. Every setup write needs the operator's console session.
- The server runs no command typed in the page. It runs only a detected or operator-chosen agent binary, only with its version flag, and records its SHA-256.
- Hagency never handles a coding agent's credentials; it only checks that they exist.
- A browser is opened only for an interactive start, never for a service start.

## Consequences

- An operator's terminal work becomes: sign in to the coding agent, then `hagency start` or `hagency service install`. The rest is in the browser.
- `hagency setup`'s code is the engine behind the setup page.
- The release is one file per platform, which is also the base for a joint Hagency and Rinx release.
- Adding Claude Code or Octos later means a detector and a runtime for it, without changing this flow.

## Slices

1. Embedded console; `hagency start` (default state directory, initialization, fleet serve without configuration, printed and opened sign-in link); `hagency service install` / `uninstall`.
2. Setup page and API: coding-agent detection and status, sign-in kind, and automatic runtime configuration.
3. Offer-a-resource button.
4. Release workflow producing one binary per platform; documentation (README, user guide, walkthrough).

## Alternatives considered

- **Hagency runs the coding agent's login.** Rejected: sign-in is between the user and the agent, and Hagency would handle credentials it does not need.
- **A runtime-settings step in the page.** Rejected: every value is detected; asking the user to confirm defaults adds a step without a decision.
- **Keep the shell installer.** Rejected: a per-user service registered by the binary removes the separate script and runs as the user who signed in to the agent.
- **A remote console with TLS and its own logins.** Not needed: Hagency runs on the machine the operator uses.
