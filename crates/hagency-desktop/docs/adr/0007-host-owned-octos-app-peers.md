# ADR 0007: Host-owned Octos app peers and Rinx deployment modes

- Date: 2026-09-26
- Status: Accepted; implemented (see the implementation record). Hosted ROM Home
  on the OnePlus 6 is unverified; the signed-in native desktop exchange is
  covered only by headless real-kernel tests.
- Extends [ADR 0005](0005-octoscript-miniapps-matrix-octos.md) and
  [ADR 0006](0006-shared-app-hub-miniapps.md). Proposes replacing AppCard-dependent
  connection discovery and mini-app-owned agent allocation, while retaining the
  shared package format, Matrix contracts and instance grants.

## Context

Rinx runs as a standalone application or a native module in OctoSense desktop
and ROM Home. It also hosts Octoscript mini apps. These are different ownership
boundaries: an OctoSense app can have an app agent, while a mini app inside Rinx
uses services granted by Rinx. A mini app does not need its own Octos kernel,
provider configuration or autonomous peer.

OctoSense should know that an app needs Octos from its native API capability
declaration. When launching an authorized app, the host should create or resume
an app peer owned by the system agent. The system agent and app peer must be able
to exchange input, questions, answers and results throughout the app's lifetime.
Opening an app must not require an LLM to decide whether to establish this link.

The intended deployment uses one Octos kernel and one shared provider profile
per OctoSense user runtime. Apps do not choose independent providers or supply
provider credentials. Selecting a model configured by the system does not require
a separate profile. Separate conversation state and app data remain necessary.

## Decision

### 1. Capabilities determine access; the host owns the peer

Use the same capability vocabulary for native module descriptors and App Hub
manifests. The existing Rinx integration recognizes these service names:

```json
{
  "capabilities": [
    "octos.session.open",
    "octos.session.history",
    "octos.turn.start",
    "octos.turn.interrupt"
  ]
}
```

This is an example subset of a manifest, not a complete package. Each app requests
only what it uses. Admission checks exact registered names; a string beginning
with `octos.` does not automatically grant access. Native linkage to Octos also
does not grant authority. The host intersects declarations with supported APIs,
host policy and user grants, and checks the resulting lease on every request.

At launch, an app with granted Octos access receives a scoped host service handle.
For an app authorized to run an agent, the host creates or resumes its peer on
behalf of the system agent, recording that system session as the originator.
History-only access must not implicitly grant turn execution. Creating a peer
does not itself require a model inference or a background task.

Apps without Octos access do not allocate peers. If an app's AI feature is
optional, its ordinary UI remains usable without a provider. Unsupported required
capabilities make an app unavailable with an explicit reason. This ADR does not
invent an optional-capabilities field in the current manifest schema; any such
schema extension must be agreed with App Hub.

### 2. One system kernel, one app peer, shared provider configuration

```mermaid
flowchart TD
    Host[OctoSense host and capability broker] --> Kernel[Shared Octos kernel]
    Kernel --> Root[System agent]
    Root <-->|input, questions, answers, results| Rinx[Rinx app peer]
    Root <-->|input, questions, answers, results| Other[Other app peer]
    Mini[Rinx mini apps] --> Grants[Rinx Matrix and Octos service boundary]
    Grants --> Rinx
    Grants --> Matrix[Rinx Matrix SDK]
```

The host persists an association between the OctoSense user, app identity,
relevant account scope, peer session, system originator and app workspace.
Reopening an app resumes the intended peer instead of creating an unrelated peer
on every UI mount. A change of Matrix account must not expose a previous account's
history, memory or pending replies. Peer identity and workspace paths come from
the host, never from an app request.

The system agent can send follow-up input, receive progress and results, answer
ordinary peer questions, and interrupt its app peers. App peers can request help
and return information to the system agent. This is an interactive relationship,
not just a single task followed by a final result. Human approvals retain their
existing policy: the system agent's ability to answer a question is not blanket
authority to approve a tool or grant another app's data.

Peers remain directly owned by the system agent. Rinx does not need to spawn
further peers for its mini apps. The current one-level peer delegation rule is
compatible with this topology. Host routing must support launching Rinx without
first opening AppCard or a system-agent chat window.

### 3. Model selection stays under host control

The shared profile owns the provider configuration and credentials. The system
agent or host policy can select an allowed configured model for an app peer.
Apps cannot create provider configurations, change the system default or submit
credentials through the mini-app API.

Octos already supports `peer_handoff` with an optional `model` key naming a
configured `sub_provider`, such as `strong`. The peer stays in the master's
profile. The platform can restrict those configured choices to its selected
provider; the underlying kernel's ability to configure other providers does not
require exposing that choice to apps.

There is a protocol gap to review: the inspected raw OUP `peer/prepare` request
does not accept this model field. OUP `profile/sub_providers/*` manages configured
choices, and `profile/llm/select` changes the profile default; neither is a
verified API for assigning a different model to one existing peer. The host path
needs parity with peer creation through `peer_handoff`, and an explicit contract
for changing an existing peer's model if required. Apply changes between turns
and expose the effective model or fallback. Do not implement this by editing
peer files from Rinx or changing the global default for a single app.

