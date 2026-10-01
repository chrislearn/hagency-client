---
kind: decision
id: ADR-185
title: "An agent's own messages reach every device of its owner; approval cards stay with the verified ones"
status: Accepted
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [native, matrix, e2ee, approval, parity]
---

## Context

ADR-183 B made the owner's devices signed by the pinned anchor the only
recipients. Its stated reason is the approval card: "a card that names
commands and files is not readable by a device the owner has not verified".

The port applied that rule to every encrypted send, because one function
serves both the approval bot's cards and an agent's ordinary replies
(`encrypted_message::prepare`). Agents also enroll with sessions for signed
devices only.

On 2026-10-01 the owner wrote to agent lttleblue from Rinx, an unverified
device. After the agent was allowed to read that (commit 68084c4b), its
replies were still withheld from Rinx (`m.room_key.withheld`), so the owner
could not read them.

TS encrypts agent messages to every joined device (`RustEngine.js:86-104`,
default `EncryptionSettings`; missing sessions claimed per send). Its approval
room is separate from the
agent's room. The port is the same: cards go from the approval bot to its
own private room, and the agent's DM carries only the agent's own messages.

## Decision

1. **An agent's SDK shares its keys with every device of its owner whose
   device keys are consistent.** Each device must be self-signed and served
   by the homeserver for that user, as `keys::accept` already checks.
   Cross-signing by the owner's identity is no longer required.
   - Enrollment and the ADR-183 B re-check claim an Olm session for each
     such device.
   - The send uses `CollectStrategy::AllDevices`.
   - The recipient-set equality check stays: the devices actually sent to
     must equal the accepted set.
2. **The approval bot is unchanged.** Its cards and its intake keep
   ADR-183 B: only anchor-signed devices are recipients, and unsigned ones
   are excluded and counted.
3. **The owner's identity anchor is unchanged for both.** The owner's
   master key must still match the pinned anchor. An unknown user, a device
   whose keys are inconsistent, or an identity that does not match the
   anchor still refuses.

This amends ADR-183 B only for agent SDKs. Their incoming side (commit
68084c4b) and outgoing side now both follow TS.

## Consequences

Good, because the owner reads their agent from any of their clients, as in
TS, without verifying each client first.

Good, because approval cards keep the protection ADR-183 B chose for them.

Bad, because a compromised but unverified owner device can read the
agent's ordinary messages, as it could under TS. Commands still need an
approval card, which that device cannot read.

## Alternatives Considered

- **Verify every owner client.** This keeps the strict rule, but each new
  client silently loses messages in both directions until it is verified.
- **Share everything, cards included, with all devices.** This reverses
  ADR-183 B for the content it was written to protect.
