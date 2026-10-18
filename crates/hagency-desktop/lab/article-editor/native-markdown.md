# Native Makepad Markdown

Rinx's default build renders Markdown using Makepad. It does not compile or
instantiate Blitz or a WebView for Markdown. The optional `html_preview` feature
is retained for HTML/CSS experiments; Markdown always uses the native reader.

Source → Apply now uses that renderer in the editing canvas as well. Advanced
blocks display their formatted content and expose **Edit source** to change
the block. Simple paragraphs retain native rich text editing. See
[editor canvas validation](editor-canvas.md) for the regression and evidence.

## Implementation

`article-core::markdown_render` parses the entire document with Comrak 0.55,
resolves reference links, heading IDs and footnotes, sanitizes embedded HTML,
and returns sections for the virtualized reader. Parsing each editor block
independently could not resolve all of those relationships.

`article-makepad` draws those sections using Makepad's native `Html`/`TextFlow`,
table layout and custom widgets. The intermediate HTML is a structure format,
not a browser surface. The native adapter preserves unfamiliar containers'
contents and styled link labels. Code uses Syntect, formulas use Makepad's LaTeX
engine, and diagrams use Rust parsing/layout plus the bounded native image path.
Mermaid uses pinned `mermaid-rs-renderer` 0.3.1. Editor.md sequence diagrams have
a separate grammar adapter; both feed the same layout engine. Diagram SVGs are
rasterized with resvg and displayed by Makepad, without a browser runtime.
These are general parsers and renderers; the supplied Editor.md sample is a
regression fixture, not a set of content-specific rendering substitutions.

## Coverage

| Content | Native behavior |
| --- | --- |
| CommonMark | Headings, paragraphs, hard/soft breaks, emphasis, lists and nesting, quotes, rules, code, escapes, entities, reference links and images |
| GFM | Tables with alignment, task lists, strikethrough, URL/email autolinks |
| Article extensions | Footnotes and back-links, duplicate-safe heading anchors, `[TOC]` / `[TOCM]`, alerts, description lists, YAML front matter |
| Formulas | Inline/display TeX and `math` / `katex` / `latex` fences, within the native math engine's supported TeX subset |
| Code | Language-aware syntax highlighting; unknown languages remain readable code |
| Images | PNG/JPEG and rasterized SVG; HTTP(S) public resources and imported relative assets |
| Emoji | Unicode and recognized shortcodes; native color emoji on Apple platforms |
| Flow diagrams | Existing `flow` / `flowchart` syntax and Chinese labels |
| Mermaid | `mermaid` fences rendered by the Rust engine; tested flowchart, sequence, class, state, ER, pie, Gantt, mindmap, git graph and timeline families |
| Sequence diagrams | Editor.md `seq` / `sequence`: participants and aliases, filled/open arrows, solid/dashed lines, self-messages, notes, titles and escaped line breaks |
| Navigation | Heading/TOC/footnote jumps; explicit relative-file links inside the chosen import folder |
| Source | Original Markdown retained; visual conversion used only when its rendered semantics match |
| Long articles | Up to 512 KiB of source, 8,192 editor blocks and 128 images; native virtualized scrolling |

This is CommonMark/GFM support plus the listed extensions. Arbitrary browser
CSS/JavaScript, animated images, and unrestricted LaTeX packages are outside
this renderer. Unsupported or malformed diagrams show a diagnostic and retain
their source. `[========]` stays literal because the reader has
no print-pagination model. SVGs are inert pixels, not interactive web content.
Image requests retain existing per-resource and total-memory limits.

See [native diagram coverage and limits](native-diagrams.md). The Rust Mermaid
engine does not promise full mermaid-js syntax or pixel parity. Sequence
messages preserve the different meanings of arrows in Mermaid and Editor.md.

Relative image files are normalized and copied into the account's asset store.
The original URL maps to an asset ID; absolute source-file locations remain
local library metadata and are not published. Relative-file links require the
source folder to remain available. Missing images remain visible as labeled
links. Cross-widget selection includes code, formula source, emoji, and aligned
table cells; reader Select All copies the full document, including offscreen
sections. Native accessibility and browser-identical typography are not claimed
by these tests.

## Verification

- 652 official CommonMark 0.31.2 parser examples and 24 GFM extension examples.
  The GFM 0.29 inherited core cases are not counted again. See
  [fixture provenance](../../crates/article-core/tests/fixtures/README.md).
- Core regressions cover global references, duplicate Chinese headings,
  footnotes, alerts, breaks, source preservation after unrelated edits, large
  documents, and asset bindings.
- Native widget tests cover styled links, container adaptation, code colors,
  emoji, formulas, SVG normalization and flow diagrams.
- `native_article_markdown.py` drives Rinx through Makepad instrumentation,
  with hidden windows and the real Metal backend. It checks pixels/OCR,
  navigation, the complete user fixture, long scrolling, and process restart.
- `native_markdown_copy.py` checks the actual native `TextCopy` event against
  the rendered widget text, without touching the system clipboard.

```sh
cargo test --manifest-path crates/article-core/Cargo.toml
cargo test --manifest-path crates/article-makepad/Cargo.toml --lib
cargo test --lib --no-default-features
cargo build --bin rinx --example article_native_markdown --no-default-features
python3 tools/wechat-ux/live/native_markdown_copy.py
python3 tools/wechat-ux/live/native_article_markdown.py --profile /path/to/test-profile
```

The Rinx runner copies a previously signed-in `@robrix_ux_` fixture. It never
publishes or sends messages and does not modify the source profile.

## Installed validation receipt · 2026-09-22

The signed app at `/Users/ychen/Applications/Rinx.app`, updated with Mermaid
and Editor.md sequence diagrams, passed 28 hidden-window Rinx checks on its
exact binary. A separate native widget test passed actual selection/copy,
including both diagram source formats. This update passed 21 native renderer
tests and 152 Rinx tests (one existing test ignored). The earlier 676 official
parser-example baseline above is retained. The optional HTML comparison
compiles, and the translation audit has no missing keys.

[Open the diagram release screenshots and receipts](native-diagram-evidence/index.html).
That tested release's SHA-256 is
`79e1b884b1a7586e2e9e9e48743d096e51d9222f40a2db101b6a2f6e3dff2b62`.

Later builds include the [desktop Contacts and editor
entries](../wechat-ux/DESKTOP-NAVIGATION.md) and the
[Source → Apply editor canvas fix](editor-canvas.md), with separate installation
and native interaction receipts.
The signed binary contains no Blitz DOM/HTML/renderer symbols. Five original
drafts and their unapplied source were preserved byte-for-byte; the app was
left closed. The rollback copy and private installation manifest are in
`target/rinx-update/20260922-141244-diagrams/` and `target/rinx-diagram-update.json`.
See the [diagram validation notes](native-diagrams.md#validation) for the initial
GPU-watchdog interruption and successful isolated rerun.
