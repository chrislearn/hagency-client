# Matrix-facing parity audit: TypeScript bridge vs Rust port

The Rust port covers the core loop: wake rules, thread intake, the approval card, verdict checks, provisioning and final replies. It has none of the "your message was received and is being worked on" signals, no `!` commands, and none of the legacy group-room management. Several wire formats also differ from what existing clients and Palpo tooling send, and three of those have no ADR behind them. I edited nothing.

**Paths:** `TS:` means `TS-repo/`. `RS:` means `RS-repo/native/`. ADRs are in `RS-repo/knowledge/decisions/`.

**Status rule:** CHANGED-ON-PURPOSE is used only where a Rust ADR states the change. ADR statuses as I found them: ADR-054, -059, -064, -074, -095, -098 and -174 are Accepted; ADR-147 is Decided; ADR-137, ADR-132 and ADR-183 are **Proposed**.

## A. Custom message and event types

| behavior | TS file:line | Rust status | Rust file:line | operator/user impact |
|---|---|---|---|---|
| Owner approval card `com.agentchat.approval.request.v1` sent to the owner DM | TS:bridge-matrix.js:2600-2646, 9341-9355 | PORTED | RS:hagency-store/src/domain/approvals/card.rs:60-115; RS:hagency-matrix/src/approval_delivery.rs:84-150; caller RS:hagency/src/bootstrap/approval.rs:224 | Works. Differences: `Runtime: codex` is hardcoded (card.rs:95,108); `approve_task` appears whenever a reusable scope exists (TS also required a task id, :2617); request ids are 40 hex instead of 32 (ADR-143:62-66) |
| Public "waiting for owner" notice `com.agentchat.approval.status.v1` in the project room | TS:bridge-matrix.js:2578-2598, 9377-9390 | PARTIAL | RS:hagency-matrix/src/approval_delivery/public.rs:12-80; approval_delivery.rs:157-200 | The only caller is a test (RS:hagency-matrix/tests/approval_delivery/pc_c1.rs:63). The wire shape also differs: it is sent as a custom **event type** (PUT `send/com.agentchat.approval.status.v1`), not an `m.room.message` msgtype. The payload key is `com.agentchat.approval.status` (TS uses `com.agentchat.approval`), there is no thread relation, and it comes from the bot rather than the agent. Project members get no approval-wait signal |
| Verdict intake, current shape (`com.agentchat.approval.verdict.v1` + `com.agentchat.approval`) | TS:bridge-matrix.js:2648-2689 | PORTED | RS:hagency-matrix/src/approval_batch.rs:604-650 | Stricter: edits are refused (:620-623) and only the exact content keys are allowed (:624) |
| Legacy verdict shape `com.hagency.approval.verdict.v1` / `com.hagency.approval` | TS:bridge-matrix.js:216,220,2661-2662 | CHANGED-ON-PURPOSE (ADR-064:84-86) | not accepted | Old clients that still send the legacy shape cannot approve anything |
| Agent-ops client session request/grant/revoke msgtypes | TS:bridge-matrix.js:221-224, 2695-2737, 7243-7324 | MISSING | none | The encrypted Agent Ops bootstrap over Matrix is gone. In TS it was behind `HAGENCY_AGENT_OPS_CLIENT`, off by default (:2691-2693) |
| Per-dispatch activity notice: `io.hagency.activity` + `m.notice`, edited in place via `m.replace`/`m.new_content` (⏳/⏸️/✅/⚠️, elapsed time, tool counts, 5 s/30 s coalescing, older pending notices superseded) | TS:lib/matrix-activity.js:1-13; TS:router/src/activity.ts:33-77; TS:router/src/store.ts:2398-2428, 2755-2761, 2841; lifecycle hooks :1984, 2048, 2232, 2262, 2624, 2679-2686; TS:bridge-matrix.js:6014 | MISSING (confirmed) | none. Acknowledged at `RS-repo/docs/progress.md:11720-11722` and ruled out of scope at `specs/task-rust-bridge-never-decides.spec.md:181`. ADR-052:21 and ADR-056:17-18 say the native progress kernel "does not complete ADR026's editable, durable Matrix status" | Users see nothing between their message and the final answer, including no "paused for owner approval" and no "interrupted" |
| Engagement request `com.hagency.engagement.request.v1` | TS:lib/fleet-protocol.js:4, 154-193 (a custom event type, verified through HTTP `POST /api/fleet/v1/requests`) | CHANGED-ON-PURPOSE (ADR-095:486-498) | RS:hagency-matrix/src/event_batch.rs:406-435 | The carrier is now an `m.room.message` msgtype with a JSON body. Requesters that emit the old event type are not recognized |
| Provider engagement verdict `com.hagency.engagement.approval.v1` (Rust only) | none; TS used HTTP `/api/engagements/:id/verdict` | CHANGED-ON-PURPOSE (ADR-143 amendment :117-130, ADR-147) | RS:hagency-matrix/src/event_batch.rs:406-409 | New wire kind, accepted only from the representative |
| Connection probe `com.hagency.connection.probe.v1` and `/api/fleet/v1/probe` binding of the reception room | TS:lib/fleet-protocol.js:98-153 | PARTIAL | reception room is set by an offline command, RS:hagency/src/bootstrap/registration.rs:1-27 | No in-band proof of the reception room; an operator must write it by hand. No ADR |
| Project binding state event | TS:lib/fleet-protocol.js:52 (`com.hagency.admin.binding.v1`, state_key = fleetId) | PARTIAL (diverges, no ADR) | RS:hagency-matrix/src/collector.rs:735-740; RS:hagency/src/bootstrap/provision.rs:306-311 (`com.hagency.project.binding.v1`, state_key `""`) | A project room set up for the retained protocol will fail target verification |
| Inbound edits (`m.replace`) never become input | TS:bridge-matrix.js:3294-3297 | PORTED | RS:hagency-matrix/src/event_batch.rs:436-447 (non-thread rel_type is `Unsupported`) | Same outcome |
| Markdown to `org.matrix.custom.html` on text/notice | TS:lib/matrix-markdown.js:10; TS:bridge-matrix.js:10750 | PORTED (ADR-050) | RS:hagency-matrix-format/src/lib.rs:60-90 | — |
| Router thread/task notices | TS:bridge-matrix.js:6015 (sent as `m.text`) | PORTED | RS:hagency-matrix/src/outgoing.rs:232-236 (sent as `m.notice`) | Cosmetic msgtype difference |

