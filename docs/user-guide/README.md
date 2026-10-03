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
- **Hagency console**: the Hagency web page where the operator connects Palpo,
  manages resources and approves agent requests.
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
| Hagency operator | Runs Hagency, signs Codex in, connects the fleet, manages resources, approves agent requests in the console. |
| Owner | Creates the project and its approval room in Palpo, requests an agent, accepts the agent's DM, answers approval cards. |
| Project members | Talk to agents in shared rooms by @mentioning them. |

One person can hold several roles.

## Before you start

You need:

- A Palpo server reachable over `https`, for example
  `https://matrix.your-server.example`.
- An administrator account on that Palpo server.
- The `hagency` program and the console files, built from this repository.
  The repository [README](../../README.md) explains how to build them.
- A Codex sign-in on the machine that runs Hagency (Step 4).
- Codex installed on that machine. In Step 2, the installer or
  `hagency setup` finds it and writes `fleet-runtime.json`, the local Codex
  runtime settings, for you.
- An owner account that has cross-signing set up in Rinx (for example, by
  setting up secure backup or verifying a session). Hagency waits until the
  owner has a cross-signing key before it creates an agent for them.

## Step 1: Add Hagency in Palpo

These pages belong to Palpo, not to Hagency, so their exact layout may differ
in your Palpo version.

1. Sign in to the Palpo web admin as an administrator.
2. Add a Hagency to the server. Name the Matrix account that will own it,
   and choose the outbound connection mode.
3. Sign in to the Palpo web admin with the account that owns this Hagency.
4. Open **My Hagency access** and click **Download Hagency configuration**.
   Palpo downloads a JSON file. Keep it private: it contains the fleet's
   credentials.

Adding a Hagency needs an administrator because it reserves a block of
account names for the fleet. After this step, nobody needs administrator
rights for daily use.

## Step 2: Start Hagency and open the console

Run these commands on the machine that runs Hagency. Replace the paths with
your own.

