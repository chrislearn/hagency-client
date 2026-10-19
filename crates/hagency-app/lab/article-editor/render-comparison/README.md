# Editor.md: WebView and Makepad + Blitz

This report records the earlier optional HTML/CSS comparison. Rinx's default
Markdown reader now uses Makepad only, including Mermaid and Editor.md sequence
diagrams. See [current native diagram coverage](../native-diagrams.md) and its
linked Makepad instrumentation captures.

Open [index.html](index.html) for ten selectable sections and two comparison
modes. The left pane contains real macOS WKWebView captures. The right contains
the RGBA output of the renderer pinned by Rinx; the table section is additionally
captured in the native Makepad `HtmlView` under `evidence/native/`.

- **Same imported HTML:** identical HTML bytes, Rinx Classic CSS, 440 CSS pixels,
  2× device pixels. Formula PNG bytes are also identical; this isolates rendering differences.
- **Browser Markdown:** CommonMark/GFM with raw HTML, image elements, original
  links and checkboxes plus KaTeX math on the left; Rinx import and native math
  on the right. Image bytes are
  captured locally and their hashes recorded. This measures both import and
  rendering differences, rather than attributing everything to Blitz.

Math is typeset in both modes. The [math comparison](math/index.html) uses
independent typesetters: KaTeX 0.16.22 in WebKit versus Makepad native math
rasterized to PNGs and laid out by Rinx’s production Blitz pipeline. The native
Rinx full reader uses the same math engine. The original TeX is preserved.
Images now render in both Rinx previews: the host fetches HTTPS PNG/JPEG/SVG
resources, normalizes them to bounded PNG bytes, and grants those bytes to the
renderer. SVG badges retain their natural size. Linked images retain their
HTTPS target. No network access is granted to Blitz itself.

The six tables have visible borders, headers and column alignment. JavaScript,
HTML and other declared code languages use Syntect highlighting. Native code
keeps newlines, indentation and Chinese glyphs; both renderers expand display
tabs without changing the source. GitHub emoji shortcodes, literal Unicode emoji,
and the sample's Font Awesome / Editor.md logo shortcuts render in previews.
On Apple platforms, the host rasterizes the installed emoji font because the
pinned Blitz backend does not paint its color glyphs.

The browser reference shares syntax highlighting and shortcode expansion with
Rinx; it uses browser-native SVG and emoji glyphs. Same-HTML mode shares all
normalized image bytes. The flowchart captures use the native Rust flow layout.
Sequence and TOC source in these historical captures predates the native reader
update. The current shared diagram adapter also supports Mermaid and sequence
rendering for this optional comparison. Arbitrary source CSS/scripts remain excluded.

The input is the 9,146-byte supplied Gist fixture, also pasted in the conversation.
Its BLAKE3 is `7088e019427ca6cf01ac86871312804c7ecd6359c6711cdf6e6abdd9e0667ad8`.
Sections are contiguous source slices with document reference definitions made
available to each parser. They contain the entire source in order. Original
source is in [source.md](source.md).

On 2026-09-22 all 20 WebView captures and all ten Blitz section captures
completed. The report verified loading all 20 pairs. All ten remote image
resources downloaded successfully. The full Blitz body measures 11,984 CSS
pixels but its default output clips at 8,192; section captures avoid that
vertical limit without changing production settings. Horizontal overflow is
left as rendered at the original viewport width. WebView's measured document
height has a minimum of its 700-pixel viewport.

Some visible differences in identical HTML: text weight and line spacing vary,
table rows accumulate a small vertical offset, and code source occupies
more height in Blitz. No alignment or image correction is applied. Screenshots
are evidence, not a numerical similarity score.

## Reproduce on macOS

From the Rinx root, with Swift and Pillow installed:

```sh
cargo build --locked --offline --features html_preview --example article_render_comparison --example article_comparison_native
target/debug/examples/article_render_comparison \
  lab/article-editor/render-comparison/source.md \
  lab/article-editor/render-comparison/article.css \
  lab/article-editor/render-comparison/evidence
swiftc tools/wechat-ux/live/article_comparison_webview.swift -o target/article-markdown-webview
# Supply local Node and an extracted official KaTeX 0.16.22 npm package:
python3 tools/wechat-ux/live/article_math_comparison.py --node /path/to/node --katex /path/to/katex/package
python3 tools/wechat-ux/live/article_render_comparison.py
swiftc tools/wechat-ux/live/article_comparison_report.swift -o target/article-markdown-report
target/article-markdown-report lab/article-editor/render-comparison
```

For the actual native Makepad window:

```sh
target/debug/examples/article_comparison_native \
  lab/article-editor/render-comparison/evidence/07-tables/rinx.html
```

`article.css` snapshots the Classic theme, normal text size and spacious
paragraphs from `src/article_app/preview.rs`. The comparison adds the same
shared table/code CSS from `article_makepad::content::CSS` that production uses. Renderer revision, sizes, image
hashes and capture details are recorded in [results.json](results.json).

The native math engine uses NewCMMath and supports a TeX subset. Its spacing,
operator limits and delimiter sizing differ from KaTeX. The Editor.md fixture's
redundant outer delimiters and escaped subscripts are normalized for preview
only; [math/result.json](math/result.json) records the adaptations. Earlier raw
KaTeX-on-Blitz diagnostics are retained under `math/raw-katex-diagnostic/`;
they bypass production admission and are not the Rinx math implementation.

The actual Rinx native reader and Blitz preview are checked with Makepad's
hidden Metal instrumentation. See [content-rendering.md](../content-rendering.md)
for the final installed-binary receipt, source persistence and full-document
scrolling checks.
