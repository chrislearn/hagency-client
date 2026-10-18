# article-makepad

Reusable native article widgets and presentation. Depends on `article-core` and
Makepad; it has no Matrix, Robrix or OctoSense application dependency.

Markdown parsing and native rendering now live in `makepad-markdown` and
`makepad-markdown-widgets` in the Makepad repository. This crate reexports the
renderers and registers legacy `Article*` aliases. Its own implementation retains
article rich-text editing, cross-block selection, document themes, and the
block-to-preview adapter. Rinx owns resource fetching, drafts, import/export,
account lifecycle, and Matrix publication. Generic consumers can use
Makepad’s `MarkdownView` directly without depending on either article crate.

Rinx explicitly enables `full` plus `editormd`. Other applications can select
`syntax-highlighting`, `math`, `emoji`, `svg`, and `diagrams` independently.
Disabling a renderer preserves a readable source fallback.

Call `article_makepad::script_mod(vm)` after registering Makepad widgets, then
use `mod.widgets.ArticleRichInput`. `presentation::style_input` and `style_html`
apply the same document styles in every host. Apple hosts can call
`apple_fonts::install(vm)` after theme selection and before widget registration
to select installed PingFang Regular/Semibold with bundled fallback.

The rich input retains Makepad's selection, clipboard and IME implementation.
Derived code is attributed in `src/rich_input.rs` and `MAKEPAD-LICENSE.txt`.
Hosts displaying multiple paragraph inputs should use
`body_selection::ArticleSelection` with their document and `EditHistory`.
Dispatch to its `handle_event` before the list, mirror the selection with
`apply_to_input` before drawing each paragraph, and call `after_draw` afterward
to restore the caret following a document edit. See `examples/standalone.rs`.
This keeps Select All, cross-paragraph dragging, replacement and clipboard
ranges independent of the list's visible/recycled widgets.
Consumers must use one compatible Makepad source graph; this repository's lock
files select the compatibility revision pinned in Cargo.toml. The Makepad git
patch keeps optional HTML preview on that same SDK graph.

Run the local-only example from the repository root:

```sh
cargo build --locked --manifest-path crates/article-makepad/Cargo.toml --example standalone
crates/article-makepad/target/debug/examples/standalone --data-dir=/tmp/article-example
```

It exercises native editing, selected-text formatting, themes, preview, saving
and reopening without either host application or a server. It is an integration
example, not the full Robrix publication workflow or an OctoSense package.

Exercise selection through Makepad's native input bridge with an isolated profile:

```sh
python3 tools/wechat-ux/live/native_article_selection.py \
  --binary crates/article-makepad/target/debug/examples/standalone
```

For full-document Markdown, use `article_core::markdown_render::render` with
`content::NativeRenderer`, then pass each section through `content::native_html`
to Makepad's `Html`. Register `rmath`, `rdiagram`, `rimage`, `remoji`, `rcode`, and
`rcell` with the corresponding Article widgets (see the standalone example).
Set `selectable: true` to include custom widgets' text in native selection and
copy. The host owns image loading and installs `DrawingImages` during drawing.
There is no Blitz or WebView dependency. Rinx's default build follows this path;
its HTML/CSS comparison is optional. See
[native coverage and tests](../../lab/article-editor/native-markdown.md).

Math previews use the pinned `makepad-latex-math` parser/layout and bundled
NewCMMath font, with glyph outlines and fraction/radical rules rasterized into
transparent 2× PNGs. Register `rmath := ArticleMath {}` inside the native `Html`
widget and feed it `math_view::native_block_html(document, block)`. For Blitz,
use `Document::block_html_with_math` with `math::html`, granting only the generated
PNG bytes through the host's ResourceMap. Original TeX stays editable. No remote
font, JavaScript, SVG admission or browser is needed by either Rinx preview.

Supported input includes `$…$`, `$$…$$`, and math/katex/latex fences. Editor.md's
redundant outer `\( \)` and escaped fence underscores are normalized only for
rendering. Leading displaystyle/textstyle are interpreted, and paired explicit
delimiter sizes use content-sized brackets. This is the pinned native parser's
TeX subset, not full LaTeX. Input length, nesting, image size and cache bytes are
bounded. The font comes from Makepad’s existing `libs/latex_math/fonts/` resources.


Flowchart previews use `content::NativeRenderer` / `content::HtmlRenderer` and
`rdiagram := ArticleDiagram {}` in the native `Html` widget. The static renderer
accepts flow/flowchart fences, start/end/operation/condition/input/output/
inputoutput/subroutine nodes, chained edges, yes/no branches, joins and cycles.
It lays out top-down automatically; only a left hint on a return edge changes
routing. Escaped `\n` splits labels. Source links, flow-state styling, parallel
nodes are not implemented for that legacy flowchart syntax. Unsupported/malformed input
shows an explanation plus the unchanged code. Limits: 16 KB, 24 nodes, 48 edges,
six label lines, 2048 CSS pixels per dimension, 32 cached images / 16 MB.
The browser comparison shares this layout but renders its SVG in WebKit;
Rinx rasterizes it with resvg and grants only PNG pixels to Blitz.

`mermaid` and Editor.md `seq` / `sequence` fences now use the native diagram
widget as well. Mermaid parsing/layout comes from pinned `mermaid-rs-renderer`
0.3.1; the Editor.md grammar adapter preserves its different alias and arrow
semantics. SVG output becomes inert pixels through the existing resvg path.
See [diagram coverage, tests and limits](../../lab/article-editor/native-diagrams.md).