1. Start Hagency in one of two ways, a or b:

   a. **As a service (recommended).** Run the installer from this repository.
      Fleet mode is its default. On Linux run it with sudo (see the README):

      ```bash
      install/install-native.sh \
        --install-dir /path/to/bin \
        --state-dir /path/to/state \
        --console-dir /path/to/console-assets
      ```

      It creates the state directory, runs `hagency setup` (see b),
      installs the service and starts it. The repository README lists its
      options and explains which user the service runs as under
      [Build and install](../../README.md#build-and-install). Then go to
      item 2.

   b. **In the foreground.** Prepare the state directory, then start Hagency
      with the Palpo connection and the console:

      ```bash
      hagency setup --state-dir /path/to/state
      hagency serve \
        --state-dir /path/to/state \
        --listen 127.0.0.1:13300 \
        --palpo-transport \
        --console-assets /path/to/console-assets
      ```

      - `hagency setup` creates the state directory if it is new, finds Codex
        and writes `fleet-runtime.json`. It reports whether Codex is signed in
        (Step 4). It does not replace an existing `fleet-runtime.json` unless
        you pass `--force`, and then it keeps the old file as a backup.
      - `--listen` must be a loopback address with a port. `127.0.0.1:13300`
        is the default. If you change it, pass the same `--listen` to
        `hagency setup`.
      - Do not add `--agent-driver`. That flag starts the older setup with a
        coordinator agent, and then Hagency does not run the fleet itself.

2. In a second terminal, print a console link:

   ```bash
   hagency console-access --state-dir /path/to/state
   ```

   If you changed `--listen`, pass the same `--listen` here.

3. Open the link in a browser on the same machine. The console opens and
   keeps you signed in until you click **End access** or close the browser.
   Restarting Hagency does not sign you out. The link keeps working until you
   print a new one, so keep it private.

## Step 3: Connect the fleet in the console

1. In the console, open **Project sides**.
2. Find the panel **Connect a Palpo project server**.
3. Under **Configuration file**, choose the JSON file from Step 1. The
   console shows "Fleet on *your server*:" and the fleet ID.
4. Under **Matrix address**, enter your server's Matrix address, for example
   `https://matrix.your-server.example`. It must use `https`.
5. Click **Connect**. The console shows "Connected to *your server*." No
   restart is needed.
6. Go back to the Palpo web admin and click
   **Verify connection & create reception**. When Palpo reports success,
   projects on the server can request agents.

The fleet runs without a coordinator agent. Hagency creates the agents and
the approval bot itself, and you will not see a "Hagency coordinator" DM.

One Hagency connects to one Palpo fleet. Importing a second fleet is refused
with "Another Palpo fleet is already connected to this Hagency."

## Step 4: Sign Codex in and check your resources

Fleet agents run Codex with a Codex sign-in on the machine that runs Hagency.
They do not use accounts added in the console.

1. Sign Codex in, if it is not signed in yet. `hagency setup`, run by the
   installer or by you in Step 2, reports whether Codex is signed in. If it
   is not, setup prints the command to run on the Hagency machine, for
   example:

   ```bash
   CODEX_HOME=/path/to/codex-home codex login
   ```

   By default, fleet agents use this machine's own Codex sign-in folder
   (`$CODEX_HOME`, or `~/.codex`). If setup ran with `--no-local-codex`, they
   use the folder `runtime-home` in the state directory instead, and the
   printed command names that folder. On a machine without a browser, add
   `--device-auth` to `codex login`. Codex keeps the sign-in in that folder.
   Hagency does not store your credentials.
2. Check your resources, and create the first one if there is none:
   - **(a) Check My resources.** In the console, open **My resources**. Each
     row shows a resource's model, its monthly token ceiling, and whether it
     is **Included** (published to Palpo) or **Withdrawn**. With no
     resources, the console suggests creating the first resource from a
     managed account on the Accounts page. Ignore that hint, and do not use
     **Managed accounts** or **Enroll resource** for a fleet: a resource made
     there is bound to a managed account, and fleet agents cannot run on it.
   - **(b) If the list is empty, run the operator API once.** The console
     cannot create a fleet's first resource. The operator creates it on the
     machine that runs Hagency, with your own `--listen` address and state
     directory:

     ```bash
     curl -s -X POST http://127.0.0.1:13300/api/native/v1/resources \
       -H "Authorization: Bearer $(cat /path/to/state/operator.token)" \
       -H 'Content-Type: application/json' \
       -d '{"presetId":"local_codex","seatId":"local_codex_seat","framework":"codex","model":"gpt-5.6-sol","provider":"openai","reasoning":"medium","ceiling":{"tokens":20000000,"period":"monthly"},"published":true}'
     ```

     The repository README shows this step in
     [First run](../../README.md#first-run).
   - **(c) The rules.** Palpo sees a resource only if its model and reasoning
     effort are a pair Hagency has qualified for at least one role. For Codex
     that is `gpt-5.6-sol` with `low`, `medium` or `high` reasoning. Hagency
     stores any other pair but does not publish it. With a `local_codex`
     block, `seatId` must also equal `local_codex.seat` (`hagency setup`
     writes `local_codex_seat`, as in the command above), with `framework`
     `codex` and `provider` `openai` or left out. Hagency accepts and
     publishes a resource that does not match, but refuses to run agents on
     it.

   After that, use the console for everything else (items 3 to 5 below).
3. To change a resource, click **Edit configuration** on its row. Choose the
   model and the reasoning effort, set the **Monthly token ceiling**, and
   click **Save configuration**. The page offers only models and reasoning
   efforts that Hagency supports. A resource cannot be changed while an agent
   is reserved or active on it.
4. To offer another model or reasoning effort on the same Codex sign-in,
   click **New resource configuration**, choose an existing resource as the
   **Source configuration**, set the model, reasoning effort and ceiling, and
   click **Create another configuration**. New resources are published at
   once.
5. To stop offering a resource, click **Withdraw from native catalog**.
   **Include in native catalog** offers it again.

Hagency sends published resources to Palpo every 15 seconds. Owners choose
from them when they request an agent.

## Step 5: Request and approve an agent

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

## Step 6: Accept the DM and talk to the agent

1. **Owner:** in Rinx, accept the invitation to the agent's DM. The agent
   waits, with no time limit, until you join. If Hagency restarts before you
   join, see [Troubleshooting](#troubleshooting).
2. Send a message in the DM. In the DM, the agent answers every message from
   its owner. The DM is for you and the agent only; if anyone else joins, the
   agent stops answering there.
3. In the project room, any member can @mention the agent to ask it
   something. The agent answers in the thread of that message. Keep
   follow-ups in the same thread.

## Step 7: Approve agent actions

Some actions, such as running certain commands, need the owner's approval.

The approval room is the private room the owner created in Palpo with
**Create project and approval room** in Step 5. Palpo invites Hagency's
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
  is waiting for: `awaiting_runtime_config` (`fleet-runtime.json` is missing:
  run `hagency setup`) or `awaiting_reception` (Palpo has not verified the
  connection yet).

**The Approve button is greyed out.**
No published resource can serve the request. Check **My resources** in
Step 4.

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
- **The console's empty-resources hint does not fit a fleet.** See
  [Step 4](#step-4-sign-codex-in-and-check-your-resources), item 2.
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
