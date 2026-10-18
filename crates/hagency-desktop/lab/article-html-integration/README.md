# Updated native HTML/CSS article preview

2026-09-21. The `wechat-ui` article app now uses
[`makepad-html b16565a`](https://github.com/OctoSense-org/makepad-html/commit/b16565ae1e56dd7c3a87f00455ae327df599ca3b),
including its table, text-shadow, horizontal ruby and inline-layout fixes.
The dependency and its vendored Parley revision are pinned in `Cargo.lock`.

HTML/CSS preview is enabled in the default build. In Robrix, open Discover →
Article studio, authorize the mini app, open a draft, then Full preview →
HTML/CSS preview. It renders to a native Makepad texture, without a WebView.
The structured native editor remains responsible for editing and selection.

The app retains a document session on one worker and routes native link taps,
fragment requests and horizontal scrolling through it. Links use the existing
HTTPS validator and host opener. Refresh, back and close discard the old session;
account and consent checks run before work and before result delivery, alongside
the UI's instance/request checks. Backend panics produce a preview error and
discard the failed DOM so another preview can be opened.

## Validation

[validation.json](validation.json) records the exact dependency, source and
binary hashes. The native runs used an isolated profile and disposable fixture
account on the existing Palpo test deployment; no personal accounts were used.
Credentials, private profiles, login logs and raw input traces are not included.

- Ten article-module unit tests pass, including native worker link/disclosure
  dispatch, cancellation, authority revocation, backend error recovery, escaped
  document conversion and image sizing.
- Default-feature compilation and an `agent_chat` build pass. A separate
  `--no-default-features --features agent_chat` check verifies the renderer-free
  build remains available.
- Actual Makepad UI tests pass at 430×820 and 1440×960: Chinese/English labels,
  authorized cover/body images, paper theme, full scrolling, long-document
  clipping notice, refresh/back/reopen, and closing the article app.
- The component's [CI run](https://github.com/OctoSense-org/makepad-html/actions/runs/35675362008)
  passed independently.

Native captures: [Chinese preview](evidence/mobile-top.png),
[article end](evidence/mobile-bottom.png),
[length limit](evidence/mobile-clipped.png),
[English desktop preview](evidence/desktop-top.png).

```sh
cargo +1.98.0 test --locked --features agent_chat --lib article_app::
cargo +1.98.0 build --locked --features agent_chat
python3 tools/wechat-ux/live/native_article_blitz.py
cargo +1.98.0 check --locked --no-default-features --features agent_chat
```

The native script requires the existing private Palpo fixture and test artwork.
It chooses an unused loopback instrumentation port, verifies the process it owns,
and does not stop other running Robrix applications.

## Scope

This integrates the updated renderer, not arbitrary HTML editing or twenty new
editor themes. The Huasheng comparison remains in makepad-html, including the
four failed grayscale/sepia captures. Catching a backend panic here does not fix
those filters or certify WeChat fidelity. OS file-picker interaction and
iOS/Android device rendering were not exercised in this pass.