### 4. App data and conversation isolation do not require app providers

Each app has its own workspace, conversation state, scoped tools and limits.
Per-app storage lives under a host-owned app namespace. For Rinx, Matrix account
identity also scopes private data. A future move to separate app processes must
preserve these logical identities and the same service contract.

In current Octos code, a `ProfileRuntime` owns memory, recall and policy as well
as the LLM provider. Therefore sharing a provider profile and giving peers
different working directories does not by itself isolate long-term memory.
The proposed shared-profile design requires app/account namespaces on memory
capture, recall, retrieval and automatic prompt injection. Explicitly shared
system knowledge can be separate from private app memory. The system agent can
obtain app information through authorized peer interactions; app histories must
not automatically become input to every other app.

App isolation is a host/kernel enforcement requirement, not a prompt instruction.
Raw OUP is not handed to untrusted apps: current peer originator checks operate
inside one user's trust domain and do not replace app capability leases. The
broker must bind requests and replies to their app, account and lease generation.
In-process native modules remain trusted code; Splash isolation is not OS process
isolation for arbitrary native Rust.

### 5. Rinx mini apps use Rinx's services

Mini apps keep the package, instance, Matrix account, optional room, storage and
capability boundaries from ADRs 0005 and 0006. They receive neither a kernel nor
an autonomous persistent peer. Rinx mediates their Octos requests through its app
service and the host-managed Rinx agent arrangement.

Mini apps may have separate request or conversation state to prevent one app's
prompts and replies from appearing in another. Such state is not an independent
agent or provider profile. The Octos integration review must choose how to enforce
those request contexts; simply forwarding every mini app into one unrestricted
Rinx transcript is insufficient. Existing ordinary mini-app sessions can remain
as a transitional adapter until equivalent isolation is demonstrated.

A mini app's request must not inherit all of Rinx's Matrix privileges. Reading
room data for AI requires the applicable Matrix read grant and Octos grant.
Posting, editing, deleting and other Matrix actions continue through Rinx's
authorized SDK adapters. A model output cannot expand a mini app's permissions.

## Rinx deployment models

| Concern | Standalone Rinx, desktop or mobile | Rinx inside OctoSense desktop or ROM Home |
| --- | --- | --- |
| UI entry | Rinx owns its window/activity and navigation | Native Rinx module uses the shell's viewport, windows and navigation |
| Current build boundary | Default `standalone` feature | `--no-default-features --features octosense-module`; `rinx::module::RINX_MODULE` |
| Octos ownership | Rinx owns an optional local runtime, or explicitly connects to a configured external provider | OctoSense owns the shared runtime; Rinx receives a scoped service handle |
| Agent role | Rinx's agent is the application root; no OctoSense system agent is assumed | Rinx's agent is a peer owned by the OctoSense system agent |
| Provider settings | Rinx-owned settings for a local runtime, or the external service's settings | Shared OctoSense settings; no duplicate provider setup in Rinx |
| Kernel version | Pinned with the standalone distribution when bundled locally | Pinned by OctoSense; Rinx must be compatible with the host API |
| Private state | Rinx-owned data root with account/app namespaces | Host-provided Rinx namespace, further scoped by account/app |
| Mini apps | Same App Hub packages and Matrix/Octos service contracts | Same App Hub packages and Matrix/Octos service contracts |
| Shutdown | Stop a Rinx-owned local runtime; disconnect without stopping an external runtime | Release Rinx leases; leave the shared kernel and other apps running |

### Deployment selection and packaging

There are two application deployments and three Octos connection modes:

| Application deployment | Octos mode | Selected by | Runtime owner |
| --- | --- | --- | --- |
| Standalone Rinx | Local | Rinx's AI settings | Rinx |
| Standalone Rinx | Remote | Explicit server configuration in Rinx | External Octos service |
| OctoSense native module | Hosted | Module creation by the OctoSense shell | OctoSense |

Hosted mode comes from the native host context, not an editable mini-app field,
environment guess, installed AppCard detection or a failed connection attempt.
Hosted Rinx cannot switch to local or remote ownership from its own settings.
Standalone mode must not silently attach to an unrelated process's provider.
An unconfigured or disabled provider is a normal state in either deployment;
Matrix and the offline editor remain usable.

Keep the existing `standalone` and `octosense-module` entry features. A standalone
desktop release builds Rinx's binary; a shell consumes the Rinx library with its
default features disabled. These existing commands describe the build boundary:

```sh
# Standalone desktop executable, using the default standalone feature.
cargo build --release --locked --bin rinx

# Library integration check; this does not build an OctoSense shell or APK.
cargo check --locked --lib --no-default-features --features octosense-module
```

Standalone Android packaging produces Rinx's own application. Hosted Android
packaging belongs to OctoSense ROM Home and links the module into that host.
Installing a Home test APK is not a ROM flash or a standalone Rinx release.
Desktop and ROM host releases choose their native runtime locks; the standalone
release chooses its own compatible pins. All modules linked into one host must
resolve a single compatible Makepad/Octoscript runtime and service interface.

