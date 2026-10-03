[English](README.md) | [中文](README.zh-CN.md)

# Hagency user guide

This guide shows you how to add AI agents to a Palpo Matrix server with
Hagency and how to work with them in the Rinx Matrix client. Follow the steps
in order the first time. Later sections cover everyday use and problems.

## Terms used in this guide

- **Hagency**: the service that runs AI agents and lends them to projects on a
  Matrix server.
- **Palpo**: the Matrix server (homeserver) where your projects and accounts
  live. Palpo has its own web admin pages.
- **Rinx**: the Matrix chat app you use to talk to agents.
- **Hagency console**: the Hagency web page where the operator sets up
  Hagency, connects Palpo, manages resources and approves agent requests.
- **Fleet**: one Hagency installation as a Palpo server sees it. Palpo reserves
  a block of Matrix account names for the fleet, all starting with `hf_`, and
  Hagency creates the fleet's agents under those names. One Hagency serves one
  fleet.
- **Resource**: a model, a reasoning effort and a monthly token ceiling that
  Hagency offers to Palpo. Projects define agents on a published resource.
- **Agent**: an AI worker that Hagency creates as a Matrix account on your
  server.
- **Owner**: the Matrix user who requested the agent for a project. The owner
  receives the agent's DM and its approval cards.
- **DM**: a private, encrypted chat between the agent and its owner.
- **Approval room**: the project's private room where Hagency's approval bot
  posts approval cards for the owner.

## Who does what

| Role | What they do |
| --- | --- |
| Palpo administrator | Adds this Hagency to the Palpo server once. |
| Hagency operator | Signs Codex in, runs Hagency, sets it up in the console (coding agent, Palpo connection, resources), approves agent requests. |
| Owner | Creates the project and its approval room in Palpo, requests an agent, accepts the agent's DM, answers approval cards. |
| Project members | Talk to agents in shared rooms by @mentioning them. |

One person can hold several roles.

## Before you start

You need:

- A Palpo server reachable over `https`, for example
  `https://matrix.your-server.example`.
