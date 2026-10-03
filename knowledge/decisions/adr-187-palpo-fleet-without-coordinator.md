---
kind: decision
id: ADR-187
title: "An imported Palpo fleet creates its agents and approval bot itself, with no coordinator agent; owner anchors are trusted on first use"
status: Accepted
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
amends: [ADR-102]
tags: [native, palpo, factory, approvals, crypto, parity]
---

## Context

Since hagency-rs #14 an operator connects a Palpo fleet from the console: they download the configuration in Palpo, import it in Hagency, and verify. Requests then reach the console. Approving one still creates no agent. In the Rust port, agents are created only inside a **coordinator**: a full Codex agent engagement that hosts these pieces:
- the provisioning host;
- the approval bot's anchor engagement;
- the membership sweep;
- the invite poller;
- a duplicate intake of reception requests.

The coordinator, its accounts and its `agent-driver.json` come from an uncommitted rig script (`palpo-coordinator.py`). That script logs in with the owner's password and reads the owner's cross-signing master key from the server.

TS has no coordinator:
- **Approval bot:** the bridge's own bot.
- **Agent creation:** the backend creates App Service agents on the operator's verdict.
- **Request intake:** the fleet request lane admits requests.

Research on 2026-10-02 found that provisioning uses only the fleet registration, the App Service token, a provisioning key and the representative credential. It needs nothing from the coordinator's identity. What it does need is a host to drive it.

Agents and approval cards must trust the owner's cross-signing master key (ADR-184, ADR-185). ADR-102 requires that key to be "obtained by the operator outside the Matrix key-query channel", and it rejects trusting server-provided master keys. No step in the console flow supplies it.

The operator chose, on 2026-10-02:
- to remove the coordinator (TS parity);
- to obtain the owner's master key by trust on first use.

## Decision

### A. The fleet service owns agent creation and approvals

1. **A fleet service.** Importing a Palpo fleet, or starting with one already imported, starts a fleet service from the fleet's own files: the registration, `palpo-appservice.json` and the transport. It needs no coordinator engagement, no `matrix` block, no `intake_sessions` and no hand-written `agent-driver.json`. The local Codex runtime settings stay operator configuration.
2. **A provisioning loop.** The fleet service runs agent provisioning on its own loop. Each pass resumes:
   - provisions waiting for their owner;
   - provisions approved in the console;
   - pending retirements.

   One engagement's failure is that engagement's alone (ADR-182). A failing pass is retried with backoff and never stops the service or the approval bot (ADR-183).
3. **The approval collector is anchored on the fleet**, not on a primary engagement: the fleet's registration and its approval bot MXID. Agents are added as they are created, as today. The "waiting for approval" notice in a project room is sent by the agent that is waiting, not by a coordinator (TS parity).
4. **One admission path.** Reception requests are admitted only by the Palpo request lane (TS `lib/fleet-protocol.js`). The coordinator's duplicate Matrix admission of reception events is not part of an imported fleet.
5. **Sweeps and invites move off the coordinator.** The membership sweep acts with the representative's credential. Each agent polls its own invites (TS `pollAgentInvites`).

### B. Hagency creates the fleet's own accounts

1. **The approval bot** `@<fleet>_approval` is created through the fleet's App Service: a masquerade `whoami`, then App Service login. Its device and SDK store are created once and reused across restarts, never rotated by a re-run.
2. **The representative** is the App Service's sender, `@<fleet>_representative`. Hagency uses one fixed representative credential for room work. That credential is part of existing room custody bindings, so it is not changed once chosen.
3. **Local keys are minted by Hagency:**
   - the approval bot's SDK key;
   - the agents' provisioning key;

   Each is random, private, created once, and never sent anywhere.

### C. Owner anchors: trust on first use (amends ADR-102)

1. **Pin on first need.** When Hagency first needs an owner's master key (to enroll an agent for that owner, or to send that owner an approval card), the representative reads it from the homeserver's key query. Hagency then pins it in the store, keyed by the owner's MXID.
2. **Pinned keys never change silently.** If the server later reports a different master key for that owner, Hagency refuses to replace it:
   - the agent or card waits;
   - the console shows the mismatch;
   - only the operator can re-pin it in the console.
3. **No key yet means wait.** An owner without cross-signing has no key to pin. Hagency waits, as for an owner who has not joined. It never treats "no key" as "no anchor needed".
4. **Pinned anchors replace the static list.** For an imported fleet, they replace ADR-102's static `peer_masters` list. A fleet may serve many owners, so the 16-entry limit applies per agent's frozen user set, not per fleet.
5. **What this amends.** ADR-102's rule that anchors come only from outside the key-query channel no longer holds for imported fleets. Its other rules stand:
   - fresh accounts;
   - frozen user sets;
   - verified recipients;
   - no impersonation for enrollment.

### D. Existing installs

An install configured with a coordinator keeps working unchanged until it is re-imported. This decision adds the fleet service; it removes nothing an existing `agent-driver.json` relies on. Removing the coordinator code path is a later, separate change.

## Consequences

- A newcomer's whole setup is done in the two UIs:
  1. import and verify;
  2. add a resource;
  3. a project requests an agent;
  4. approve it;
  5. the agent joins.

  No script, no command line, and no owner password is ever given to Hagency.
- The first time an owner's key is pinned, Hagency trusts the homeserver's answer. A homeserver that lies at that moment could make Hagency trust a key the owner does not hold. Every later change is caught. This is the same trade Matrix clients make when they first see a user. Choosing to paste the key in the console (option a) would avoid it.
- Approval notices in project rooms come from the waiting agent, as in TS, instead of a coordinator account.
- `!` commands are answered by each agent in its own rooms. There is no coordinator DM to send them to on an imported fleet.

## Alternatives considered

- **Paste the owner's master key in the console (ADR-102 as written).** It keeps the out-of-band anchor, but adds a security-settings step for every owner. Not chosen: the operator chose trust on first use.
- **Move the rig script into Hagency.** This keeps a coordinator TS does not have, and needs the owner's password. Not chosen.
- **Keep provisioning inside some agent's intake.** That would tie every agent's creation to one agent's health, which is the coupling this decision removes.

## Amendment 2026-10-02: one approval device per owner

Design work for slices 2, 4 and 5 found that an approval-bot device's encryption trust is frozen at its first enrollment (ADR-102's frozen user set, kept by §C.5: `sdk/enrollment.rs` refuses a different anchor list or user set). One device can therefore only ever serve the owners it first enrolled with. A Palpo server can have several project owners requesting agents from the same Hagency.

The operator chose, on 2026-10-02, **one approval-bot device per owner**:
- **§B.1 changes:** the approval bot `@<fleet>_approval` gets one device, with its own SDK store and key, **for each owner**. The device is created the first time that owner needs approvals, and reused from then on. Each device trusts only {the bot, that owner}. The bot account stays one account. "Created once" now means once per owner.
- The owner's private approval room is served by that owner's device. Isolation between owners is unchanged.

Two consequences recorded with this amendment:
- **An operator re-pin (§C.2) does not repair agents already enrolled.** Their frozen key list keeps the old key, so their sends to the owner fail until the agent is provisioned again. The same holds for that owner's approval device: a re-pin gives the owner a new approval device.
- **The approval pump is supervised.** A refused card must not stop approvals for every agent of the fleet: the pump restarts with backoff (ADR-183 decision 0).

