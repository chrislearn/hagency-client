# A2App mini-app integration review

Reviewed 2026-09-20. Source: [a2app/robrix_a2app, commit
34c9d6d3](https://github.com/a2app/robrix_a2app/tree/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f).
This is a source compatibility review, not an imported or runtime-tested A2App
implementation. Our existing HTTP(S) web mini-app cards remain implemented in
[`src/mini_app.rs`](../../src/mini_app.rs).

Follow-up: [Octoscript mini-app review](OCTOSCRIPT-MINIAPPS-REVIEW.md) checks the
newer A2App `3cfc70e1` revision and proposes the Octoscript UI/runtime adapter.

## Conclusion

Yes: reuse A2App as a second mini-app runtime alongside web mini-apps. A2App
executes native Makepad Splash scripts; a URL still opens our native WebView.
The two can share a WeChat-style launcher and sharing flow without treating an
arbitrary website as executable Splash source. Neither mechanism alone provides
WeChat public-account subscriptions, publisher verification, or a publication
backend.

Recommended user flow: **Discover → Mini Apps → open / import / create → Share
to chat**. A received card opens an app details page, then Install/Open. Add the
same picker under the chat **+** menu. Keep the four primary tabs. A native app
opens in a full-height mobile page with Back and Share; desktop can reuse the
room pane. Recent/favorite web and native apps belong in the same launcher.

## Reusable implementation

| Component | Evidence in the reviewed revision | Integration |
| --- | --- | --- |
| Portable applications | [bundle.rs](https://github.com/a2app/robrix_a2app/blob/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/a2app/core/src/bundle.rs), [manifest.rs](https://github.com/a2app/robrix_a2app/blob/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/a2app/core/src/manifest.rs) | Preserve versioned `.splashapp` import/export; grant state is deliberately absent from bundles. |
| Native runtime and lifecycle | [instances.rs](https://github.com/a2app/robrix_a2app/blob/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/src/a2app/instances.rs) | Reuse per-app/per-room instances and their state across surface changes. Configure the isolate before evaluating source. |
| Permissions and Matrix services | [services](https://github.com/a2app/robrix_a2app/tree/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/a2app/core/src/services), [Matrix policy](https://github.com/a2app/robrix_a2app/blob/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/src/a2app/matrix/policy.rs) | Reuse host-side authorization, room scopes, revocation, request budgets and data-sharing rules; adapt the prompts to our bilingual UI. |
| Chat cards | [timeline_card.rs](https://github.com/a2app/robrix_a2app/blob/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/src/a2app/timeline_card.rs) | Support received `rs.robius.a2app` events and render a compact native mini-app card. Receipt/rendering must not execute the app. |
| App management | [core](https://github.com/a2app/robrix_a2app/tree/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/a2app/core/src), [mini_apps_screen.rs](https://github.com/a2app/robrix_a2app/blob/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/src/a2app/mini_apps_screen.rs) | Keep registry, private storage and version history; adapt the management UI rather than replacing our navigation. |
| AI generation | [agent crate](https://github.com/a2app/robrix_a2app/blob/34c9d6d3d0266a2240d4b5d7abfecf71859cc54f/a2app/agent/Cargo.toml) | Optional later slice: imported/built-in apps should run without an AI provider. |

## Concrete compatibility work

1. **Makepad is the first prerequisite.** We pin
   `button-grab-key-focus@47837267`; A2App pins
   `splash-host-io-cancel@a8a210f2`. Our revision already has Splash, host
   requests, capability tags and jailed storage, but lacks
   `Splash::set_host_io_only`, `validate_splash_body_with_host_io`, and
   `alloc_splash_vm_with_host_io`, which A2App calls. Reconcile those changes
   in one tested Makepad revision for widgets and code editor. Simply removing
   the missing calls would discard the host-enforced I/O boundary.
2. **The Matrix SDK pin matches:** both lockfiles resolve
   `6892cb217ae4a886571e928c8efcccfbec5490a6`. This reduces adapter work but does
   not prove the fork's full application glue will compile here.
3. **Separate runtime from generation.** Upstream `a2app` enables both core
   and agent crates; its runtime also manages AI rooms. Introduce separate
   runtime/generator features rather than importing its complete runtime into
   our existing `agent_chat` module. For AI generation on iOS, upstream
   documents its embedded backend instead of a subprocess worker.
4. **Keep both wire formats.** Our web cards use `m.room.message` with msgtype
   `rs.robius.robrix.mini_app` and a `mini_app` object. A2App shares a separate
   `rs.robius.a2app` event whose content contains a JSON-string `bundle` and
   `body`, using the SDK's `send_raw`. Add a separate event renderer/parser and
   forwarding adapter. Test encrypted receipt, redaction, malformed bundles,
   unknown versions, size limits and a readable file/text fallback. Other
   Matrix clients need not render A2App's custom event.
5. **Integrate our room semantics.** Upstream room listing iterates joined and
   invited rooms behind its policy filter; it has no Robrix Moments type
   handling. Preserve our Moments exclusions in ordinary room pickers, and
   require explicit access before exposing Moments or private File Transfer to
   an app. A chat association must not imply account-wide read access.
6. **Apply English/Chinese and PingFang.** Translate host chrome and prompts
   through our catalogs; leave app code, sender names and user data untouched.
   Expose locale to native apps with a defined change notification. Check
   whether each app inherits our font theme; a separate VM is not proof that
   it does.

## Proposed implementation order and acceptance

1. Add the reconciled Makepad runtime and core behind a feature flag, then run
   a bundled offline sample in an isolated native test profile. Keep current
   web mini-app behavior passing.
2. Add Discover's unified launcher, import/export, full-page native host,
   first-use permissions, uninstall and saved state. Test denied/revoked access
   and lifecycle across room/account changes and app restarts.
3. Add sharing and received-card installation. Use two encrypted fixture
   accounts on Palpo and Synapse: send → receive → inspect → install → run →
   forward. Permissions and local app data must not travel with the bundle.
4. Add optional AI creation and version editing after those journeys pass.
   Reuse the guarded generation path rather than exposing Matrix credentials
   or unrestricted host APIs to generated code.

Acceptance also needs Chinese/English mobile and desktop captures, back
navigation, draft preservation, a malformed app that cannot affect another
app, and physical iOS testing. The upstream source includes broker/isolate
tests, but this review did not execute them and is not a security certification
or a WeChat 9/10 visual-parity result.
