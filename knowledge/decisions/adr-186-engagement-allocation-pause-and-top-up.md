---
kind: decision
id: ADR-186
title: "An engagement's token allocation is chosen at approval, pauses its agent when used up, and can be topped up"
status: Accepted
satisfies: [REQ-RUST-MIGRATION-EXECUTION]
tags: [native, engagements, allocation, quota, console, parity]
---

## Context

When an engagement is approved, the tokens it may use come out of its resource's ceiling. TS (`POST /api/engagements/:id/verdict`) lets the operator approve a different amount with `allocatedTokens`; the native console's approve route takes no amount, so it grants exactly the requested tokens or refuses with `over_commit`. It also drops the refusal's explanation.

In both TS and native, nothing stops an agent when it has used its allocation. Ceilings are checked only at approval time. Afterwards, an over-limit resource only raises an `agent_ceiling_overrun` alert ("Alerts inform; never stop work"). No route raises a running engagement's allowance either: in TS the operator had to revoke the engagement and approve a fresh request.

On 2026-10-01 the operator asked for two things:
- approving with an adjusted amount, including "give all the remaining quota to this agent";
- an agent that has used its quota pauses, and resumes when tokens are added.

## Decision

### A. Approval chooses the amount (TS parity, plus "all remaining")

1. `POST /console/api/engagements/{id}/approve` accepts an optional `allocatedTokens` (a positive integer). If it is absent, the requested amount is granted, as today.
2. The amount is checked against what the resource can still give: the smallest of ceiling, seat and pool headroom, using the same rule approval uses today. A larger amount is refused with `over_commit` or `insufficient_capacity`, and the response carries the human message naming the binding limit.
3. The candidate read returns `remainingTokens` as that same smallest headroom, not the ceiling alone. The console's "All remaining" button fills in exactly this number.
4. The engagement row gains `allocated_tokens`. Reservation, draw, side commitment and the figure published to Palpo (`allocatedTokens`) use the allocated amount whenever one is set. `requested_tokens` is kept unchanged as the requester's ask.

### B. A used-up allocation pauses the agent; it never ends the engagement

1. An engagement's **spend** is the sum, over every period since approval, of its known fresh usage: input + output + cache writes. Cache reads are not counted, as with ceilings today.
2. When spend reaches the allocation, the engagement enters the `quota_paused` hold:
   - the turn already running finishes;
   - no new turn is dispatched for it;
   - work that arrives meanwhile stays queued. Nothing is dropped, refused or marked done (ADR-183 rule: the bridge never decides done).
3. When the hold begins, the agent posts one notice through the existing task-notice path. It lands in the project room, in the thread of the task it concerns: "Paused: used N of M tokens. The owner can add tokens in the Hagency console." A session that received no request from Matrix has nowhere to post, so no notice is sent there.
4. Unknown usage never pauses an agent. Spend is the known lower bound of usage. It counts as unknown only when no period carries a known count; in that case the agent keeps running and the console shows its spend as unknown. (Live metering marks every observation incomplete, so "incomplete" alone cannot mean "unknown".) A lower bound can delay a pause but never cause a false one.
5. The hold is checked whenever usage is recorded. An engagement that is already over its allocation pauses at its next usage report.
6. This makes host-attributed usage, which until now was diagnostic only, the trigger for the pause. That is a deliberate change to the usage ledger's "never execution or quota authority" rule. It is limited to this pause: it never revokes an engagement, never ends one, and never refuses admission.

### C. Topping up resumes the agent

1. `POST /console/api/engagements/{id}/allocation` with `{commandId, addTokens}` raises the allocation of a reserved, active or paused engagement by `addTokens`. The console's "All remaining" option adds the resource's current headroom.
2. The increase is checked exactly like an approval (step A2), against the headroom left after this engagement's current allocation.
3. When the new allocation is above the spend, the `quota_paused` hold lifts at once. Queued work is dispatched without a restart, and the agent posts "Resumed: N tokens available." in the thread of the oldest queued task, or otherwise where the pause was announced.
4. Palpo receives the new `allocatedTokens` with the next status update.
5. Raising the resource ceiling does not top up any engagement by itself. It only makes room for a top-up or a new approval.

## Consequences

- The operator can approve within the limits instead of refusing, and keeps an agent working by adding tokens rather than revoking it and re-approving.
- Agents now stop taking new work when their allocation is used up, where before they ran on with only an alert. The pause is visible in the console (Workforce and Engagements show "paused: quota") and in the project room.
- Usage reporting gaps can delay a pause, because spend is only as current as the last observation, but they can never cause a false one.
- The schema change is one nullable column, `engagements.allocated_tokens`. Existing engagements keep their requested amount as their allocation.