## B. Bot commands (TS:lib/bot-commands.js:30-59, dispatch :364-389)

No `!` parser exists anywhere under RS:. One side effect: in a direct room a `!…` message is now ordinary text and wakes the agent (RS:hagency-store/src/domain/verified_ingress.rs:650-651).

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| `!request` (tier 0) | :525-629 | CHANGED-ON-PURPOSE (ADR-095:492-498 moves the `/api/engagements` body into an event) | event_batch.rs:406-435 | Typing `!request` now gets no reply |
| `!offer` (tier 0) | :469-524 | MISSING | none | Borrowers cannot see published offers before requesting |
| `!help`, "Unknown command", bot-DM fallback "Send !help…" | :326, 388, 630-693; bridge :7140-7160 | MISSING | none | No discoverability |
| Tier-1 reads: `!status !agents !groups !group !agent !sessions !mcp !bridge` | :694-968, 1371 | MISSING | none | Operators must use the console |
| Tier-2 admin: `!mkgroup !bindroom !addmember !rmember !joingroup !dm !identity !rmgroup` | :970-1370 | MISSING | none | Group management from chat is gone |
| Tier-3 terminal: `!spy !agentctl !ctl` | :1302, 1492 | MISSING | none | tmux-based; moot for native runners |
| Command ACL tiers, fail-closed unconfigured ACL, tier0Only refusals, replies via `sayInRoom` with deterministic txn | :84-101, 337-361, 397-437 | MISSING | none | Moot without commands |

