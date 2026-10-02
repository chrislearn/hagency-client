---
kind: decision
id: ADR-184
title: "A provisioned agent publishes its keys before anyone can write to it: enroll alone, then invite the owner"
status: Accepted
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [native, matrix, provisioning, e2ee, enrollment]
---

## Context

`appservice_login_home_rooms_enrollment_step_v1` (ADR-147) runs its stages
in this order:

1. The agent creates its encrypted owner DM with the owner already in `invite`.
2. The agent joins the project room.
3. The job waits for the owner's actual join.
4. Only then does pre-activation SDK enrollment (ADR-102) upload the agent's
   device keys and cross-signing identity. `current_at` refuses `Recipients`
   until the owner is joined in both rooms.

Between the owner's join and the key upload, the agent has no device keys.
The owner's client encrypts to the devices it can see, which is none of the
agent's. So anything the owner writes in that window never reaches the agent.
Matrix clients do not re-share a room key with another user's device later.
The loss is permanent.

Observed live on 2026-09-30 on the mini1 Palpo:
- Agent maya's owner joined the DM at 21:04:03 and wrote at 21:04:29 and
  21:04:43.
- maya uploaded its device keys at 21:04:48.
- Both messages stayed undecryptable. No turn ran and nothing was reported.

TS has no such window. Its agent DMs are plaintext (`bridge-matrix.js:10061`),
and its encrypted approval bot holds keys from startup. The operator chose to
keep encrypted agent DMs and close the window, rather than follow TS to
plaintext.

## Decision

The agent is enrolled before the owner is invited. Its enrolled user set
already names the owner, so nothing about the set changes when the owner
joins.

1. **The DM is created with no invite.** It keeps its preset, encryption,
   `history_visibility: invited` and power levels.
2. **The agent's project-room join is unchanged.** The representative
   invites, then the agent joins.
3. **Enrollment runs before the owner is invited.**
   - The frozen user set is the agent plus the owner, and the owner is
     anchored by the pinned master key. It also includes the joined members
     of an encrypted group room, as before.
   - Enrollment needs no room membership to do its work. It reads the
     owner's keys through `/keys/query`, checks them against the anchor and
     claims Olm sessions for the owner's signed devices.
   - The DM check in `current_at` requires the agent joined and no other
     joined member except the owner. The owner may be absent (before the
     invite), invited (a resumed wait) or joined (the pre-activation
     re-check). A left or banned owner refuses. Every provisioning attempt
     re-runs this check, so it must hold at each of those points. The owner
     is joined before activation because the owner stage (step 4) returns
     only once the owner has joined.
   - The ledger's user set is frozen as ADR-102 requires. The pre-activation
     re-verify sees the same set after the owner joins, so ADR-183 B only
     adds devices that appeared in the meantime.
4. **The owner is invited by new create-only custody stages.**
   - The rooms step ends with `agent-rooms` once the agent-only DM exists and
     the agent has joined the project room.
   - After enrollment, the owner stage records `owner-invite-possible`, then
     `owner-invite-response`, then `complete` once the owner has joined.
   - A lost invite response is inspected with a GET and never repeated.
   - The owner-join wait moves after the invite and keeps its no-deadline
     rule.
   - As before, only the job that saw the wait may resume it. On disk, an
     invite without `complete` could be a wait or a completed custody that
     lost its last record, and a restart during the wait stays with the
     operator.
5. **Activation stays behind the second, GET-only enrollment check.**
   `finish_factory` already runs it after the owner has joined, before
   activation.
6. **Rooms custody from before this change stays valid.** It has
   `complete` and no owner-invite stages, and its DM already holds the
   owner. Re-attach accepts it unchanged. Provisions still in flight were
   not considered: none exist on any rig.

The check cannot tell a resumed wait's `invite` from an owner who joined,
left and was re-invited before activation. Both are accepted, and the owner
stage's own join check is what holds activation back.

An encrypted project room keeps a smaller version of the same window. The
owner can post there between the agent's join and its key upload. Project
rooms on the Palpo rig are plaintext, so this is noted and not addressed.

## Consequences

Good, because the owner's first message is always encrypted to the agent's
device: the device exists and is published before the owner can write.

Good, because nothing is weakened. The anchors, the frozen user set, the
fresh-account check, the recipient rule and the approval cards are unchanged.
Only the moment the owner is invited moves.

Bad, because the rooms custody gains two stages and the owner-join wait
moves out of the rooms step.
Every existing selector that asserts the old order must change, and resumption
must tell the two orders apart.

Bad, because the owner sees the DM invite a few seconds later, after
enrollment completes.

## Alternatives Considered

- **Plaintext agent DMs with a separate encrypted approval room (TS
  parity).** Removes the race altogether. The operator declined it on
  2026-09-30 and chose to keep agent DMs encrypted.
- **A "ready" message after enrollment.** It tells the owner when writing
  becomes safe, but messages sent before it are still lost.
- **Requesting lost room keys afterwards.** Clients do not answer key
  requests from another user's devices, so this recovers nothing.
