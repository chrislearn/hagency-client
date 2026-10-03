---
kind: decision
id: ADR-188
title: "An agent works in rooms it joins by invitation, without changing the identity it was created with"
status: Accepted
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [native, matrix, invites, rooms, parity]
---

## Context

Since ADR-187 slice 6a, an imported fleet's agent accepts its owner's invitations, and the console decides everyone else's. After it joins, though, the agent does nothing in the new room: it neither reads nor answers there. TS agents work in any room they have joined. In a group room they answer when mentioned, and a direct room is their conversation.

In the port, an agent's rooms are the ones it was created with: its DM with the owner and its project room. That list is fixed in three places:
- the agent's Matrix host configuration, whose room list is hashed into the **binding of its encrypted store**. Any change would refuse the store on the next restart (`Identity`).
- its **work-claim profile**, which may only act in those rooms.
- its **intake**, which only reads those rooms.

Encryption limits what any new room can be (ADR-184, ADR-185): an agent's encrypted store trusts a frozen user set of {the agent, its owner}. It can decrypt what others send it, but it encrypts **only for its owner's devices**. Today's project room is unencrypted for exactly this reason (`with_plaintext_project`).

## Decision

1. **The identity rooms never change.** The rooms an agent was created with stay its host configuration and its store binding, exactly as today. A joined room is never added there, so the agent's encrypted store keeps opening after any restart.

2. **A joined room becomes a stored room scope of the agent.** When the agent joins by invitation (trusted, or accepted in the console), Hagency records the room for that engagement in the store:
   - its privacy: a group room, or a direct room with the owner;
   - whether it is encrypted;
   - a room generation that advances on membership changes (ADR-153), as the project room's does.

   The agent's intake and work-claim profile read these scopes from the store on every pass, alongside the identity rooms. Re-attach after a restart rebuilds them from the store; nothing is lost.

3. **What the agent does there follows the project room's rules (TS parity):**
   - **Unencrypted group room:** a message that @mentions the agent wakes it. It answers in that message's thread and can read the room for context.
   - **Room whose only human is the owner,** encrypted or not: the owner's messages wake it, as in its DM.
   - **Encrypted room with other people in it:** the agent **does not work there**. Its replies could be read only by the owner (the frozen user set), and silently half-readable replies are worse than none. The agent stays joined and posts one plain notice saying it can't work in encrypted rooms with other members. The console shows the room as "joined · not working (encrypted, shared)". When the room's state changes, for example people leave so only the owner remains, the rule is re-evaluated.

4. **Approvals and tokens are unchanged.** Work in a joined room spends the same allocation. Its approval cards go to the owner's private approval room, never into the joined room.

5. **Leaving ends the scope.** When the agent leaves or is removed (a console decline, a kick, or the room closing), the scope is retired. Queued work for that room stays recorded (ADR-183) and is shown in the console; it is never dropped silently.

## Consequences

- Owners can bring an agent into any unencrypted room, or any room that is just them and the agent, and use it there, as in TS.
- An encrypted room shared with other people is a visible, named limit, not a silent failure. Lifting it would mean growing the agent's frozen user set, which is a separate decision on ADR-184/185.
- The identity and store binding stay fixed, so no restart or migration risk is added to existing agents.

## Alternatives considered

- **Add joined rooms to the host configuration.** This changes the store binding, so every restart after a join refuses the agent's store. Rejected.
- **Re-provision the agent with the new room list.** It would lose the agent's identity, history and grants on every invitation. Rejected.
- **Grow the agent's encrypted user set to all room members.** This is the real fix for shared encrypted rooms, but it reopens ADR-184/185's frozen set and its trust rules. It is left for its own decision.
