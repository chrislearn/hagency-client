# Markdown and HTML import validation

This records the initial import implementation. Current rendering coverage is
in [Native Makepad Markdown](native-markdown.md), and the Source → Apply canvas
fix is documented in [editor canvas validation](editor-canvas.md).

Validated on 2026-09-22 against the [exact supplied Gist revision](https://gist.githubusercontent.com/iHaPBoy/a7d1c4f9ccf180c43b696a0a7c9d5381/raw/0bc43a0e5cc62614bae559396abb57a07c9a277c/markdown%25E6%25A0%25BC%25E5%25BC%258F%25E6%25B5%258B%25E8%25AF%2595.md).
The fixture contains 9,146 UTF-8 bytes. Its BLAKE3 is
`7088e019427ca6cf01ac86871312804c7ecd6359c6711cdf6e6abdd9e0667ad8`.

## Result

The previous converter rejected the first HTML tag on line 9. The new importer
accepts the complete fixture as 133 blocks and preserves its source exactly.
Simple structures become rich text. Extended structures become editable source
blocks whose previews are generated with CommonMark/GFM and inert HTML.
Changing body content invalidates the original source snapshot; changing
metadata alone preserves the source spelling. Reference definitions remain
available when rendering or reparsing individual Markdown blocks.

| Feature | Current behavior |
| --- | --- |
| Headings H1–H6, inline/fenced/indented code | Rendered in preview; declared code languages highlighted, indentation and Chinese preserved |
| Nested lists and quotes, custom list starts | Structure retained in source blocks and preview |
| GFM tables, task lists, strikethrough | Rendered; tables include borders and alignment, task markers are static glyphs |
| YAML front matter | Source retained, omitted from visible body |
| Raw HTML and imported HTML files | Source retained; preview uses allowed article tags |
| CSS, scripts and event attributes | Retained in original source, omitted from preview |
| External images | HTTPS PNG/JPEG/SVG fetched by the host and rendered from normalized PNG bytes; linked images retain their targets |
| HTTPS links | Clickable; other URL schemes remain inert |
| Math (`$`, `$$`, math/katex/latex fences) | Typeset by native Makepad math in full preview and Blitz; TeX source retained |
| Emoji | GitHub shortcodes, Unicode emoji and the sample’s logo/star/gear shortcuts render; code literals remain unchanged |
| Flowcharts (`flow` / `flowchart`) | Nodes, branches, joins and return arrows rendered in both previews; original source retained |
| Editor.md TOC, sequence diagrams, page breaks | Source/literal text; dedicated extension renderers are not implemented |

This supports importing and editing source; it does not provide arbitrary
HTML/CSS WYSIWYG editing or full Editor.md rendering parity.

## File import

In Rinx, open Discover → Article editor → Import Markdown or HTML file.
The native picker accepts `.md`, `.markdown`, `.txt`, `.html` and `.htm`.
Files must be UTF-8 and at most 24,000 bytes. Each import creates a separate
local draft and opens its preview. The Source view edits original Markdown or
HTML. HTML imports keep their format when source edits are applied.

## Verification

The actual macOS picker imported the supplied Markdown fixture and an HTML
fixture containing Chinese, bold text, code and a table. Native checks covered
scrolling to the Markdown fixture's final `End` section, opening the bounded
HTML/CSS preview, applying source, and HTML editing/save/reopen. The experimental
HTML/CSS preview reports its existing length limit; the native full preview
continues through the complete document.

Eight native import/source journeys passed. The latest run is recorded in
`target/article-source-regressions/776f67312d704d89bb0a6ca93206afb1/result.json`.
Those import results precede the math follow-up. Current math validation and
native screenshots are recorded in [math-rendering.md](math-rendering.md).
The image/table/code/emoji follow-up is in [content-rendering.md](content-rendering.md). Exact fixture/report files remain in ignored local evidence
under `target/markdown-compatibility/`.

To reproduce the pure import result from the Rinx root:

```sh
cargo build --locked --offline -p article-core --example markdown_compatibility
target/debug/examples/markdown_compatibility path/to/article.md
```

The JSON report includes the source hash, block count, exact-source round-trip
result and any import error. The example exits nonzero on an import error.
Native reproduction is in [selection-source-validation.md](selection-source-validation.md).

## Source storage

Unapplied source is saved per article, separately from the current document.
Failed imports retain both forms. Closing and reopening the editor, restarting
Rinx and ordinary visual saves preserve the source. Successful source imports
commit the new body and source-draft removal together before returning to
editing. Pending saves flush on background, window close and shutdown while
the account's consent remains valid.