## C. Thread notices other than activity

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Lifecycle notices in the task thread | TS:router/src/store.ts: 1525-1531 session_quarantined, 1543-1547 task_blocked, 1580-1581 completed_task_followup, 1603-1607 workspace_quarantined, 1619-1623 waiting_for_approval, 2346-2352 and 2651-2655 outcome_unknown, 2542-2546 runner_launch_failed, 2586-2590 runner_launch_retry, 3225-3229 outcome_resolved, 1203-1206 thread_delivery_failed, 1242-1246 start failure, 3314-3315 "Task status: X" | PARTIAL | RS:hagency-store/src/domain/task_intents.rs:64-79 (session_quarantined), 120-134 (outcome_unknown), 390-397 ("Task created"), 526-533 ("Continuing task"); attempt_events.rs:162-175, 334-380 (over-budget notice, which is Rust only) | No notice for blocked, queued behind approval, workspace quarantined, launch failure/retry, outcome resolved, or task status. Users see silence |

## D. Presence signals

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Typing indicator as the agent (45 s timeout, 30 s refresh, 2 min cap, cleared on send) | TS:bridge-matrix.js:354-367, 10527-10559, 10603-10668, 10884-10885 | MISSING | none (no `typing` under RS:) | No "working" signal |
| 👀 `m.reaction` acknowledgement after the backend accepts a message | :369, 10568-10594, 10608 | MISSING | none | No proof the message was received |
| Read receipts | not sent by TS | N/A | — | — |

## E. Threads, replies, mentions and inbox rules

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Wake rules: DM from the bound human; group needs an @mention of the agent; agent and service senders never wake | TS:bridge-matrix.js:6887-6894, 7075-7105 | PORTED | RS:hagency-store/src/domain/verified_ingress.rs:643-653 | A human `m.notice` is admitted but never wakes (TS treated it as text, :3310) |
| Mention fallback to HTML pills or plain `@name` | :3133-3174 | CHANGED-ON-PURPOSE (ADR-054:70-72) | event_batch.rs:484-496 (`m.mentions.user_ids` only) | Clients that do not set `m.mentions` cannot wake agents |
| Inbound thread root capture; unaddressed thread replies kept but not woken | :3298-3301, 7089-7095 | PORTED | event_batch.rs:436-459 | — |
| Mention inferred from a reply in legacy groups | :4180, 7033-7044 | MISSING | none | Legacy group rooms only |
| Outbound reply relation | :3318-3393 (thread: `m.thread` + fallback; top level: `m.in_reply_to` to the question; incidental messages start a thread) | PARTIAL | outgoing.rs:234-236 (thread root only) | Top-level answers in a group are not linked to the question |
| Completed-task follow-up (only the original requester reopens) | TS:router/src/store.ts:1560-1581 | PARTIAL | verified_ingress.rs:655-665 | The rule is ported; the explanatory notice to other members is missing |
| Opt-in default recipient (`MATRIX_DEFAULT_WAKE=auto`) | :432-434, 7166-7173 | MISSING | none | Opt-in only |
| Delivery-feedback notices (⚠️ not delivered / offline / unknown mention / not in group / failed after retry) | :6492-6572 | MISSING | none | Failures are silent in the room |
| Loop prevention (agents, representative, bot, ignored senders) | :6944-6990 | PORTED | verified_ingress.rs:643 | — |
| Discussion history backfill since admission | :6833-6871; TS:lib/matrix-direct-chat.js:193-218 | CHANGED-ON-PURPOSE (ADR-054:73-75) | none | Pre-session context is not archived |
| Inbound dedup and redelivery | :4096-4128 | PORTED | receipts/dispositions, RS:hagency-matrix/src/event_batch.rs:687-699 | — |
| Retry of undecryptable events when late room keys arrive | :6646-6705 | CHANGED-ON-PURPOSE (ADR-065:93; ADR-064:155-157) | event_batch.rs:354 (terminal rejection) | A late key loses the message |
| Room trust gate (audit/enforce); rooms not selected are ignored | :6992-6997, 7359-7364 | CHANGED-ON-PURPOSE (ADR-054:76-78) | frozen targets, event_batch.rs:448-465 | Stricter |

