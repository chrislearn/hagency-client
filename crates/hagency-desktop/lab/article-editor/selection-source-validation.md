# Selection and Markdown source validation

Validated on macOS on 2026-09-22 with native Makepad windows. File imports also
exercise the visible macOS file picker through accessibility input. Source
storage tests use a copied `@robrix_ux_` fixture profile with consistent SQLite
backups; they restore offline and do not publish messages. This run does not
verify Matrix delivery or physical mobile devices.

- Chat: nine native checks cover partial/reversed selection, Chinese and emoji,
  multiline selection, rich text, link drags versus clicks, clipboard contents,
  and the selected-text context menu.
- Article body: seven native checks cover select-all with replacement/undo,
  bidirectional paragraph selection, formatting, caret collapse, Shift-click,
  80 virtualized paragraphs, edge scrolling, and the empty-body placeholder.
- File import and source: eight native journeys cover Markdown/HTML file pickers,
  preview, HTML editing and reopening, localized line diagnostics,
  autosave, rejection without losing either editing form, close/reopen,
  per-article isolation, restart, and successful import followed immediately by
  an image-library reload. Import preserves document identity and metadata.
- Rust: 188 application tests passed, with two pre-existing backend-dependent
  tests ignored; article-core passed 19 unit and 11 host/storage tests, and
  article-makepad passed three tests.
- Translation catalog: 982 entries, 992 call sites, no missing entries.

Extended Markdown structures now use editable source blocks with rendered
previews. The supplied 9,146-byte Gist imports as 133 blocks, preserves its exact
source, scrolls through its final section in the native reader, and opens the
bounded HTML/CSS preview. HTML file import preserves the original source and
renders inert headings, formatting, code and tables. CSS/JavaScript and
Editor.md-specific math/diagram rendering remain outside this importer.

The latest import run is recorded locally in
`target/article-source-regressions/776f67312d704d89bb0a6ca93206afb1/result.json`.
Its tested binary SHA-256 is
`9b01671f27060a411fe2047f5e2d99bcf146ddff1761fb07f50244ca8fd60ab9`.

## Reproduction

From the Rinx root:

```sh
cargo build --locked --offline --features agent_chat --bin rinx --example chat_text_selection
cargo test --locked --offline --features agent_chat --lib
cargo test --locked --offline -p article-core -p article-makepad
cargo build --locked --offline --manifest-path crates/article-makepad/Cargo.toml \
  --example standalone --target-dir target/article-selection-build
python3 tools/wechat-ux/live/native_chat_selection.py
python3 tools/wechat-ux/live/native_article_selection.py \
  --binary target/article-selection-build/debug/examples/standalone
python3 tools/wechat-ux/live/native_article_source.py \
  --profile /path/to/test-fixture/profile --markdown-file /path/to/fixture.md
python3 tools/wechat-ux/check_i18n.py
```

The scripts put their result JSON, screenshots and input traces in unique run
directories under `target/chat-selection-regressions`,
`target/article-selection-regressions`, and `target/article-source-regressions`.
The source runner refuses non-fixture account IDs and edits only its new copy.
Private profiles and raw runtime logs stay in ignored local evidence.
