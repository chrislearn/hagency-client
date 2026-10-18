# Native diagrams in Rinx

`mermaid`, `seq`, `sequence`, `flow` and `flowchart` Markdown fences render
visually in the native Makepad article reader. Original Markdown remains
editable and is saved unchanged. Selecting a diagram copies its source text.
No JavaScript, WebView or Blitz is required.
The optional HTML comparison renderer uses the same generated diagram pixels.

## Pipeline

- Mermaid: pinned [`mermaid-rs-renderer = 0.3.1`](https://github.com/1jehuang/mermaid-rs-renderer), library-only features.
- Editor.md sequence syntax: grammar adapter builds the same sequence graph IR.
- Existing flowchart.js syntax: existing bounded Rust parser/layout.
- SVG output: resvg with system fonts and external-resource resolution disabled.
- Presentation: `ArticleDiagram` uses Makepad's image widget, scales to the
  available width and registers original source with native text selection.
- Content hashes cache successful images and diagnostics. An edit changes the
  cache key and regenerates the result. Source is never replaced by a bitmap.

The sequence adapter follows the [js-sequence-diagrams grammar](https://github.com/bramp/js-sequence-diagrams/blob/master/src/grammar.jison).
It handles quoted participant names, declarations/aliases, implicit participants,
notes left/right/over one or two participants, titles, comments and `\n` breaks.
Alias order is `participant Display Name as ID` for Editor.md and
`participant ID as Display Name` for Mermaid.

The pinned Rust engine drops sequence arrowhead distinctions. The adapter
restores arrow semantics in the graph, and supplies open SVG markers by edge
index. This applies to arbitrary parsed diagrams, not specific sample strings.

| Arrow | Editor.md `seq` | Mermaid sequence |
| --- | --- | --- |
| `->` / `-->` | Filled arrowhead, solid/dashed | Line without arrowhead, solid/dashed |
| `->>` / `-->>` | Open arrowhead, solid/dashed | Filled arrowhead, solid/dashed |
| `-)` / `--)` | Not this dialect | Open arrowhead, solid/dashed |
| `-x` / `--x` | Not this dialect | Cross ending, solid/dashed |
| `<<->>` / `<<-->>` | Not this dialect | Bidirectional filled arrowheads |

Mermaid sequences also use the engine's activation bars, self-messages, notes,
participant groups, numbering, loops, alternatives and parallel/critical frames.
Unknown sequence statements produce a diagnostic instead of being silently
discarded. Malformed diagrams retain their complete source below the error.

## Verified coverage and practical limits

Rust tests exercise the supplied Andrew/China example, Chinese aliases, notes,
line breaks, source edits and cache invalidation, actual raster output and both
article host adapters. Diagram family fixtures cover flowcharts, sequences,
class/state/ER diagrams, pie charts, Gantt, mindmaps, git graphs and timelines.
This is a compatibility test set, not a claim that every mermaid-js feature or
visual detail is identical. The underlying engine exposes additional families.

Inputs are bounded to 16 KiB per fence. Mermaid/sequence layouts allow 64 nodes,
128 messages/connections, 32 groups, 64 notes and 128 activation events.
Existing flowcharts retain 24 nodes and 48 connections. Images are limited to
2048 logical pixels per dimension and 256 KiB of generated SVG; cached PNGs
have a 16 MiB / 32-entry limit. Oversized diagrams show a diagnostic.
Rendering uses a fixed light diagram theme. Mermaid init/theme configuration,
interactive links/callbacks, newer half-arrows, actor creation/destruction,
browser typography parity and a diagram-specific zoom UI are not implemented.

## Reproduce

```sh
cargo test --manifest-path crates/article-makepad/Cargo.toml --lib
cargo build --bin rinx --example article_native_markdown --no-default-features
python3 tools/wechat-ux/live/native_markdown_copy.py
python3 tools/wechat-ux/live/native_article_markdown.py --diagrams-only --profile /path/to/test-profile
```

The app test uses an isolated copy of a signed-in test fixture and hidden Metal
windows. It drives Makepad instrumentation, checks OCR/pixels, edits the sequence
source, checks exact source preservation, then restarts at a wider window size.
The copy test sends the real native `TextCopy` event without using the system
clipboard and verifies that both sequence and Mermaid source are included.

## Validation

The diagram release passed **28 hidden-window Makepad checks**, covering all
existing native Markdown cases plus the exact Editor.md sequence sample,
quoted Chinese participant aliases, Mermaid flowcharts, nested sequence
frames, invalid input, source edits, exact source preservation, and a restart
at 920×820 after the 430×820 checks. The full user fixture, remote image,
navigation and long-document checks also passed. All window-visibility samples
were zero. The native copy fixture separately verified both diagram sources.

The Rust suites passed 21 renderer tests and 152 Rinx tests, with one existing
ignored test. The optional HTML comparison compiles and both article host
adapters have a source-preservation/rendering test. The default dependency
graph and signed binary contain no Blitz renderer.

An earlier full-suite attempt hit Makepad's five-second GPU watchdog during a
capture of the code-highlighting page, before the diagram tests. The complete
isolated rerun passed with the watchdog unchanged. Both results are retained;
the cause of that single GPU timeout was not established.

[Screenshots and passed test receipts](native-diagram-evidence/index.html) ·
[Initial interrupted-run receipt](native-diagram-evidence/gpu-timeout-result.json).
The tested diagram release SHA-256 is
`79e1b884b1a7586e2e9e9e48743d096e51d9222f40a2db101b6a2f6e3dff2b62`.

The subsequent [desktop navigation update](../wechat-ux/DESKTOP-NAVIGATION.md)
adds Contacts and direct editor access. Its installed build and 14 native
interaction checks have separate receipts.
Five original drafts and their unapplied source remain byte-for-byte unchanged.