## F. Engagement, provisioning and owner join

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Reception request verification (requester and representative joined, target binding/powers, private owner DM) | TS:lib/fleet-protocol.js:48-68, 154-193 | PORTED | RS:hagency-matrix/src/intake.rs:275-320, 405-441; collector.rs:673-740 | — |
| Agent account minted through the appservice token | TS:lib/matrix-work-executor.js:26-40 | PORTED | RS:hagency-matrix/src/token_provision/application_service.rs | — |
| Owner DM creation | TS:bridge-matrix.js:8772-8797 (bot creates it: topic, agent invited, locked power levels) | PARTIAL | RS:hagency-matrix/src/token_provision/rooms.rs:408-412 (agent creates it; encrypted, `m.federate:false`, history "invited") | Different creator and no lockdown |
| Approval DM power-level lockdown (`approvalRoomPowerLevels`, `ensureApprovalDmRestricted`) | :2561-2576, 8861-8887 | MISSING | none | Owner and agent keep default powers |
| Representative invites the agent into the project room; agent joins | TS:lib/matrix-representative.js:933, 1191 | PORTED | rooms.rs:425-470 | — |
| Wait for the owner to join | :7484-7486, 8849-8858 | PORTED | rooms.rs:489-500 | — |
| Owner leaves the approval DM, so bindings are removed | :7487-7489 | PARTIAL | fence on negative room evidence (ADR-183 amendment, ADR-137:131-140) | — |
| Warning when the owner is not in the approval room | :9267-9297 | MISSING | none | The operator never learns the card went unseen |
| "Approved" receipt (`m.notice`) sent by the representative to the request room | TS:lib/engagement-notice.js:2-20; TS:lib/matrix-work-executor.js:8-12 | MISSING | none | Borrower is never told the request was approved or which agent serves it |
| Retirement: agent leave/logout, representative logout | TS:lib/matrix-work-executor.js:13-49 | MISSING | the retire route records an effect only (RS:hagency/src/console/engagements.rs:142); no leave/logout call under RS: | Retired agents stay in rooms |

## G. Approval lifecycle

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Deny if delivery fails | TS:bridge-matrix.js:9392-9405 | PORTED (ADR-137) | approval_delivery.rs:144 | — |
| Verdict bound to request, digest, agent, project, room, owner and scope | TS:lib/approval-store.js:596-680 | PORTED | approval_batch.rs:629-641 | — |
| Owner device trust for verdicts | TS: none required (device self-signature only for agent-ops, :7277-7288) | CHANGED-ON-PURPOSE (ADR-064:84-86; ADR-183 §B) | approval_batch.rs:247-253 | Only verified, cross-signed owner devices can approve |
| Card recipients | TS: every joined device (ADR-183:101-104 cites `RustEngine.js`) | CHANGED-ON-PURPOSE (ADR-183 §B, ADR-137 amendment) | approval_delivery.rs:410 onward | Unverified owner devices cannot read cards |
| Expiry decline | TS:lib/approval-store.js:253-262 | PORTED | RS:hagency-execution/src/approval/control.rs:132-175; RS:hagency-store/src/domain/approvals.rs:19, 654 | Nothing is posted to Matrix in either version |
| Rejected verdicts are final; transient failures can be replayed | :7336-7346 | PORTED | ADR-064 tombstones | — |