The local kernel packaging dependency must be optional and separate from the
client/provider interface. A standard standalone package offering local AI ships
one pinned compatible kernel. A hosted Rinx library must not pull that owned
runtime into the shell's build. Cargo feature unification must be checked at the
final host workspace, not inferred from Rinx's feature names alone. New feature
names and platform artifact paths will be chosen in implementation; none are
claimed to exist by the commands above.

### Standalone with a local kernel

Rinx owns a single local runtime for the application, its provider settings and
its private kernel data root. The local runtime can be embedded or an owned child
process according to platform packaging; both use the same OUP service adapter.
Opening another mini app must not create another kernel. The Rinx agent is the
root application agent, so this mode does not manufacture an OctoSense system
agent merely to recreate the hosted peer relationship.

Start the runtime on the first authorized AI operation, or an explicit action in
AI settings. Establish the transport, negotiate supported services and prepare
the account-scoped workspace before enabling agent operations. Starting a
transport is not proof that a provider is authenticated or ready. Report startup
and provider errors in Rinx, with retry; ordinary Matrix use remains available.

Persist non-secret mode/settings separately from protected credentials. Allocate
the kernel root and app workspaces under Rinx-owned storage, independently of an
OctoSense installation or another desktop Octos profile. Do not import that other
profile's provider keys, memories or Matrix credentials implicitly. If a child
process is used, launch the packaged executable directly and track ownership;
do not find a random executable on `PATH` or terminate processes by name.

On shutdown or local-provider replacement, revoke dependent request leases,
cancel or finish owned work according to policy, disconnect and stop only the
runtime Rinx owns. The implementation must verify shutdown rather than assume
that dropping a UI reference terminates the kernel.

### Standalone with a remote kernel

Remote mode is an explicit alternative to local mode. Rinx stores the selected
endpoint, authenticated profile reference and protected transport credential.
The remote server owns model-provider credentials, model configuration and its
kernel lifetime. Rinx does not need an independently configured LLM provider in
addition to the server. Disconnecting or exiting Rinx must not stop the server.

Negotiate the APIs required by the installed mini app and bind each operation to
the authenticated Rinx account/app context. The server must provide or validate
the corresponding remote workspace namespace. A path under Rinx's local data
directory is not a remote workspace, and a remote connection does not grant the
server access to that directory. If scoped workspace provisioning is unavailable,
disable the affected Octos operations with a reason instead of broadening access
or claiming that a connection alone makes the mini app runnable.

Switching servers, profiles or accounts revokes old request authority and drops
late replies before the replacement provider is attached. Local app documents
remain local unless an authorized request explicitly transfers them. Reconnect
restores only the sessions belonging to the selected server and current account.

### Hosted in OctoSense desktop or ROM Home

The shell supplies a scoped Octos service provider when creating the native Rinx
module. The same service contract applies on desktop and mobile. Rinx registers
the required native capabilities so the shell can determine its Octos needs
before launch. The host creates/resumes the Rinx peer under the system agent and
supplies the peer binding together with its app namespace and effective grants.

The hosted module must not initialize a fallback kernel, maintain independent AI
credentials, or require AppCard to publish a connection first. Kernel startup,
model selection and provider configuration belong to OctoSense. Rinx can display
provider availability and an entry to the host's AI settings, but does not offer
an independent endpoint/key form in hosted mode. Missing host services produce
an unavailable AI state without changing the deployment mode.

Closing Rinx releases its leases and subscriptions. It does not retire the host
connection, stop the kernel, terminate another app's work or delete system AI
configuration. Logout also revokes Matrix-account-bound requests and prevents
their later restoration under another account. The shell owns any approved
background work and the persisted Rinx peer's lifecycle.

### Provider contract and division of implementation

Rinx should consume a provider interface, not the shell's process-global raw
connection. The following are contract requirements, not new OUP method names:

| Contract surface | Rinx responsibility | Provider/host responsibility |
| --- | --- | --- |
| Availability and services | Render actual state; reject unsupported calls | Report readiness, negotiated services and replacement/revocation |
| Open an app request context | Supply host-authenticated account/app identity and granted lease | Bind the correct peer/request context and validated workspace |
| Requests and events | Enforce Matrix/mini-app grants; route replies to the originating instance | Enforce effective tools, memory scope and resource limits; correlate events |
| Model/settings presentation | Show effective model/status and the appropriate settings entry | Select configured model and retain provider secrets |
| Cancellation and release | Revoke leases on close/account change; discard stale replies | Cancel scoped work and release context without stopping unrelated work |
| Runtime shutdown | Invoke only for a Rinx-owned local provider | Distinguish owned local lifetime from borrowed hosted/remote lifetime |

Rinx can implement deployment selection, local/remote ownership, provider
injection points, UI state, lease invalidation and tests within its repository.
OctoSense must supply the native injection and root-owned peer binding. Octos
must supply or confirm the remote workspace, peer/model, memory and request-scope
contracts identified above. Until those exist, an adapter must report the
missing capability; it must not substitute a privileged ordinary session and
claim the hosted design is complete.

Preserve the existing account/app storage layout where possible. If a migration
is needed, make it explicit and recoverable. Standalone and hosted installations
must not concurrently open the same Matrix database, transfer login secrets or
merge kernel stores automatically. Deployment mode is a packaging/ownership
choice, not an automatic account migration mechanism.

