# Source → Apply rendering in the editor

The installed desktop build reproduced the reported failure with the complete
9,146-byte Editor.md fixture: Source → Apply returned to raw Markdown blocks,
including YAML and heading syntax. Earlier native tests primarily checked
**Full preview**, missing this editing path.

Advanced Markdown/HTML blocks now display native rendered content inside the
editing canvas. **Edit source** opens that block's source input; **Render block**
returns to its formatted display. Select All in this input affects that block.
Ordinary rich paragraphs retain cross-paragraph selection and formatting.
YAML is retained as metadata, with an **Edit metadata** control.

The editor serializes current blocks with source-line positions and parses them
together. Rendered fragments map back to editable blocks without isolating
TOCs, reference definitions or heading anchors. This rendering projection does
not replace the retained original source. External images load in the editing
page using the same bounded loader as Full preview.

The shared renderer also keeps `$$…$$` inline when it occurs inside a sentence.
Standalone formulas, including ones within lists and quotes, remain display
math. This applies to both the editing canvas and Full preview.

This is the existing native rendering feature set, not arbitrary HTML/CSS
WYSIWYG support. The [documented rendering limits](native-markdown.md#coverage),
including literal print-page markers, still apply.

## Verification

```sh
cargo test --locked --manifest-path crates/article-core/Cargo.toml
cargo test --locked --lib article_app::
python3 tools/wechat-ux/live/native_article_markdown.py \
  --editor-canvas --profile /path/to/test-profile --binary /path/to/rinx
python3 tools/wechat-ux/live/native_article_selection.py \
  --binary target/article-selection-build/debug/examples/standalone
```

The canvas runner uses a private copy of the signed-in fixture, hidden windows
and Makepad instrumentation over the real Metal backend. It applies the exact
sample through Source, scrolls the editing canvas through End, records native
widget images, checks inline math and both diagram types, edits one block, and
checks its saved source and rendering after a process restart. No messages or
articles are sent or published.

Core regressions cover editor/reader equivalence for the complete fixture,
whole-document reference/TOC context, metadata, source preservation, and inline
versus display math. The existing official parser corpus remains 652 CommonMark
examples plus 24 GFM extension examples.

## Signed build result · 2026-09-22

The [evidence gallery and receipts](editor-canvas-evidence/index.html) record
16 passing native canvas checks and seven passing native selection checks.
The final canvas run completed with no unexpected runtime errors and all 15
window-visibility samples equal to zero. The source fixture remained unchanged.
JavaScript and HTML checks also verified colored pixels in their code widgets.

Core tests passed (23 unit, 10 host isolation, nine rendering/conformance),
along with seven Rinx article integration tests and the translation audit
(996 catalog entries, no missing translations).

The tested signed binary, SHA-256
`0580f55a222fb543243645db065e6654c10e9008dcde0b151a0b9a5a70a754c0`,
is installed at `/Users/ychen/Applications/Rinx.app`.
The existing article library was backed up and its hash remained unchanged.
The installer wrote no profile files. A separate development process was using
the normal profile, whose login marker was already absent; that process was
left running and the installed app was not launched concurrently.