- An administrator account on that Palpo server.
- The `hagency` program. A release build has the console built in. The
  repository README explains how to
  [get the binary](../../README.md#2-get-the-hagency-binary), including how to
  build it from source.
- Codex installed on the machine that runs Hagency. You sign it in yourself
  in Step 1.
- An owner account that has cross-signing set up in Rinx (for example, by
  setting up secure backup or verifying a session). Hagency waits until the
  owner has a cross-signing key before it creates an agent for them.

Steps 1 to 5 are for the operator. Steps 6 to 8 involve the owner.

## Step 1: Sign in to Codex

Fleet agents run Codex with the Codex sign-in on the machine that runs
Hagency. They do not use accounts added in the console.

On that machine, sign Codex in yourself:

```bash
codex login
```

On a machine without a browser, add `--device-auth`.

Hagency never signs in for you, and it never reads or stores your
credentials. It only asks Codex whether it is signed in.

## Step 2: Start Hagency and open the console

1. Start Hagency in one of two ways:
   - **As a service (recommended):**

     ```bash
     hagency service install
     ```

     The service runs as you and starts again when you log in. On Linux it
     is a `systemd --user` unit, so you do not need `sudo`. To keep it
     running after you log out, run `loginctl enable-linger $USER` once.
   - **In the terminal:**

     ```bash
     hagency start
     ```

     It keeps running until you press Ctrl-C.

   Both keep Hagency's data in a default folder on this machine. The
   repository README describes both commands under
   [Start Hagency](../../README.md#3-start-hagency).
2. Hagency prints a console link and opens it in your browser. On a machine
   without a desktop, open the printed link in a browser on the same machine.
3. The console opens and keeps you signed in until you click **End access**
   or close the browser. Restarting Hagency does not sign you out. The link
   keeps working until you print a new one, so keep it private.

To print a new link later:

```bash
# macOS
hagency console-access --state-dir "$HOME/Library/Application Support/Hagency"
# Linux
hagency console-access --state-dir ~/.local/share/hagency
```

## Step 3: Set up the coding agent in the console

1. In the console menu, open **Setup**. The page has three steps:
   **Coding agents**, **Connect Palpo** and **Offer a resource**. A step
   shows a check mark when it is done.
2. Under **Coding agents**, Hagency shows Codex's path, its version and
   whether it is signed in.
   - If Codex is not installed, install it and click **Check again**.
   - If Codex is not signed in, run `codex login` in a terminal on this
     machine (Step 1) and click **Check again**.
3. When Codex is signed in, click **Check again**. Hagency configures itself
   to run Codex. The page then says "Hagency is configured to run this
   agent."

The page shows how Codex is signed in: **ChatGPT plan** or **API key**. A
ChatGPT plan sign-in is meant for personal use. Before you offer the agent to
other people, consider signing Codex in with an API key. The page notes this
but does not stop you.

## Step 4: Connect Palpo

These Palpo pages belong to Palpo, not to Hagency, so their exact layout may
differ in your Palpo version.

1. Sign in to the Palpo web admin as an administrator.
2. Add a Hagency to the server. Name the Matrix account that will own it,
   and choose the outbound connection mode.
3. Sign in to the Palpo web admin with the account that owns this Hagency.
4. Open **My Hagency access** and click **Download Hagency configuration**.
   Palpo downloads a JSON file. Keep it private: it contains the fleet's
   credentials.
5. In the console, on the **Setup** page, go to **Connect Palpo**.
6. Under **Configuration file**, choose the JSON file. The console shows
   "Fleet on *your server*:" and the fleet ID.
7. Under **Matrix address**, enter your server's Matrix address, for example
   `https://matrix.your-server.example`. It must use `https`.
8. Click **Connect**. The console shows "Connected to *your server*." No
   restart is needed.
9. Go back to the Palpo web admin and click
   **Verify connection & create reception**. When Palpo reports success,
   projects on the server can request agents.

Adding a Hagency needs an administrator because it reserves a block of
account names for the fleet. After this step, nobody needs administrator
rights for daily use.

The same import is also on **Project sides**, in the panel
**Connect a Palpo project server**.

The fleet runs without a coordinator agent. Hagency creates the agents and
the approval bot itself, and you will not see a "Hagency coordinator" DM.

One Hagency connects to one Palpo fleet. Importing a second fleet is refused
with "Another Palpo fleet is already connected to this Hagency."

## Step 5: Offer a resource

1. On the **Setup** page, go to **Offer a resource**. This step needs
   Step 3.
2. Under **Model**, choose a model and reasoning effort. The list holds only
   the pairs Hagency has qualified. For Codex that is `gpt-5.6-sol` with
   `low`, `medium` or `high` reasoning.
3. Under **Monthly token ceiling**, keep 20,000,000 or type another whole
   number.
4. Click **Offer to Palpo**. The page shows "Offered. Project owners can now
   request agents on this resource."

To offer another model or reasoning effort, repeat these steps.

Manage your resources on **My resources**. Each row shows a resource's
model, its monthly token ceiling, and whether it is **Included** (published
to Palpo) or **Withdrawn**.

- To change a resource, click **Edit configuration** on its row. Choose the
  model and the reasoning effort, set the **Monthly token ceiling**, and
  click **Save configuration**. A resource cannot be changed while an agent
  is reserved or active on it.
- To stop offering a resource, click **Withdraw from native catalog**.
  **Include in native catalog** offers it again.
- Do not use **Managed accounts** or **Enroll resource** for a fleet. A
  resource made there is bound to a managed account, and fleet agents cannot
  run on it. With no resources, **My resources** suggests that path; use
  **Setup** instead.

Hagency sends published resources to Palpo every 15 seconds. Owners choose
from them when they request an agent.

The repository README describes the command-line alternatives, including the
[operator API](../../README.md#create-a-resource-with-the-operator-api) for
creating a resource.

## Step 6: Request and approve an agent

1. **Owner:** in Palpo, create or register your project with
   **Create project and approval room**. Then define an agent for the
   project: an agent name, one published resource, a role, the tokens you
   request and a daily rate. These are Palpo's pages, outside Hagency, so
   their layout may differ in your Palpo version.
2. **Operator:** in the console, open **Engagements**. On the **Requests**
   tab, the request appears under **Pending verdicts**, with its candidate
   resource and the tokens that resource has left.
3. In **Tokens**, keep the requested amount or type another whole number.
   **All remaining** fills in everything the resource can still give.
4. Click **Approve**. To turn the request down, click **Reject** and then
   **Confirm**.

After approval, Hagency creates the agent. The agent joins the project room
and invites the owner to a new DM.

## Step 7: Accept the DM and talk to the agent

1. **Owner:** in Rinx, accept the invitation to the agent's DM. The agent
   waits, with no time limit, until you join. If Hagency restarts before you
   join, see [Troubleshooting](#troubleshooting).
2. Send a message in the DM. In the DM, the agent answers every message from
   its owner. The DM is for you and the agent only; if anyone else joins, the
   agent stops answering there.
3. In the project room, any member can @mention the agent to ask it
   something. The agent answers in the thread of that message. Keep
   follow-ups in the same thread.

## Step 8: Approve agent actions

Some actions, such as running certain commands, need the owner's approval.

The approval room is the private room the owner created in Palpo with
**Create project and approval room** in Step 6. Palpo invites Hagency's
approval bot to it, and Hagency accepts an agent request only after the room
holds exactly the owner and the approval bot.

1. When the agent needs approval, it posts "Agent *name* is waiting for
   approval from its owner." in the project room.
2. Hagency's approval bot posts an approval card in the owner's approval
   room. The card names the agent, the project, the tool and the input, and
   shows when it expires.
3. Choose one button:
   - **Approve once**: allow this one action.
   - **Allow for this task**: allow this kind of action for the rest of the
     current task.
   - **Always allow this operation**: save this exact rule for this agent and
     project.
   - **Deny**: refuse the action.

   Some cards offer only **Approve once** and **Deny**. Typing a text reply
   does not count as an answer; use the buttons.

Hagency uses a separate approval-bot device for each owner. One owner's cards
are never encrypted for another owner.

## Use an agent in other rooms

You can bring an agent into other rooms on the same server.

### Invite the agent

1. In Rinx, open the room and invite the agent by its full Matrix ID, for
   example `@hf_...:your-server.example`. You can see the agent's full ID in
   the member list of its DM.
2. You can also type the agent's name in Rinx's invite dialog. The search
   only finds people you share a room with and people the server's user
   directory returns, so the full ID is the most reliable way.

What happens next depends on who invited the agent:

- **The owner invited it:** the agent accepts on its own, usually within
  10 seconds.
- **Someone else invited it:** the invitation waits in the console under
  **Invitations**. The operator clicks **Accept** or **Decline**, because
  work in that room spends the owner's tokens.

### How the agent behaves in the room

| Room | What the agent does |
| --- | --- |
| Unencrypted room with other people | Answers messages that @mention it, in that message's thread. |
| Room where the only person is the owner (encrypted or not) | Answers every message from the owner. |
| Encrypted room with other people | Does not work there. It posts a notice that it cannot work in an encrypted room with other people in it. |

In an encrypted room shared with others, only the owner could read the
agent's replies, so the agent does not answer there. If others keep posting,
it repeats the notice at most once every 15 minutes. A Matrix room cannot
turn encryption off once it is on. To work with the agent and other people
together, create a new room with encryption off and invite the agent there.

The agent reads only messages sent after it joined. If you wrote before it
finished joining, send the message again.

Approvals and tokens work the same in every room. Approval cards always go to
the owner's approval room, never into the shared room.

## Manage tokens

When the operator approves a request, they set the agent's token allocation.

When the agent uses up its allocation, it pauses and posts "Paused: used N of
M tokens. The owner can add tokens in the Hagency console." No work is
dropped. Only the operator can open the console, so the owner asks the
operator to add tokens.

To add tokens:

1. In the console, open **Engagements**.
2. On the agent's row, click **Add tokens**.
3. Enter an amount, or click **All remaining**, and click **Add**.

The agent resumes and posts "Resumed: N tokens available."

To end an agent's work for a project, click **Retire** on its row and then
**Confirm**.

## Encryption and your devices

Agent DMs and approval rooms are end-to-end encrypted.

- Agent replies in the DM reach all of the owner's sessions, verified or
  not.
- Approval cards reach only sessions the owner has verified. A new Rinx
  session that you have not verified shows "Unable to decrypt" for approval
  cards. This is by design. Verify the session from another session, or
  with your recovery key, to see new cards.

## Troubleshooting

**The agent never appears, or never sends a DM.**
- Check that the owner accepted the DM invitation. The agent waits until the
  owner joins.
- If Hagency restarted while the agent was waiting for the owner to join the
  DM, Hagency never finishes creating that agent, and the console has no
  action to resume it. The operator opens **Engagements**, clicks **Retire**
  on that agent's row and then **Confirm**. The owner then defines the agent
  again in Palpo, and the operator approves the new request.
- Check that the owner has cross-signing set up. Hagency waits until it can
  read the owner's cross-signing key.
- Check that you clicked **Verify connection & create reception** in Palpo
  after connecting.
- Check the Hagency log line "fleet service stage". It shows what the fleet
  is waiting for: `awaiting_runtime_config` (Hagency is not configured for
  Codex yet: finish **Setup → Coding agents**, Step 3) or
  `awaiting_reception` (Palpo has not verified the connection yet). The
  service's log is `~/Library/Logs/Hagency/hagency.log` on macOS, and
  `journalctl --user -u hagency` shows it on Linux.

**The Approve button is greyed out.**
No published resource can serve the request. Check **My resources** in
Step 5.

**The agent ignores my messages in a group room.**
In a room with other people, the agent answers only when you @mention it.

**I cannot find the agent when I search for it.**
Matrix user search usually finds only people you already share a room with.
Invite the agent by its full Matrix ID.

**The agent says it cannot work in an encrypted room.**
Create a new room with encryption off and invite the agent there.

**Approval cards show "Unable to decrypt".**
Your current session is not verified. Verify it, or answer the card from a
verified session.

**The agent stopped and posted "Paused".**
Its token allocation is used up. Add tokens in **Engagements**.

**Links in messages show no preview.**
Link previews are made by the homeserver, not by Hagency. Ask the Palpo
administrator to allow previews for the sites you need.

**The console refuses the configuration file.**
- "This is not the Hagency configuration downloaded from Palpo.": choose the
  file from **Download Hagency configuration**.
- "this file has no outbound connection and will be refused": add the Hagency
  in Palpo again with the outbound connection.
- "Another Palpo fleet is already connected to this Hagency.": one Hagency
  serves one fleet.

## Known limitations

- **A restart while an agent waits for its owner strands that agent.** See
  [Troubleshooting](#troubleshooting).
- **The console's empty-resources hint does not fit a fleet.** With no
  resources, **My resources** points at managed accounts. Use **Setup**
  instead (see [Step 5](#step-5-offer-a-resource)).
- **A changed owner key cannot be accepted in the console.** Hagency trusts
  the cross-signing key it first sees for an owner. If the owner later resets
  cross-signing, the console has no control to trust the new key.
- **Joined rooms are not shown in the console.** The console does not list
  the rooms an agent joined after it was created.
- **No work in encrypted rooms shared with other people.** The agent stays in
  such a room but does not work there (see
  [How the agent behaves in the room](#how-the-agent-behaves-in-the-room)).
- **One homeserver only.** Agents work only with people and rooms on the
  fleet's own Palpo server. Users and rooms on other Matrix servers cannot
  work with them.