## H. Project side and appservice

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Registration issue (YAML with as/hs tokens) | TS:lib/appservice-receiver.js:70-143 | MISSING (absence recorded in ADR-132:20-28, Proposed) | none | Cannot onboard a new side from Rust |
| Install/verify the credential; staged replacement | TS:lib/project-side-store.js:509-560; TS:backend-v2.js:9931-9947 | MISSING | none; RS:hagency/src/console/project_sides.rs:26-45 lists these fields as unavailable | No credential rotation |
| Fleet registration record | TS:backend-v2.js `/api/project-sides` | PORTED | RS:hagency/src/bootstrap/registration.rs:1-60; project_sides.rs:101-140 | — |
| Appservice transaction receiver/listener | TS:lib/appservice-receiver.js:145-348; TS:lib/appservice-listener.js:89 | MISSING | none; intake is each agent's own SDK `/sync` (ADR-047/054) | No push intake |
| Appservice sync collector | TS:lib/appservice-sync.js:59-99, 192+ | PARTIAL | `/sync` as the agent device, RS:hagency-matrix/src/collector.rs | — |
| Edge/puller outbound link | TS:lib/appservice-edge.js:59; TS:lib/appservice-puller.js:85 | PARTIAL | RS:hagency-palpo/src/adapter.rs:64, 306; the Matrix lane is stored but never handed to intake (ADR-037:75-79 "future Matrix adapter") | Delivered events are never routed |
| Re-invite and rejoin the agent when a send fails on membership | TS:bridge-matrix.js:10888-10950 | MISSING | the outgoing preflight refuses (outgoing.rs:222) | A kicked agent goes silent |

## I. Files

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Outbound `m.image`/`m.file` with guessed MIME, also into plaintext rooms | TS:bridge-matrix.js:10998-11036, 5978-6000 | CHANGED-ON-PURPOSE (ADR-098:40-45: always encrypted, `application/octet-stream`) | RS:hagency-matrix/src/upload/publication.rs | No image previews; no files in plaintext project rooms |
| Inbound attachments in encrypted rooms plus `receive_file` | :6799-6831, 7017-7020 | PORTED (ADR-105) | event_batch.rs:497-517 | — |
| Inbound attachments in plaintext rooms | same | CHANGED-ON-PURPOSE (ADR-074:57-59) | event_batch.rs:504-506 | Rejected |
| "⚠️ Attachment not delivered" notice | :11045-11052 | MISSING | none | — |
| Legacy `LocalPath` media caching | :7021-7022, 4136 | MISSING | none | Legacy |

## J. Rooms and membership

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Room name creates or remaps a group | :7367-7453 | MISSING | none | Legacy groups |
| Member events sync group membership | :7455-7519 | MISSING | none | Legacy groups |
| Bot join/leave maps or unmaps the room | :7466-7479 | MISSING | none | Legacy groups |
| Tombstone migration | :7522-7563 | MISSING | none | Upgraded rooms are lost |
| `createRoomForGroup`, member changes invite/kick | :11057-11103, 9606 | MISSING | none | Legacy groups |
| Humans inviting agents into new DMs/rooms; DM-to-group promotion guard | TS:lib/matrix-direct-chat.js:163-183, 279-292 | MISSING | routes come only from provisioning or factory | Users cannot start new chats with an agent |
| Bot/agent pending invites, operator accept/reject | :7894-8131, 8949-9153 | MISSING | none | — |
| Greeting humans, bot DMs, reaping dead DMs | :6078-6293 | MISSING | none | — |
| Avatars | :1924-2030 | MISSING | none | Cosmetic |
| Display name set from the agent definition | :5938-5955 | MISSING | none (the DM room *name* is set, rooms.rs:410) | Agents show raw MXIDs |
| Room state observation (members, join rules, encryption, power levels, name) | TS:lib/fleet-protocol.js:48-68 | PORTED | collector.rs:640-740 | Note: `matrix_user(key, server)` at :674 refuses members from another server |
| Operator alerts posted in rooms (blocked, recovered, compact, system info) | :9408-9580 | MISSING | none | — |

## K. Retry and backoff

