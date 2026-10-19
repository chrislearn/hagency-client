# Robrix mobile WeChat UX conversion

Use AppCard as a **reference measurement and native-widget verification pipeline**,
then connect the reviewed components to Robrix's existing room/timeline/profile
controllers. It is not a screenshot-to-Matrix-API compiler. The user selected
mobile first: **Chats / Contacts / Discover / Me**.

The initial [production scope](SCOPE.md) was **Matrix messaging and four native
tabs**. Subsequent requests add web mini-app sharing and the first Moments slice.
Payments, calls and broader WeChat parity remain separate work. The original
visual acceptance gate remains unchanged.

This directory contains an implementation specification and an executable
reference/evidence workflow. **The scoped Matrix/four-tab port is implemented and
runtime-tested; 9/10 similarity has not passed.** See [implementation results](IMPLEMENTATION.md)
for native changes and live Palpo testing. The generated reference is not a capture of
WeChat and is not an authoritative specification of any particular WeChat version.
See [current visual gaps](REVIEW.md) and the local
[native comparison gallery](evidence/live/review.html) for the remaining work.

## What is available now

- [English and Simplified Chinese](I18N.md): persistent language selection,
  bilingual main flows and native draft-preservation tests.
- [A2App integration review](A2APP-REVIEW.md): source compatibility and a
  proposed native Splash runtime alongside existing web mini-app cards.
- [Moments and private File Transfer](MOMENTS.md): encrypted personal timelines,
  invited audiences, posts/albums, comments/likes, and separate self-messaging.
- [Joined Spaces in Chats](JOINED-SPACES.md): All Chats / Spaces switch,
  named expandable memberships, nested rooms, and Space-aware name search.
- [Room search and shared attachments](ROOM-HISTORY.md): search decrypted room history,
  browse photos/videos, files and audio, and return to the original message.
- [Chat actions and forwarding](CHAT-ACTIONS-FORWARDING.md): swipe actions, local history clearing,
  message selection, individual forwarding, and bundled chat-history cards.
- [Explore Rooms and back navigation](EXPLORE-ROOMS.md): plain-name directory search,
  joined-room matches, a fixed back button, and trackpad back gestures.

- [Mobile Settings and profile details](MOBILE-DETAILS.md): grouped native Settings,
  separate Personal Information editors, and full-width contact pages backed by Matrix.
- [Interactive flow explorer](index.html): 48 states, 22 journeys and per-screen
  API mappings. Build it with the command below; it opens directly in a browser.
- [Flow catalog](flow-catalog.json): navigation, visible state, inputs, back/draft/
  scroll behavior, error branches and acceptance procedures.
- [API map](api-map.json): 45 actions, with real source locations and known gaps.
  These are **binding specifications**, not installed runtime handlers.
- [Captured public mock](source/mock-captures-v2/capture-receipt.json): full-size
  800×1600 screenshots from the published Makepad WeChat WASM demo, with input
  coordinates, WASM hash and browser/viewport provenance. These are mock captures,
  not captures of Tencent WeChat. See the [capture review](source/mock-capture-review.md).
- [Original generated atlas](source/atlas-v1.png) and its
  [exact submitted prompt](source/atlas-prompt.txt), generated with the built-in
  image tool. The tool did not expose an underlying model identifier.
- [AppCard manifest](image-to-appcard-flow.json): 12 measured crops in one atlas.
- [Source receipt](source/generation-receipt.json): requested versus actual size,
  hashes and provenance.
- [Current acceptance result](acceptance-status.json): fails for missing native
  captures/reviews. No visual score has been assigned.

The atlas requested 3840×5760 and returned **1024×1536**. Individual source screens
are about 236–238×455–464 pixels. Intake produces 812×1552 references by fitting
these crops to a 406×776 artboard at 2×. **This is enlargement, not additional
source detail.** Use this atlas to discuss the flow. Obtain full-resolution
references before accepting typography, icons or fine geometry. The requested
390×844 screen shape was also not faithfully followed; measured crops, rather
than prompt coordinates, are recorded in the manifest.

