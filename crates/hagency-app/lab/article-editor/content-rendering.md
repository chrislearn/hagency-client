# Article content rendering

Rinx’s native Full preview, its Makepad + Blitz preview, and the WebView
comparison now render the supplied Editor.md fixture’s images, tables, code
and emoji. Markdown/HTML source remains unchanged by preview rendering.

- PNG/JPEG images and SVG badges are fetched over HTTPS by the host, decoded
  with size limits, and passed to the renderer as PNG bytes. Linked images
  retain their target. Unavailable resources retain an alt-text link.
- Tables have visible borders and headers, with left/center/right alignment.
  The native reader isolates cell alignment to avoid the pinned TextFlow’s
  duplicate alignment offset.
- Code fences use their declared language for highlighting. Native code uses
  explicit runs so newlines, indentation, token spacing and Chinese glyphs
  survive. Display tabs expand to spaces; indented code keeps its first line.
- GitHub shortcodes and Unicode emoji render outside code/math. The sample’s
  star, gear and Editor.md logo shortcuts are also supported. Apple color
  emoji are rasterized with the installed OS font and shared as inert pixels.
- The existing nine math occurrences still use the native math engine. TeX
  edits change both previews and survive restart.

The full fixture is 9,146 bytes, imports as 133 blocks, and retains BLAKE3
`7088e019427ca6cf01ac86871312804c7ecd6359c6711cdf6e6abdd9e0667ad8`.
The native reader reaches its final End heading. The whole Blitz document is
12,290 CSS pixels and reaches its existing 8,192-pixel height cap; all ten
comparison sections are captured completely. Flowcharts now render in both
previews; TOC and sequence diagrams remain literal source. This is not arbitrary source CSS or script execution.

The flowchart follow-up and its current signed-app verification are documented
in [Flowchart previews](flowchart-rendering.md).

## Actual renders

- [WebView vs Makepad + Blitz, ten sections](render-comparison/index.html)
- [Rinx native Full preview vs its Blitz preview](render-comparison/evidence/rinx-headless-content/index.html)
- [Signed application test receipt](render-comparison/evidence/rinx-headless-content/result.json)

## Image/table/code/emoji baseline validation

Makepad loopback instrumentation drives a real hidden Metal window and captures
its framebuffer. No desktop clicks or screenshots are used. Each test copies
the signed-in fixture into an isolated profile; the original draft files are
hashed before and after installing the update.

The checks cover PNG, raw HTML SVG, linked JPEG, table border pixels and OCR
of the right-aligned price column, code color pixels and Chinese glyph OCR,
emoji color pixels, exact source after preview, the complete fixture through
all five feature families to End, nine math occurrences, edited TeX pixels,
and restart persistence. The receipt records every visible-window sample and
the exact signed executable hash. The comparison separately verifies all 20
WebView pairs, ten complete Blitz sections, and zero denied resources.

Core tests: 23 unit + 10 host isolation/storage tests. Native component tests:
8. Rinx library tests: 190 passed, 2 ignored. The i18n check has no missing keys.

```sh
python3 tools/wechat-ux/live/native_article_math.py --content-checks \
  --binary /path/to/Rinx.app/Contents/MacOS/rinx \
  --profile /path/to/isolated-fixture
```

Baseline signed executable SHA-256: `ed49ae24797dfaf343540ab5251e96fa53956babf0dcecbe421030b77a50bff0`.
All 12 checks passed; 26 visible-window samples were zero. The existing 5 drafts
and unapplied source files were preserved byte-for-byte.
