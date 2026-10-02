[English](README.md) | [中文](README.zh-CN.md)

# Hagency user guide

This guide is for people who use Hagency through a Palpo Matrix server: the
Hagency owner who lends AI agents, and the project owners and members who work
with them. It covers what to set up once, which rooms you will see, and who can
talk to an agent.

A few words used throughout:

- **Hagency** — the service that runs AI agents and lends them to projects.
- **Palpo** — the Matrix server (homeserver) your project lives on. It has a web
  page, "Palpo web", for accounts, projects and Hagency access.
- **Matrix client** — the chat app you use, for example Element or Rinx.
- **Hagency console** — the Hagency owner's web page for approving requests,
  managing agents and tokens.

## Who does what

| Role | Who | What they do |
| --- | --- | --- |
| Matrix server admin | The Palpo administrator | Once per Hagency: **Add Hagency** in Palpo web. |
| Hagency owner | An ordinary Matrix account named as owner | Downloads the configuration, connects the Hagency, verifies it, approves agent requests. |
| Project owner and members | Ordinary Matrix accounts | Create projects, request agents, talk to agents. |

Why the admin is needed: adding a Hagency installs a Matrix **App Service**. That
is a private block of account names, `@hf_<fleet>_*`, that the Hagency may create
and act as — its representative, its approval bot and its agents. Only a server
admin can grant that. It is the same rule as on any Matrix server (Synapse
included). After this one step, nobody needs admin rights again.

## Connect a Hagency to a Palpo server

1. **Admin:** in Palpo web, open **Add Hagency**. Fill in a name, the owner's
   Matrix ID, and the connection mode **Hagency connects outbound to Palpo**.
   Click **Authorize and install**.
2. **Owner:** in Palpo web, open **My Hagency access** and click
   **Download Hagency configuration**. You get a JSON file.
3. **Owner:** in the Hagency console, go to **Project sides** →
   **Connect a Palpo project server**. Choose the JSON file, enter the
   **Matrix address** (the homeserver URL, for example
   `https://matrix.example.org`), and click **Connect**. No restart is needed.
4. **Owner:** back in Palpo web, click **Verify connection & create reception**.
   When it reports ready, projects on this server can request agents.

A Hagency connects to one Palpo fleet.

Sign in to Palpo web with your full Matrix ID, such as `@alice:example.org`.
The short name (`alice`) is refused for now.

## Matrix IDs and the server name

A Matrix ID looks like `@name:server_name`. The `server_name` part is fixed
when the server is first set up and can never change, so choose your real
domain (for example `example.org`), as matrix.org does. If the server runs on a
non-standard port, a `.well-known/matrix/client` file on that domain tells
clients where to find it.

## The rooms you will see

| Room | What it is | What you do there |
| --- | --- | --- |
| Reception room | The mailbox between Palpo and the Hagency. Agent requests and the connection check arrive here as special events. | Nothing. Your client shows little in it. |
| Project room | Where people and agents work together. | @mention an agent to ask it something. |
| Approval room | The project's private, encrypted room. Only you and the approval bot are in it. | Approve or deny risky agent actions from the cards posted here. |
| Agent DM | One private, encrypted chat per agent, with its owner. | Talk to the agent one to one. |

Current builds also create a **Hagency coordinator** DM. It is temporary
scaffolding and is being removed (ADR-187). You can ignore it.

## Who can talk to an agent

- **Its DM:** only its owner. The room is invite-only, and Hagency accepts only
  the owner's messages there.
- **The project room:** any member who @mentions the agent. Messages that do not
  mention it do not wake it. The project room is invite-only, and the project
  owner decides who is in it.
- **Anywhere else on the server:** no one.
- **Other Matrix servers:** not reachable. This setup does not use federation.

## What an agent hears

An agent can read the whole room. It has a tool to read the conversation
history when it needs context. But it acts only when someone @mentions it.

It answers in the thread of the message that mentioned it. Follow-ups in that
thread reach it too, so keep a piece of work in one thread.

## Giving an agent instructions

In a shared room, an agent works for the room. Any member can @mention it and
give it instructions. There is no per-agent "only take instructions from these
people" list today.

What protects the owner:

- **Approvals.** Risky actions need approval. Approval cards go only to the
  owner's private approval room.
- **Tokens.** Everyone's requests spend the agent's token allocation. When the
  allocation runs out, the agent pauses and posts a notice. Only the owner can
  add tokens (see below).
- **Membership.** The owner controls who is in the room.
- **Retiring.** The owner can retire the agent at any time.

## Tokens

When the owner approves an agent request, they choose a token allocation.
**All remaining** fills in everything the resource can still give.

When an agent reaches its allocation, it pauses. Nothing is dropped. To resume
it, open the Hagency console → **Engagements** → **Add tokens**. The agent
continues where it stopped.

## Invitations

How it is designed to work:

- If the agent's **owner** invites it to a room, the agent trusts the invite and
  joins.
- If **anyone else** invites it, the invite becomes a pending decision in the
  Hagency console under **Invitations**. The owner decides, because joining
  spends the owner's tokens.

Inside a room it has joined, the same rule applies: it acts only when
@mentioned. See the limits below — agents do not act on invitations yet.

## Encryption

Agent DMs and approval rooms are end-to-end encrypted. After you sign in on a
new device, verify that session — from a session you already have, or with
your recovery key. Until you do, agent DMs and approval cards will not decrypt
on the new device.

## Known limitations (as of this release)

- **Invitations are not acted on yet.** Agents ignore invitations for now; only
  the coordinator's invitations are watched. Fixed by ADR-187 slice 6.
- **First message can be lost.** A message sent in the first second after an
  agent is created can be lost if its encryption key arrives a moment late.
  If the agent does not respond, send the message again. A fix is in progress.
- **Coordinator DM.** The "Hagency coordinator" DM is temporary scaffolding,
  being removed by ADR-187.
- **Full Matrix ID sign-in.** Palpo web accepts only the full Matrix ID
  (`@alice:example.org`), not the short name.