### Deployment completion criteria

| Mode | Required evidence before marking implemented |
| --- | --- |
| Standalone local | Packaged runtime starts on an authorized request; two mini apps share it with isolated request state; restart restores intended state; Rinx exit stops only its owned runtime |
| Standalone remote | Real authenticated connection and scoped workspace work; profile/account replacement rejects stale replies; exit leaves the server running |
| Hosted desktop | Launch Rinx before AppCard; use the system provider/peer; exchange a question and answer with the system agent; closing Rinx leaves a second app's peer functional |
| Hosted ROM Home | Exercise the same service flow on OnePlus 6, including Back/background/resume and process restart; verify that no Rinx-owned kernel is started |

These criteria complement the cross-repository acceptance list below. Build
checks alone establish neither runtime ownership nor live-service compatibility.
The deployment specification is complete for review; implementing these modes
and recording their evidence remains separate work.

The initial hosted design can run in the OctoSense process. Moving app UIs to
separate processes later changes the broker transport, not kernel ownership.
Per-app kernel instances or independently versioned kernels inside that process
are not part of this decision.

## Lifecycle and resource policy

The host manages peer creation, restoration, input scheduling and termination.
Persist the system originator or explicitly rebind ownership through a supported
kernel operation after restart. An ordinary session with a convenient name is
not a substitute for a correctly registered peer.

Closing a mini app revokes its lease, cancels its pending work where supported and
drops late replies. Closing Rinx releases foreground app resources; background
agent work requires an explicit host policy. Logout revokes account-bound work.
Reopening an authorized app can restore its saved state without keeping every
peer runtime resident while idle.

Concurrent apps need bounded queues, fair scheduling, cancellation and per-app
token/tool limits under the host's total budget. Neither the runtime cache size
nor the per-turn peer-handoff limit is the maximum number of installed apps.
Concrete limits must follow desktop/mobile measurements and provider limits;
this ADR does not claim unlimited parallel turns.

## Implementation evidence and gaps

These were the source findings when the ADR was proposed; the implementation
record below says what changed:

| Area | Inspected implementation | Required integration |
| --- | --- | --- |
| Native Rinx declaration | `src/module.rs` declares `net`, `storage`, `audio.output`, `clipboard`; no Octos capability | Add the agreed native capability declaration and host service injection |
| Native agent control | Rinx's `ServiceExecutor` currently returns unavailable | Define authorized app operations; peer messaging alone does not expose every UI/Matrix action |
| Mini-app capabilities | The pinned Hub policy and Rinx adapters know four `octos.*` services | Coordinate publication, host admission and supported service versions |
| Connection ownership | `KernelProvider::shared` discovers AppCard's published connection | Replace implicit discovery with the shell-owned provider/broker handle |
| Mini-app execution | Each current provider creates an ordinary `rinx-mini-*` session | Implement the app-peer arrangement while preserving request isolation |
| Peer primitives | Octos stages peers, records originators, routes input/questions and supports configured model selection | Bind launch/resume lifecycle to the system agent and host API |
| Memory | Current profile runtime shares memory and recall stores | Add app/account scoping before claiming isolated persistent memory |
| Standalone | Explicit remote connection UI and standalone entry exist | Complete local-runtime provisioning and verify remote workspace behavior |

Relevant Rinx sources: [module](../../src/module.rs),
[Octos adapter](../../src/host/octos.rs),
[package admission](../../src/miniapps/package.rs),
[mini-app UI](../../src/miniapps/ui.rs), and
[build features](../../Cargo.toml).