`intake` and `prepare` ran successfully through the upstream tool. Their original
receipts and prepared scenes are local under `pipeline-output/` and `scenes/`.
They are reproducible and ignored by Git. Source atlas, prompt, manifest,
catalog, API mapping, tooling and the concise run status are retained.

## Findings from the two repositories

Inspected revisions:

| Source | Revision | Role |
| --- | --- | --- |
| [Octoscript-AppCard](https://github.com/OctoSense-org/Octoscript-AppCard/tree/a6ea4e2c33a9e35ec27a9e94d6c40ff3386ee3cc/lab/image-to-appcard-flow) | `a6ea4e2` | Atlas intake, semantic/native compilation, Studio gates |
| [makepad_wechat](https://github.com/project-robius/makepad_wechat/tree/02d2f1e97b2f7a609016f928b9ee1c6d1d72cefb) | `02d2f1e` | Mock navigation/layout reference |
| Robrix in this checkout | `7c98054` | Real Matrix actions and native app controllers |

The AppCard runner preserves source hashes, reversible crop transforms and
immutable execution receipts. `prepare` installs references but intentionally
does not invent a semantic mapping. `semantic`/`compile` need reviewed
`contract.json`, `mapped.json`, `semantic-map.json` and `service-actions.json`.
Compilation produces L0, kit JSON, data and source-to-native-widget mappings;
it does not modify Robrix's Rust source. Its stock capture/gate targets the
AppCard Kit host. Acceptance of that host does not establish acceptance of Robrix.

The current `flow.py` requires `artboard: [406,776]` and `locales: ["en","cn"]`.
The 12 generated references are English only; declaring locales is not proof
that Chinese has been generated or tested. The compiler emits English copy from
the supplied tree. Chinese copy, real CJK fonts and separate captures still need
authoring. Real device viewports and native keyboard behavior require additional
Robrix checks.

The flow tool's stock example concerns service cards; its prompt deliberately
excludes chat. That restriction belongs to that example. Robrix should keep
chat as its core navigation and use independently owned AppCard subtrees for
actual service/agent content. Do not reuse the aircon reducer, payment rules or
website controller as Robrix business logic. WASM/Astro packaging is optional
for a reference gallery and does not replace the native Matrix client.

`makepad_wechat` explicitly describes itself as mostly UI screens without business
logic. Its useful examples are:

| Reference | Reuse in the design | Robrix destination |
| --- | --- | --- |
| `src/app.rs` | Four root tabs, pushed screens without tabs | `home/home_screen.rs`, `home/navigation_tab_bar.rs` |
| `src/home/chat_list.rs` | Avatar, title, preview, timestamp row | `home/rooms_list_entry.rs`, `home/rooms_list.rs` |
| `src/home/chat_screen.rs` | Left/right bubbles and composer arrangement | `home/room_screen.rs`, `room/room_input_bar.rs` |
| `src/contacts/*` | Alpha grouping and contact/profile flow | New Contacts controller over explicit contact data |
| `src/discover/*` | Grouped rows and Moments layout | New Discover routes; fixture backend initially |
| `src/profile/*` | Me and My Profile row grouping | Existing profile/settings handlers under a new Me shell |
| `src/api.rs` | Deterministic mock chat/message model | Test fixture adapter, never production identity/storage |

Do not copy its `live_design!`, `DefaultNone`, old resource syntax or widget
registration into this project. Follow this checkout's AGENTS.md and search the
new Makepad `widgets/src/` implementations first. `PageFlip`, `StackNavigation`,
`PortalList` and `RadioButton` are already available in the current script system.
No third-party implementation was copied into Robrix. The captured mock screens
are retained only as reference evidence, with the upstream
[Apache-2.0 license](source/MAKEPAD_WECHAT_LICENSE.txt). Do not extract its photos
or other bundled imagery into production assets without checking their provenance.

## Native conversion and API boundaries

The conversion chain should be:

```text
source screen + observed geometry + authored semantic intent
  -> reusable native row/bubble/tab/composer/profile components
  -> typed UI intent + current room/user/event identity
  -> existing Robrix UI handler or MatrixRequest
  -> actual async result / timeline update
  -> native state change and a fresh screenshot
```

Keep semantic IDs such as `chats.row.<room_id>`, `chat.composer.send`,
`contact.<user_id>.message`, `me.settings` stable across design contracts and test
selectors. Bind virtualized row identities when drawn; do not use row indices
as room IDs. Native controls must own hit testing and enabled/selected state.
Text stays in Labels/native text widgets; input stays in TextInput; scrolling
stays in PortalList. The screen image is never the rendered product UI.

| Intent | Existing integration | Important distinction |
| --- | --- | --- |
| Open conversation | `RoomsListAction::Selected(SelectedRoom)` | Keep the existing timeline and selection lifecycle |
| Send/reply/edit | `RoomInputBar` -> `SendMessage` / `EditMessage` | Preserve mentions, slash commands, reply relation, E2EE and optional TSP signing |
| Retry failed send | `MatrixRequest::RetrySend` | Reuse `timeline_event_id`; don't submit a second message |
| Photo/file | Existing picker -> upload preview -> `SendAttachment` | Cancel must send/upload nothing |
| Contact Messages | `OpenOrCreateDirectMessage { allow_create: false }` | Existing confirmation handles room creation if needed |
| View contact | `GetUserProfile` and existing profile cache | Room membership is not a persistent friend list |
| Mark unread/read | `SetUnreadFlag` / `MarkRoomAsRead` | Preserve preferred read-receipt privacy |
| Sticky on Top | `SetIsFavorite` plus room-list ordering work | A favorite flag alone does not prove sticky ordering |
| Change own profile | `SetDisplayName` / `UploadAvatar` | Await AccountDataAction; roll back failed changes |
| Invite/leave/block/logout | Existing modal + backend handler | Keep the existing confirmation and error behavior |

Of 45 mapped actions, 30 have existing source entry points, 5 have semantic gaps,
and 10 need implementation. The four-tab navigation conversion is implemented.
“Existing” means the source path exists; it does not mean the WeChat presentation
or an end-to-end native test has passed.

Material gaps found in this checkout:

- `home/search_messages.rs` has a disabled `Search (TODO)` control. Room filtering
  must not be presented as full message search, especially for encrypted history.
- Room notifications now use actual Matrix push rules and synchronized state;
  native checks cover all four choices, external updates and restart persistence.
- Contacts/remarks/tags, group creation and private saved items need data models
  and controllers. Moments now has its own [implementation](MOMENTS.md).
  Shared pinned messages are not private Favorites.
- Speech input is dictation, not WeChat hold-to-talk audio. Calls, QR scanning,
  payments and mini-program lifecycle need separate capabilities.
- Matrix redaction, local chat deletion and WeChat recall are different operations.
  Do not wire “Clear Chat History” to global `ClearEventCache` or room leave.

Use the permitted mocks for deterministic navigation, layout and interaction
development. Fixture mode must be visibly identified in developer evidence,
cannot use real accounts, and cannot count as live API completion. In production,
only present working capabilities or explicit unavailable states.

## Implementation sequence

1. **Freeze the reference set.** Select a WeChat OS/version/language if actual
   WeChat parity is the acceptance target. Capture the catalog's screens using
   synthetic contacts and identical content. Retain full-resolution originals,
   viewport, scale, keyboard mode, device, app/OS version and hashes. Generated
   references are an allowed design target, with a different acceptance claim.
2. **Build the four-tab shell.** Adapt the existing mobile `PageFlip`/stack;
   retain per-tab scroll and per-room draft state. Put Spaces behind a deliberate
   route rather than silently losing Matrix functionality. Keep Desktop separate.
   Add a scoped visual treatment instead of globally replacing the existing teal
   design tokens, which would change unrelated desktop/settings screens.
3. **Finish Chats first.** Reuse `RoomsList`, timeline, input and upload controllers.
   Match row density, square avatar geometry, time/unread hierarchy, chat canvas,
   bubbles, title bar and keyboard/tray/emoji transitions. Exercise empty, offline,
   sending, failure/retry, history pagination and reply states immediately.
4. **Contacts and Me.** Add explicit contact state and grouped native rows. Route
   Messages/profile/settings into existing handlers. Keep identity, verification,
   receipts, permissions and session recovery intact.
5. **Discover and extended flows.** Use clearly separated deterministic fixtures
   for Moments/service screens while implementing their real providers. Create
   additional 8–12-screen atlases for the remaining catalog states; don't pretend
   that 12 static screenshots describe all interaction/error behavior.
6. **Bind and validate incrementally.** Review scene semantics, compile reusable
   components, mount them in Robrix and drive actual widget bounds. Record API
   effects and failures with the same scenario IDs. Replace a mock only when the
   equivalent live journey passes.
7. **Repair until each required comparison is >=9/10.** Capture after every visual
   repair; retain failed rounds. Never manufacture a pass by widening tolerances,
   resizing only the candidate, lowering the screen count or reusing old reviews.

## Reproduce the prepared reference pack

From the Robrix repository root, use Python with Pillow installed. The upstream
documentation recommends Python 3.11/3.12 for its full OCR/compiler environment.
The local tooling itself uses only the standard library and Pillow.

```sh
python3 -m venv /tmp/robrix-wechat-tools
/tmp/robrix-wechat-tools/bin/python -m pip install -r tools/wechat-ux/requirements.txt
export UX_PYTHON=/tmp/robrix-wechat-tools/bin/python

git clone https://github.com/OctoSense-org/Octoscript-AppCard.git /tmp/robrix-appcard
git -C /tmp/robrix-appcard checkout a6ea4e2c33a9e35ec27a9e94d6c40ff3386ee3cc
export APPCARD_ROOT=/tmp/robrix-appcard
export WECHAT_PROJECT="$PWD/lab/wechat-ux"

"$UX_PYTHON" tools/wechat-ux/ux.py validate
"$UX_PYTHON" tools/wechat-ux/ux.py appcard-plan --appcard "$APPCARD_ROOT"
"$UX_PYTHON" "$APPCARD_ROOT/lab/image-to-appcard-flow/flow.py" run \
  --project "$WECHAT_PROJECT" \
  --manifest "$WECHAT_PROJECT/image-to-appcard-flow.json" \
  --stages intake,prepare
"$UX_PYTHON" tools/wechat-ux/ux.py report
open lab/wechat-ux/index.html
```

Run `prepare` once. On this checkout it has already run; only `intake` can replay
idempotently. New source/crop measurements require new versioned output/scene
directories. `report` also refuses to overwrite an old report; choose a new
filename **in this directory** when regenerating it, so relative image links work.

For subsequent authoring, install the full upstream
[native prerequisites](https://github.com/OctoSense-org/Octoscript-AppCard/blob/a6ea4e2c33a9e35ec27a9e94d6c40ff3386ee3cc/lab/core/REPRODUCE.md)
and matching Makepad/Octoscript/Kit-host workspace. The bare reference clone used
here is sufficient for intake, not Studio acceptance. Exact native fonts and
compatible Studio/bridge/Kit-host revisions are required.

```sh
# After authoring each contract.json, with the full upstream environment:
"$BEAUTY_PYTHON" "$APPCARD_ROOT/lab/image-to-appcard-flow/flow.py" run \
  --project "$WECHAT_PROJECT" --manifest "$WECHAT_PROJECT/image-to-appcard-flow.json" \
  --stages observe,measure,map
# Review mapping and semantic decisions, supply service-actions.json, then:
"$BEAUTY_PYTHON" "$APPCARD_ROOT/lab/image-to-appcard-flow/flow.py" run \
  --project "$WECHAT_PROJECT" --manifest "$WECHAT_PROJECT/image-to-appcard-flow.json" \
  --stages semantic,compile
# Through matching release Studio, after native authoring is complete:
"$BEAUTY_PYTHON" "$APPCARD_ROOT/lab/image-to-appcard-flow/flow.py" run \
  --project "$WECHAT_PROJECT" --manifest "$WECHAT_PROJECT/image-to-appcard-flow.json" \
  --stages capture --launch
```

To repeat collection from the published mock, install the optional Playwright
dependency in a separate environment, then run:

```sh
python3 -m venv /tmp/robrix-wechat-browser
/tmp/robrix-wechat-browser/bin/python -m pip install playwright
/tmp/robrix-wechat-browser/bin/python -m playwright install chromium
/tmp/robrix-wechat-browser/bin/python tools/wechat-ux/capture_reference.py \
  --output lab/wechat-ux/source/mock-captures-next
```

The capture script requires a fresh output directory and visual review of each
result. Its coordinates address real native canvas controls in the inspected
400×800 demo, but a future deployment may move them. The deployed commit is
unknown; the fetched WASM hash identifies the actual artifact. Its mixed-language
mock content does not substitute for two separately validated locale fixtures.
Keep capture provenance distinct from `generation` provenance. A capture intake
adapter is needed before feeding separate screenshots into the one-atlas flow;
do not falsely label these images as generator output.

The author/compile/Studio commands above are the next workflow, **not stages reported as executed**.
Use upstream `gate` for the generated Kit-host composition, followed by the
Robrix-specific acceptance process below after native integration.

## 9/10 means evidence, not a self-awarded score

The local gate requires all 48 catalog states in both EN and CN (**96 reviews**),
all 22 journeys in both locales (**44 traces**), and a named review of every
current reference/native pair. Each screen must score at least 9/10 for visual
match; every layout/typography/color/icon/density/state criterion must pass.
The score reported is the minimum per-screen review, not an average that can hide
a broken screen. Navigation, error handling and backend effects pass separately.

Test native tap/long-press/scroll/back input, not direct reducer calls. Required
checks include real controls, enabled state, no clipping, readable text, hit
targets, draft/scroll restoration and duplicate-effect prevention. The
`layout-input` journey additionally covers 360×780, 390×844 and 430×932 logical
viewports, keyboard/IME input, multiline drafts and safe areas. Record these
measurements in its assertion log. Accessibility includes labels, focus order,
contrast and usable touch targets; it cannot be inferred from a PNG alone.

Two independent labels must be present in an acceptance receipt:

| Field | Values | Meaning |
| --- | --- | --- |
| `claim` | `generated-target` / `wechat` | Similarity to an authored design versus actual versioned WeChat captures |
| `execution_mode` | `fixture` / `live` | Mock-backed native UX versus live backend integration |

Fixture mode is permitted by the requested scope. A successful fixture run
reports `production_ready: false` and lists backend gaps. Actual-WeChat claims
require real WeChat reference captures and app/OS/device provenance. Live mode
also fails while API mappings remain partial/missing/planned. A passing AppCard
host screenshot does not satisfy native Robrix evidence.

```sh
"$UX_PYTHON" tools/wechat-ux/ux.py fingerprint
"$UX_PYTHON" tools/wechat-ux/ux.py gate \
  --evidence lab/wechat-ux/evidence/acceptance.json
"$UX_PYTHON" -m unittest discover -s tools/wechat-ux -p 'test_*.py'
```

The gate exits 1 for missing/incomplete evidence, 2 for malformed inputs and 0
only for a complete passing receipt. See [EVIDENCE.md](EVIDENCE.md) for its schema.
Tests of this gate are not tests of the native UX. The gate checks supplied
evidence and hashes; it does not independently authenticate a reviewer or server.

For a read-only baseline from an already running test instance:

```sh
# Start a test instance with MAKEPAD_REMOTE=8099 using an isolated test account.
"$UX_PYTHON" tools/wechat-ux/ux.py capture \
  --bridge http://127.0.0.1:8099 --binary target/debug/robrix \
  --screen chats --locale en --output lab/wechat-ux/evidence/baseline-001
```

This records pixels, bridge state and native widget snapshot. It never clicks,
logs in, sends messages or quits an app it did not launch. Its supplied binary
hash is declared rather than process-attested, so it deliberately cannot be
used alone as acceptance evidence. Use an instrumented native runner/Studio
build receipt plus the full input and backend traces for acceptance. Keep
account captures local; `evidence/` is ignored by Git.