| behavior | TS | status | Rust | impact |
|---|---|---|---|---|
| Sync/refresh failure: 1 s doubling to 60 s, reset on success; only 401/403 are fatal | TS:lib/appservice-sync.js:12, 73; TS:bridge-matrix.js:1647-1649 | PORTED (ADR-183 A, recent; ADR-174 amendment) | RS:hagency/src/bootstrap/driver.rs:311-377 | Worker state becomes `refresh_refused` instead of dying |
| 429 handling | TS:bridge-matrix.js:261-279 (6 tries, shared cooldown) | CHANGED-ON-PURPOSE (ADR-174:14-23) | RS:hagency-matrix/src/http.rs:505-518 (4 attempts, GET only; writes are single-attempt) | — |
| Outbox send retry | :6030-6060 (non-permanent failures retried every poll) | CHANGED-ON-PURPOSE (ADR-059:123-134) | outgoing.rs:44-80 (45 s bound; uncertain sends are never re-sent) | A lost send stays "Uncertain" until an operator inspects it |

## (1) Counts (79 rows; read receipts marked N/A and not counted)

| status | count |
|---|---|
| PORTED | 21 |
| PARTIAL | 10 |
| MISSING | 34 |
| CHANGED-ON-PURPOSE | 14 |

## (2) Top 20 MISSING/PARTIAL behaviors, ranked by user impact

1. Activity notice (⏳/⏸️/✅, edited in place) is missing: users get no progress between their message and the answer (TS:router/src/activity.ts:33-77).
2. Typing indicator is missing: no sign the agent is working (TS:bridge-matrix.js:10527-10668).
3. 👀 acknowledgement reaction is missing: no proof the message was received (:10568-10594).
4. Public "waiting for owner approval" notice is not wired (test-only caller) and uses a different wire shape (RS:hagency-matrix/src/approval_delivery.rs:157).
5. Thread notices for blocked, queued-behind-approval, workspace-quarantined, launch failure/retry and outcome resolved are missing (TS:router/src/store.ts:1543-1623, 2542-2590, 3225).
6. Delivery-failure and offline-mention feedback notices are missing (:6492-6572).
7. Humans cannot invite agents into new DMs or rooms (TS:lib/matrix-direct-chat.js:163-183).
8. No "Approved, mention @agent to begin" receipt in the requester's room (TS:lib/engagement-notice.js:2-20).
9. `!help` and `!offer` are gone, and a `!…` line in a DM now becomes an agent prompt (TS:lib/bot-commands.js:469, 630).
10. Top-level group answers carry no `m.in_reply_to` link to the question (TS:bridge-matrix.js:3386-3392).
11. The binding state event is `com.hagency.project.binding.v1` instead of `admin.binding.v1`, with a different state key and no ADR; this breaks interop (RS:hagency-matrix/src/collector.rs:735).
12. No connection probe: binding the reception room is an offline operator step (TS:lib/fleet-protocol.js:98-153).
13. Appservice registration issue, verify and staged replacement are missing (TS:lib/appservice-receiver.js:70-143; TS:backend-v2.js:9931).
14. An agent that loses membership is not re-invited and rejoined on send (TS:bridge-matrix.js:10888-10950).
15. The approval DM is not locked down with power levels (:8861-8887).
16. No warning when the owner is absent from the approval room (:9267-9297).
17. The completed-task explanation notice to non-requesters is missing (TS:router/src/store.ts:1580-1581).
18. Retirement does not make the agent leave rooms or log out (TS:lib/matrix-work-executor.js:13-49).
19. The Palpo edge Matrix lane is stored but never routed to intake (ADR-037:75-79).
20. Agent display names and avatars are not set; the "⚠️ Attachment not delivered" notice is also missing (:5938-5955, 1924-2030, 11045-11052).

## Also worth knowing

- **Rust-only behavior:** the over-budget "Still running after N" notice (RS:hagency-store/src/domain/attempt_events.rs:162-175) and the Matrix-carried engagement verdict (`com.hagency.engagement.approval.v1`).
- **Undocumented wire differences** (no ADR found): the binding state event name, the event type and payload key of the public status notice, and the msgtype of the router notices.
- **Proposed-only ADRs:** the approval trust and recovery changes rest on ADR-137 and ADR-183, and the project-side gaps are recorded in ADR-132. All three are still Proposed, so those rows count as intentional only once they are accepted.