# Native article mini app

Open **Discover → Article editor**, or **chat → + → Article editor**. Share app
sends an app card to a selected chat. The recipient opens App details, continues
with their own Robrix account and explicitly grants this open session permission
to save local drafts and propose a publication. They receive their own empty or
previously saved draft, never the sender's draft or permissions.

The editor supports a title, multiline Markdown, selection-based Markdown
formatting, local draft save, native HTML preview, chat name filtering and a
trusted final confirmation. Publishing sends a standard Matrix rich-text message
with a readable plaintext fallback, through the current user's SDK client.
Both English and Simplified Chinese are supported; Apple builds use the existing
PingFang theme setup. No WebView is instantiated by this app.

## Image-to-app flow

Workflow source: [Octoscript-AppCard lab/image-to-appcard-flow](https://github.com/OctoSense-org/Octoscript-AppCard/tree/main/lab/image-to-appcard-flow).
The local workflow revision used for intake is
`2f3bba1b840d222d026233b4db87030b74e9ff56`.

1. `source/atlas-prompt.txt` is the exact imagegen prompt. `source/atlas-v1.png`
   is the one generated eight-state atlas. Requested 2048×2048; actual output
   **1254×1254**. No model identifier was exposed by the built-in generator.
2. `image-to-appcard-flow.json` records measured, nonoverlapping atlas crops.
   Upstream `flow.py run --stages intake,prepare` produced the immutable input,
   crop and normalized 812×1552 reference hashes under `pipeline-output/intake`.
   Each scene's reference preserves aspect ratio rather than stretching it.
3. `build_mapping.py` writes explicitly authored semantic mappings and service
   actions for the eight scenes. Approximate reference region measurements are
   distinct from native widget geometry and are not a glyph measurement pass.
4. The **Robrix native adapter** uses `resources/mini_apps/article-editor/app.card`
   and `src/article_app`. Actual Octoscript L0 realization supplies both fields,
   localized placeholders and declared input transitions. Host views implement
   identity, permission, room choice and publication. `Html` renders sanitized
   Markdown. Text, inputs, buttons and room lists are real native widgets.
5. `tools/wechat-ux/live/native_article_app.py` exercises real Makepad input and
   screenshots using hidden windows, two isolated Palpo fixture accounts, and
   recipient-side Matrix event assertions. It also checks English/Chinese UI,
   cancel before grant, fresh consent on reopening and per-account drafts.
6. `verify_native.py` checks the mapping against source and optional native
   receipts, records source/binary/capture hashes, and leaves visual acceptance
   and a similarity score **unset**. No claim of a 9/10 WeChat similarity pass.

The upstream static Studio/WASM compiler does not implement this Robrix account
broker or native Html role. Its compile/capture/gate/website stages were not run;
the implementation is compiled with Cargo and exercised in the actual Robrix
host. This substitution is explicit, not an upstream Studio gate result.

## Reproduce

From the Robrix repository root:

```sh
cargo test --locked --features agent_chat --lib
cargo build --locked --features agent_chat --bin robrix
python3 tools/wechat-ux/check_i18n.py
python3 lab/article-editor/verify_native.py
# Requires an existing test-only Palpo fixture; credentials stay ignored.
python3 tools/wechat-ux/live/native_article_app.py
python3 lab/article-editor/verify_native.py --evidence lab/wechat-ux/evidence/live/article-editor/RUN_ID
```

To reproduce the reference intake with an Octoscript-AppCard checkout:

```sh
python3 /path/to/Octoscript-AppCard/lab/image-to-appcard-flow/flow.py run \
  --project lab/article-editor --manifest lab/article-editor/image-to-appcard-flow.json \
  --stages intake
```

`prepare` intentionally refuses to overwrite existing authored scene mappings.
Private profiles, credentials and raw runtime logs stay under the ignored
`lab/wechat-ux/evidence/live` directory. Selected fixture screenshots can be
reviewed separately from those session stores.

## Scope and authority

This first release admits only the built-in editor's exact app ID, version and
L0 source digest. The installed Robrix build is the trust root. A received card
cannot supply script source, a URL, a publisher claim, credentials or a grant.
Unknown versions/digests are rejected. This is not a general app catalog, a
publisher signature verifier, a raw Splash VM or a downloadable native plugin.

L0 is pinned at Octoscript `86a51a30767da5bf1f2e87559730f8539300b5d0` and has no
VM/ambient host calls. The field tree has explicit work/node/depth limits. The
host grant binds the account, login generation, random instance and one-hour
expiry; closing, session replacement and logout revoke it. Storage is under
an account hash and app ID, with atomic replacement and mode-0600 files. Local
drafts are not encrypted by this feature or synced across devices.

The host strips executable HTML, remote images/media, SVG and active links.
Preview and publication derive from the same sanitized content. Confirmation
captures the exact room, message and transaction. Membership, send power and
grant validity are rechecked before dispatch; the SDK client is captured before
awaiting, and the app never receives tokens. A failed send may be retried from
that confirmation with the same transaction ID. SDK automatic retries are
turned off for this operation. Closing cannot retract a request already
submitted to the homeserver; an interrupted session must not be presented as a
confirmed send failure. Cross-restart retry recovery is not implemented.

There is no separate remote editor backend, so it does not need a second OAuth
or Matrix OpenID login. Third-party services with their own accounts and signed
remote packages remain the future work described in ADR 0002. Publishing here
means sending to a selected Matrix chat; WeChat public-account subscriptions and
Moments publication are separate features.

## Validated build

On 2026-09-21 UTC, both `agent_chat` and `agent_ops_dev` library suites passed
**187 tests**, with the same two explicit external-backend tests ignored.
The existing Python UX suite passed 26 tests. Catalog validation passed with
819 bilingual entries, 934 translated call sites and no missing entries.
The new hidden-window native test passed all six two-account journeys against
Palpo. The native run exercised unencrypted DMs; it does not qualify encrypted
room delivery or an iOS device build.

[Side-by-side screenshot review](validation/index.html) and
[hash-bound native receipt](validation/native-validation.json) preserve the
actual captures. Native behavior passed; visual similarity remains unscored.
