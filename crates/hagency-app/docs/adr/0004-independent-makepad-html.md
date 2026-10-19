# ADR 0004: Independent makepad-html component

Status: accepted; supersedes the renderer ownership in ADR 0003.

## Decision

HTML/CSS engine work belongs to the independent
[OctoSense-org/makepad-html](https://github.com/OctoSense-org/makepad-html) repository.
It owns a standalone workspace, generic rendering API (`RenderedDocument`,
`RenderOptions`, `ResourceMap`), optional Makepad `HtmlView`, a native viewer,
headless tools, generic CSS regressions, pinned Blitz source/patches and HTML5 /
WeChat-derived compatibility suites.

Robrix links a pinned git revision with the Cargo alias `makepad-html-renderer`.
The alias distinguishes this renderer from the small `makepad-html` parser
already re-exported by Makepad widgets. `html_preview` enables the integration
and is now a default feature;
`article_blitz` remains a compatibility feature alias for existing commands.
No downstream Blitz Cargo patch overrides are necessary.

`apps/article-editor/native/preview.rs` remains an app-specific adapter: it converts validated
article documents to HTML, grants already-authorized asset bytes, schedules the
render, and checks the login/grant/request lifecycle before display.
Drafts, themes, Markdown, consent, Matrix credentials, storage, publication and
withdrawal remain outside makepad-html. The renderer does not depend on article-core,
article-makepad, Robrix, Matrix, OctoSense or Octoscript.

## Consequences

Other Makepad applications can use and test HTML rendering independently.
WeChat HTML/CSS is one compatibility suite, rather than the library's domain model.
Engine fixes and interaction work go to makepad-html; editor workflow changes stay
here. Removing the renderer feature still supports the regular native article UI.

The current pin is `b16565ae1e56dd7c3a87f00455ae327df599ca3b`, including the table,
text-shadow, horizontal ruby and inline-baseline fixes and the actual Huasheng
editor-export comparison corpus. The lab corpus records failures as well as
successful captures; it does not establish complete WeChat compatibility.

The app now keeps a worker-owned `DocumentSession` while its HTML/CSS preview is
open. Native clicks and horizontal scrolling reach that session; changed layouts
return new bitmaps, fragments request native scrolling, and HTTPS links pass
through the existing article link validator before the host opens them. Link
handling grants no network access to the renderer itself.

Every job and result checks the existing account/consent authority. The UI also
matches instance and request IDs. Back, close, refresh and account/grant changes
discard the old session; a dropped handle cannot publish queued results. A
bounded input channel and the existing process-wide render permit limit work.
Backend panics become preview errors and discard the failed DOM; they are not
treated as successful rendering or as a sandbox boundary.

Draft editing still uses the validated structured article document. This does
not add lossless arbitrary HTML editing, selection, SVG admission or the twenty
Huasheng themes as editor choices. Resource and active-content restrictions are
retained. `--no-default-features` keeps the renderer-free native article UI.
