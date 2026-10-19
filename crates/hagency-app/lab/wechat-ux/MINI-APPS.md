# Web mini-app cards

The follow-up request adds shareable HTTP/HTTPS web pages to the Matrix client.
In a chat, choose **+ → Share mini app**, enter the address and title, optionally
preview the page, choose a chat, and send. Tap a received card to open it. The
viewer has Back, Reload, Open in browser, Share and Close controls. Share reuses
the original card URL; it does not claim to share a later page reached inside
the website.

Makepad's `CxSystemBrowser` embeds a native `WKWebView` on macOS and iOS. The
platform's browser process executes the website. There is no JavaScript bridge
to Robrix, Matrix access tokens, room history or native app actions. Merely
receiving or drawing a card does not load its website. Other platforms retain
card sharing and offer Open in browser.

The payload is an `m.room.message` event with custom msgtype
`rs.robius.robrix.mini_app`:

```json
{
  "msgtype": "rs.robius.robrix.mini_app",
  "body": "[Mini app] Example\nhttps://example.com/app",
  "mini_app": {
    "version": 1,
    "title": "Example",
    "url": "https://example.com/app"
  }
}
```

Robrix adds this msgtype to its Matrix timeline filter and renders native cards.
The body supplies readable title/URL fallback data to other clients; their
handling of unknown msgtypes varies. Sending uses the logged-in Matrix SDK's
room send API, including its encryption path, and reports success after the
homeserver acknowledges. A failed attempt keeps its transaction ID for an
unchanged retry. Choosing a different chat or changing the card starts a new
transaction. Thread shares retain the source thread relation.

Both local input and received metadata are validated. Only HTTP and HTTPS URLs
with a host are accepted; embedded credentials, control characters, oversized
URLs/titles and unknown payload versions are rejected. The Apple app plists
allow HTTP specifically in web content, without broadening native HTTP policy.

`tools/wechat-ux/live/native_mini_apps.py` drives an isolated native client and
serves a small interactive page on loopback. It verifies Matrix recipient
content, actual WebKit pixels, JavaScript button input, in-page navigation,
sharing to another chat, incoming cards and return to the conversation.
WindowServer screenshots include the native web view; Makepad's GPU captures
alone cannot prove that a WebKit page rendered. Receipts are private under
`evidence/live/native-mini-apps.json` and its immutable run directory.

Run `python3 tools/wechat-ux/live/native_mini_apps.py --headless` for Makepad's
hidden-window instrumentation mode. This checks the absence of an on-screen
window, form/card input, exact Matrix delivery, sharing to another chat and
incoming cards through the loopback automation bridge. The local HTTP fixture
also records WebKit's JavaScript load callback and reload while hidden. Its
separate `native-mini-apps-headless.json` receipt does not claim native web-page
pixel or pointer-interaction coverage; those come from the visible run.

This is web-page card sharing inspired by the requested WeChat flow. It does
not implement Tencent official-account subscriptions, a mini-program runtime,
payments or a verified publisher directory. Native WebKit permission/error
handling and real iOS device behavior still require release qualification.
No 9/10 visual similarity score is awarded by these tests.

# Apple typography

`src/apple_fonts.rs` resolves installed PingFang SC Regular/Semibold through
CoreText and selects the correct face from Apple's font collection. It installs
the faces before Makepad widgets inherit the theme, so labels, text inputs,
messages and bold rich text use PingFang for English and Chinese. Existing CJK
and emoji fallback remains. Code and icon fonts retain their specialized faces;
PingFang has no italic face, so italic prose uses its upright weight. Apple font
files are never included in the repository or redistributed in the app.

Rust tests check English, simplified/traditional Chinese glyphs and outlines,
distinct weights, theme inheritance and fallback order. `native_fonts.py`
checks rendered mixed-language composition, exact UTF-8 delivery to Palpo and
incoming bold text. iOS shares the implementation but has not been built or run
on this machine, which has only macOS command-line developer tools.
