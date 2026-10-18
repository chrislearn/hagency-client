# Rinx math rendering validation

Validated on 2026-09-22 using the supplied Editor.md/Gist fixture, unchanged at
9,146 bytes. All nine formula occurrences render in the native full reader and
production Blitz preview: display and inline dollars, math/katex/latex fences,
radicals, summations, nested fractions and integrals. The original TeX remains
editable; source apply, save and restart retain its spelling.

## Implementation

`article-core::math` extracts TeX before Markdown escaping, sanitizes source
HTML, and asks the host to typeset each formula. The native Makepad math engine
lays out glyphs and rules using the bundled NewCMMath font. `article-makepad`
rasterizes those into transparent 2× PNGs. The full reader uses an `ArticleMath`
inline widget; Blitz receives only the generated PNG resource grants. Neither
Rinx preview depends on a WebView, JavaScript or remote math fonts.

For this Editor.md fixture, rendering removes redundant outer `\( \)` within
`$$` and interprets Markdown-escaped underscores in math fences as subscripts.
Leading displaystyle/textstyle are interpreted; explicitly sized delimiter
pairs use content-sized brackets. These are preview adaptations, not source
edits. This is the pinned native parser's TeX subset, not full LaTeX. Formula
length, nesting, image dimensions and cache memory are bounded.

## Makepad instrumentation, without visible windows

The [headless receipt](render-comparison/evidence/rinx-headless-math/result.json)
records six passing checks:

1. The owned process had no visible window (all nine visibility samples were zero).
2. The complete Gist imported into 133 blocks with exact source retention.
3. The native reader rendered and scrolled through the math fixture.
4. The production Blitz preview rendered and scrolled through the fixture.
5. Changing `E=mc^2` to `E=mc^3` changed formula-region pixels in both previews.
6. Restarting Rinx preserved the exact edited TeX.

This uses Makepad's loopback instrumentation (`/snap`, `/click`, `/k`, `/t`,
`/m`, `/g`) with `MAKEPAD_HIDE_WINDOWS=1` and `MAKEPAD_NO_FOCUS=1`. It runs the
real Metal renderer with hidden windows, rather than the separate CPU headless
rasterizer. There are no desktop clicks, OS file dialogs or screen captures.
The test uses an isolated copy of a fixture profile and does not send messages.

[Native reader, top](render-comparison/evidence/rinx-headless-math/native-math-top.png)
· [Native reader, fractions and integral](render-comparison/evidence/rinx-headless-math/native-math-bottom.png)
· [Blitz preview](render-comparison/evidence/rinx-headless-math/blitz-math-bottom.png)
· [Edited formula](render-comparison/evidence/rinx-headless-math/native-math-edited.png)

Reproduce from the Rinx repository:

```sh
python3 tools/wechat-ux/live/native_article_math.py \
  --profile /path/to/robrix-ux-fixture \
  --binary /Users/ychen/Applications/Rinx.app/Contents/MacOS/rinx
```

The separate visible native file-picker/source regression passed nine journeys,
including HTML import and math edit/save/reopen. Its local receipt is
`target/article-source-regressions/c7b335079a5a4742a29dd734871211a7/result.json`.
The Rust suites passed 189 Rinx tests (two existing ignored tests), 21 article-core
unit tests, 10 host/storage tests and five article-makepad tests. Translation
validation found zero missing strings; `git diff --check` passed.

## Comparison and installed build

[Interactive comparison](render-comparison/index.html)
· [Independent math comparison](render-comparison/math/index.html)
· [Full math comparison PNG](render-comparison/math/pair.png)

The math reference is KaTeX 0.16.22 in WKWebView; the other side is the actual
Rinx native-math/Blitz pipeline. Their typography, operator limits and delimiter
sizes differ. Same-HTML mode instead shares Rinx's generated formula images,
so it isolates the HTML engines. All 20 section/mode pairs loaded successfully.
All ten Blitz section captures are complete. The full long document retains
Blitz's existing 8,192 CSS-pixel output limit; the native reader reaches its end.
Diagrams, TOC and icon extensions remain literal.

The installed `/Users/ychen/Applications/Rinx.app` is the signed binary used by
the headless test, SHA-256
`c990084ba9a25444d26081d9311d90a8cadf05fd5ef302f4fca963e2f09b7ed6`.
Its signature was verified. All five existing drafts and unapplied sources were
preserved byte-for-byte. Rinx was already closed and was left closed. The prior
app and draft backup are under `target/rinx-update/20260922-113021/`.