Octos source evidence is pinned to inspected revision
`6ad76e5c1e659bdf10ec05ae869428b48edccf7f`:
[peer tool model argument](https://github.com/octos-org/octos/blob/6ad76e5c1e659bdf10ec05ae869428b48edccf7f/crates/octos-agent/src/tools/peer_handoff.rs),
[peer ownership and delegation](https://github.com/octos-org/octos/blob/6ad76e5c1e659bdf10ec05ae869428b48edccf7f/crates/octos-cli/src/peers/mod.rs),
[OUP requests and model resolution](https://github.com/octos-org/octos/blob/6ad76e5c1e659bdf10ec05ae869428b48edccf7f/crates/octos-cli/src/api/ui_protocol_transport.rs),
and [profile-owned memory](https://github.com/octos-org/octos/blob/6ad76e5c1e659bdf10ec05ae869428b48edccf7f/crates/octos-cli/src/runtime/profile.rs).
The Rinx capability additions are in
[Hub policy revision b0591e2c](https://github.com/OctoSense-org/OctoSense-App-Hub/blob/b0591e2caa0dc5c62927f7b85dbf826a50618a03/crates/app-policy/src/manifest.rs).
Recheck each consumer's actual pin during implementation; these references are
evidence baselines, not an instruction to downgrade its dependencies.

## Cross-repository review and acceptance

OctoSense owns native capability discovery, broker injection, system-agent
identity, launch/resume and shared AI settings. Octos owns peer/OUP behavior,
memory namespaces and effective per-request tool authority. App Hub owns the
published capability/schema contract. Rinx owns its deployment adapters, Matrix
SDK authority, account scoping and nested mini-app lifecycle.

Before implementation, the OctoSense/Octos review must resolve the host peer
creation/resume API, model selection parity, memory namespaces and isolated
mini-app request contexts. It must also agree how permission prompts reach the
user and what background work survives an app close. No new raw RPC method or
manifest field in this ADR should be treated as an already available API.

Acceptance must cover:

1. Native and Hub declarations select the correct host services; unsupported or
   ungranted calls are denied, and non-AI apps allocate no peer.
2. Rinx and another app run concurrently using one kernel and shared AI settings,
   without opening AppCard first. Both remain independently addressable.
3. The system agent sends input, receives a peer question, answers it, observes
   continuation and interrupts work. Replies and approvals reach the right app.
4. Launch, close, restart and logout preserve ownership and revoke stale leases;
   no old reply reaches a new account or instance.
5. Workspace, transcript, memory and tool tests demonstrate app/account isolation,
   including automatic memory injection and simultaneous Rinx mini-app requests.
6. Selecting a configured model affects only the intended peer, keeps credentials
   with the host and reports fallback; it does not change the profile default.
7. Standalone local and external-provider modes work without OctoSense. Hosted
   Rinx cannot spawn a second kernel, and closing it leaves other peers usable.
8. Current macOS desktop and OnePlus 6 ROM Home builds exercise these flows;
   record native evidence and exact revisions separately from headless tests.

## Implementation record

The decision is implemented across six repositories. Kernel contracts came
first, and each consumer builds on the published revision of the one before
it. The review questions were resolved with the conservative options below and
are listed for the owners' confirmation.

| Layer | Change (merge) | What it provides |
| --- | --- | --- |
| Kernel | octos-org/octos#2555 (`552767dd`), UPCR-2026-034 | `peer/prepare` with `model` (parity with `peer_handoff`; effective model and fallback reported), `memory_namespace` (host-owned app peer bound to its workspace and an app/account memory namespace, system originator persisted), `resume`, and a kernel-provisioned workspace for remote hosts. Control of a host-owned peer is bound to a host token minted at creation (only its SHA-256 is stored). Allocation is exclusive: no shared namespace, context subspace or workspace. Torn bindings fail closed. `peer/model/set` changes one peer's lane between turns without touching the profile default. `peer/context/open\|close` provide bound request contexts. Memory isolation covers capture, retrieval, prompt injection, episodes and the refresh sweep |
| Capability contract | OctoSense-App-Hub#14 (`9e986135`) | The exact `octos.*` and `matrix.*` service names, admitted by exact name, with store wording; history does not grant turns |
| Broker | OctoSense-System-Apps#14 (`3daf54e8`), #15 (`64aa865e`), #16 (`4fa0f122`), #17 (`cc1cee3e`) | `crates/app-peers`: the contract Rinx consumes (`OctosAppService`, `OctosContext`), creation-time injection (`offer` / `claim` / `withdraw`), and one broker for all three deployments. It keeps one system-owned peer per app and account and one kernel request context per mini-app instance. The lease is checked on every request and before every reply, and host tokens are kept. The connector authenticates remote servers by token only. #17 moves the shell kernel to octos `552767dd` |
| Runtime | makepad#35 (`db4691d0`), OctoScript-Makepad#46 (`c3d53ba8`) | Splash `reapply_text` and stateful mini-app inputs on the current runtime (the ADR 0005 additions from makepad#32 / OSM#40, without #32's Android Back part) |
| Shells | OctoSense-Desktop#50 (`c806921c`, then #41 `1e8b217`), OctoSense-ROM#34 (landed with #24 `1dc753a7`) | `app_peers_host`: a module whose declared `octos.*` services host policy grants gets a scoped service for its instance at `create`, released at teardown. Rinx is linked as a module (`octosense-module` only). CI keeps its standalone features out of every graph. No `www.github.com` patch; AppCard stays opt-in |
| Rinx | #25 (`086208d7`, stacked on #24; on main with #24 `5cf70a82`, which both shells pin) | `src/host/service.rs` selects the deployment: hosted from module creation, standalone local or remote from Rinx's settings. `module.rs` declares the four `octos.*` services and claims the injected service. Mini apps get request contexts, and admission and dispatch use exact names. Account changes revoke, hosted close releases, standalone exit stops only an owned runtime |

`KernelProvider::shared` and AppCard connection discovery are gone. No mode
starts a fallback kernel or substitutes an ordinary privileged session. A hosted
Rinx without an injected service has no assistant; its settings entry points to
OctoSense's Settings → Accounts → AI providers.

### Decisions on the open review questions

- **Permission prompts.** Tool approvals raised in a mini app's context go to
  Rinx's native approval UI and are answered by the person. The system agent's
  ability to answer a peer's question is never used to approve a tool.
- **Background work after close.** This is the conservative choice. Closing a
  mini app closes its context and interrupts its turn. Closing Rinx releases its
  contexts and interrupts its peer's running turn. The peer, its workspace and
  its memory stay for the next launch, and the shared kernel keeps running.
- **Model selection.** Host policy may set a configured lane for an app's peer
  (`BrokerConfig::model_lane`, `peer/model/set`). No app-facing API selects a
  model or provider.
- **Credentials.** The standalone remote server token stays in memory and is
  asked for again after a restart; only the URL and profile are persisted. The
  kernel's per-peer host token is a capability for the app's own peer, kept
  with mode 0600 (shell: `<core_dir>/../app-peers`; Rinx: `octos/peers`) so a
  restart resumes the peer.
- **Session-plane trust.** The kernel enforces a bound session's workspace,
  memory and closure. Who may drive a profile's sessions is the profile's raw
  surface, as for every session. Apps never receive raw OUP.
- **The system agent acts in Rinx through Rinx's agent.** When the system
  agent's event-driven driver (later work) wants Rinx to act, it sends input
  to Rinx's peer, for example "open the team room and draft a reply saying X".
  Rinx's peer then calls Rinx's tools. The system agent never holds Rinx's
  tools. The tools are defined once, independent of transport
  (`src/assistant/mod.rs`, `TOOLS`). Today the shells' chat pane reaches them
  through Rinx's `ServiceExecutor`. A later host route, where the app-peers
  broker serves the same manifest to Rinx's peer, calls the same entry point.
  The octos app-tool protocol is not part of this change.
- **Room reads are granted per account and persisted.** The first `read_room`
  of a room shows Rinx's read sheet with three answers: allow once, always
  allow, or deny. If the person does not answer within 45 seconds, that
  counts as a denial. "Always" is stored for the signed-in Matrix account
  only (`<data>/assistant/room_grants.json`). Another account on the device
  never inherits it. Settings → Privacy → Assistant access lists the granted
  rooms and revokes them.
- **Rinx owns the confirmation of risky actions.** `send_message` shows the
  room and the exact text in Rinx's own sheet and sends only on the person's
  yes. That sheet is the only confirmation. The tool is declared
  `ToolDef::confirmed_by_app()` (OctoSense-org/makepad#36), so the chat pane
  skips its own confirm card. A host honours that mark only for in-process
  modules: it clears the mark from other processes' registrations, and a
  destructive risk floor overrides it. `draft_message` fills the composer
  and never sends.

### Acceptance evidence (2026-09-27)

Kernels are release builds of octos#2555. Models are standard-library Python
fixtures. No provider key or Matrix credential is used anywhere.

| Criterion | Evidence | State |
| --- | --- | --- |
| 1. Declarations select services; ungranted calls denied; non-AI apps allocate no peer | App Hub admission tests (exact names, prefixes refused, history ≠ turn). app-peers `apps_without_assistant_services_allocate_no_peer` and `history_access_does_not_allow_a_turn_and_ungranted_apps_get_no_context` (no kernel connection). Shell `a_granted_module_gets_its_service_at_creation_and_others_get_none` (ROM Home and Desktop) | Met (headless) |
| 2. Rinx and another app share one kernel and settings without AppCard, independently addressable | app-peers `two_apps_share_one_kernel_and_closing_one_leaves_the_other_usable` (real kernel: one generation, two connections, distinct peers owned by `_main:api:octosense#system`). Native Desktop run: Rinx launched from the dock as an in-process module with AppCard neither linked nor opened | Met headless; native launch verified. A signed-in native exchange is unverified |
| 3. System agent input, peer question, answer, continuation, interrupt | `the_system_agent_and_the_app_peer_exchange_a_question_and_answer`: `peer_send_input` → `ask_user_question` → kernel wake → `peer_respond` → continuation. `closing_the_app_interrupts_its_peers_running_work`: the turn goes `active` → `interrupted`. Both on a real kernel with a scripted model | Met (headless, scripted model) |
| 4. Launch/close/restart/logout preserve ownership; no old reply reaches a new account or instance | Kernel: closed or never-opened contexts are refused at bootstrap and at every turn start, and control needs the host token. Broker: a late reply after an account switch delivers nothing (scripted and real kernel). A restart resumes the same peer via the persisted token (real kernel, and Rinx `standalone_local`) | Met (headless) |
| 5. Workspace, transcript, memory and tool isolation, incl. automatic injection and simultaneous mini-app requests | octos `should_isolate_app_peer_workspace_and_memory_from_the_system_and_each_other`, `should_open_isolated_request_contexts_and_refuse_them_after_close`, `should_refuse_a_binding_that_shares_state_with_another_app_peer`, `should_never_extract_an_app_bound_session_into_the_profile_memory` | Met (kernel tests) |
| 6. Model selection affects only the intended peer, keeps credentials with the host, reports fallback, leaves the default | octos `should_select_a_configured_model_for_one_peer_without_touching_the_profile_default`; a request context runs on its peer's lane | Met (kernel tests) |
| 7. Standalone local and remote work without OctoSense; hosted cannot spawn a kernel; closing leaves others usable | Rinx `tests/standalone_local.rs` and `tests/standalone_remote.rs`. The shell tests `rinx_is_hosted_with_the_shells_service_and_starts_no_kernel`. Shell CI keeps Rinx's `standalone` / `octos-local` / `octos-remote` out of every graph. Native Desktop run: no kernel process started by launching Rinx | Met (process level + native launch) |
| 8. macOS desktop and OnePlus 6 ROM Home native flows | macOS: native standalone discovery and earlier hosted launch/close/reopen. OnePlus 6: standalone and separate hosted validation APKs launch; matrix.org discovery, single-event Back/edge swipe, IME dismissal, background/resume and restart verified (details below) | **Partially verified**: signed-in mini-app/approval flows and privileged production ROM integration remain unverified |

Deployment completion criteria:

| Mode | Evidence | State |
| --- | --- | --- |
| Standalone local | `standalone_rinx_owns_one_local_runtime_and_stops_only_it`: no kernel before the first request, then the packaged kernel (explicit path, never PATH) under Rinx's data root. Two mini apps share it with separate contexts. Exit stops Rinx's kernel and leaves another owner's running. A restart resumes the same peer | Met at process level |
| Standalone remote | `standalone_rinx_uses_an_explicit_authenticated_remote_server` against `octos serve --auth-token <fixture>`: a wrong token is refused, a turn runs in a server-provisioned workspace, the server token is never written to disk, an account change drops the late reply, and exit leaves the server running | Met at process level |
| Hosted desktop | Launch before AppCard: native (Desktop#50 build, hidden window). System peer, question/answer and second-app survival: real-kernel broker tests. A signed-in Rinx mini app exchanging with the system agent natively: not run (needs a Matrix account) | Partially verified |
| Hosted ROM Home | Consolidated OctoSense test APK installed separately on OnePlus 6: native launch, Back, edge swipe, background/resume and restart. No Rinx-owned kernel starts before authorization | Partially verified; signed-in mini-app flows and the privileged installed Home/Bridge are not covered |

Known limits and follow-ups:

- The initial repeated-input check passed against `552767dd` after the
  scripted model used unique tool-call IDs (System Apps #18). That did not
  cover providers that reuse an ID in a later turn. The consolidated
  OctoSense fixture deliberately reuses `call_1`; kernel `a6ea8505` handles
  that case. Both the question/answer exchange and the next system-to-peer
  input now pass on the real Android kernel. The earlier unique-ID result
  alone was insufficient to claim repeated-input compatibility.
- Rinx's `ServiceExecutor` now serves the first action set: `status`,
  `list_rooms`, `open_room`, `draft_message`, `read_room` (grant-gated),
  `open_mini_app` (the reviewed app only; Run still grants it) and
  `send_message` (confirmed in Rinx). Matrix reads and sends go through the
  mini-app adapters under a `Lease` for the app `assistant`, the account and
  the one room. If the account changes or signs out, waiting and running
  calls end `Unavailable` and late results are dropped. Rinx's peer does not
  call these tools yet; that needs the host route described above.
- In-flight turns fail with a retry message when the shell restarts its kernel
  (provider change). The broker reconnects and resumes the peer on the next
  request.
- Per-app token/tool budgets and fair scheduling beyond the kernel's existing
  limits are not part of this change.

### Deployment validation follow-up (2026-09-26)

The initial validation used System Apps `cc1cee3` and kernel `552767dd`,
matching the shell contracts before the repository consolidation. [Runtime packaging](../../packaging/README-octos.md)
builds the pinned executable for desktop and Android and includes its license.
Hosted builds continue to use only the injected service.

Rechecked in isolated worktrees and disposable data directories:

- All 20 app-peers tests pass, including five tests using the real pinned
  kernel and a scripted model. The repeated-input regression is in
  [System Apps #18](https://github.com/OctoSense-org/OctoSense-System-Apps/pull/18).
- Standalone local and authenticated remote process tests pass with the
  aligned dependency. The local test also passes from a private bundle with
  `RINX_OCTOS_BIN` removed, finding the adjacent packaged executable.
- Six packaging checks pass, including seven-character Git revisions from
  shallow source builds. cargo-packager 0.10.1 produces a macOS bundle
  with `Contents/MacOS/octos` and its license. This inclusion check reuses the
  existing fast-build Rinx binary; it is not a notarized distribution test.
- Native Makepad instrumentation verifies the standalone development bundle's
  login screen and hosted Rinx's launch, close and reopen in the rebuilt
  Desktop #41 shell. Both native test processes exited after capture. The
  hosted launch starts no child kernel before an authorized request.
- The pinned Android kernel cross-builds as an aarch64 ELF PIE with 16 KB
  load-segment alignment and only Android system-library dependencies.
- Standalone Android builds as the separate `dev.makepad.rinx.validation`
  package. APK v2/v3 signature verification passes and the bundled
  `lib/arm64-v8a/liboctos.so` matches the staged kernel's SHA-256 exactly.
  This development APK is installed under its separate package on OnePlus 6.

Signed-in native mini-app AI/approval flows still require a disposable Matrix
login. No normal Rinx account or AI credentials are copied into the test apps.

The merged ROM Home source also produced development-signed Home and Bridge
APKs using the ROM-patched cargo-makepad. The Home APK contains the exact pinned
kernel. Neither APK was installed; the installed Home remains untouched.

#### OnePlus 6 follow-up (2026-09-27)

The phone runs Android 15. The packaged kernel starts under the separate test
app's UID and reports `552767d`, the abbreviated pinned revision. Sixteen
checks pass against that Android executable over stdio with a scripted model:
two app peers and three isolated request contexts, streamed/persisted replies,
history isolation, host-token rejection, repeated turns, the system agent's
input/question/answer exchange and second input, and context closure while
another context remains usable. This exercises the kernel protocol, not a
signed-in native mini-app flow.

The native standalone UI exposed a reqwest 0.13 Android verifier panic during
the browser-registration metadata request. Auxiliary HTTP clients now use the
same bundled trust roots as the Matrix client, retaining certificate and
hostname verification. This also covers article image downloads and the
optional agent transport. Nine homeserver tests pass. The rebuilt test APK
successfully discovers matrix.org and renders its sign-in choices on the phone.

A separate `dev.makepad.octosense.rinxvalidation` APK launches Rinx in-process
and displays its sign-in screen. Its validation manifest omits the Home intent;
it does not replace the installed Home or Bridge. The unrelated privileged
bridge correctly rejects its test identity. Native Back testing exposed the
shell's Activity fallback closing the host before its Rust navigation handles
Back. Subsequent predictive-Back testing also caught duplicate compatibility
key events. The corrected callback and key routing are verified below.
Signed-in native AI/approval flows and production ROM integration are not yet
claimed complete.

#### Consolidated runtime validation (2026-09-27)

Rinx `e6443ae1` follows main’s app-peers pin `OctoSense@f38aa250`, Makepad
`6cf03859` and kernel `a6ea8505`. Runtime packaging follows that kernel pin.
The preceding results identify the earlier builds; these checks use the
updated graph:

- Six packaging checks and nine homeserver tests pass. Both Rinx CI jobs pass.
- On macOS, Makepad instrumentation verifies native startup and public
  matrix.org discovery. Real-kernel process tests pass for packaged local
  discovery (without `RINX_OCTOS_BIN`), per-context isolation, restart and
  owner-only shutdown, and for the authenticated remote deployment.
- OnePlus 6 standalone validation package version `2026092708` starts and
  discovers matrix.org without the verifier panic. Its packaged kernel's
  hash matches the source-build receipt and reports `a6ea850` on the phone.
- All 16 Android kernel checks pass using the consolidated scripted-model
  fixture, including repeated tool-call IDs across turns. These are protocol
  tests under the test app's UID, separate from native signed-in UI tests.
- The consolidated OctoSense dependency check finds one Rinx, one Makepad
  runtime and one shared Octos service; the hosted graph contains no Rinx
  standalone kernel features.
- The hosted validation APK (`2026092710`, OctoSense `33eaf68`, Rinx
  `e6443ae1`) includes the same verified Android kernel. It fixes the renamed
  phone crate's theme asset path and owns system Back through Android's
  dispatcher. The Makepad overlay suppresses the compatibility Back key
  while that callback is registered, preventing two navigations per press.
  The clean pinned runtime plus both patches reproduces the locked tree.
- On OnePlus 6, the Back key and an edge swipe each produce exactly one
  navigation event and return Rinx to the launcher without closing the host.
  IME Back dismisses the keyboard before app navigation. Reopen and
  background/resume keep the same host process. After process restart,
  Rinx launches again and discovers matrix.org. No separate kernel process
  exists under the host UID before an authorized request. These tests use
  the separate validation identity, not the privileged Home/Bridge identity.

The subsequent signed-in OnePlus check successfully read the Matrix profile
through the native mini-app, but exposed an account-binding error: login
invalidated the previous session before storing the new Matrix client, so
the injected assistant retained a signed-out account. Client replacement
now revokes old contexts, stores the new client, then binds the assistant.
Invalidation after login failure explicitly revokes instead of reading the
stale client. An offline regression using real SDK sessions and the broker
passes login, restore, account switch, login failure and logout for all
three deployments. Rinx's service dependency follows OctoSense `c806889`
and its renamed `octosense-kernel` crate.

The signed-in test also found that the mini-app adapter wrapped every
completed result as `text`, hiding `session_id` and `messages` from history
callbacks. It now preserves the broker's result while retaining the reply
size limit; regression tests cover session, history, turn and error results.

The follow-up OnePlus APK `2026092712` (OctoSense `0336018`, Rinx `11d794d6`)
includes both fixes and the merged shared shell from `c806889`. Native UI
checks pass for Matrix session restoration, profile access, Octos session
open/history, an AI reply, and a tool approval prompt whose Deny action
reaches the kernel. Closing during a delayed response returns to Discover
without a late UI update; reopening creates a new context and replies again.
System Back closes the mini-app; an edge swipe from Discover returns to the
launcher with the same host process. Process inspection finds exactly one
kernel child under that host. The standalone APK `2026092710` built from
the same Rinx source also starts and discovers matrix.org.

AI replies use a temporary scripted provider with the real packaged Android
kernel. Its profile and its own ADB reverse tunnel are removed afterward.
No Matrix messages were sent and no room access was granted to the fixture.
These checks do not claim paid-provider inference, native cross-app agent
exchange, approval acceptance, or privileged production Home/Bridge
integration. The validation packages do not replace normal Rinx, Home or
Bridge.
