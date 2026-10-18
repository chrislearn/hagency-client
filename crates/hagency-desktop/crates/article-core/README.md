# article-core

Transport-independent article documents, editing, assets, scoped local storage,
and host interfaces. Robrix and OctoSense can both embed this library; neither
application is a dependency. No background service or network client is needed.

Implement `host::ArticleHost` with a trusted storage root, an opaque current
account identifier and a `SessionAuthority` owned by your login lifecycle.
Issue `ConsentGrant` only from trusted host code after consent. Invalidate the
authority on logout/account changes. Use `storage::LocalStore<Host, P, O>`;
`P` and `O` are your publication and outbox metadata, or `()` for a local editor.
The `ArticlePublisher` port lets a host retain its transport-specific operations
and receipts. It does not grant authority merely by being implemented.

Image import accepts host-selected bytes. Reading assets validates content
hashes. `assets::crop_cover` and `editing::EditHistory` are shared by hosts.
The optional `l0` feature provides the reviewed builtin field bindings from
`resources/app.card`; no arbitrary downloaded code is admitted.

```sh
cargo test --locked --manifest-path crates/article-core/Cargo.toml --features l0
```

The v2 model imports UTF-8 Markdown and HTML up to 512 KiB through
`markup::import_file(name, bytes)`. Simple Markdown becomes rich text; extended
structures remain editable Markdown source blocks with a rendered preview.
HTML remains an editable source block. Unchanged imports retain their exact
source, including after storage and reload. Body edits invalidate that source
snapshot so later exports cannot silently restore stale content.

Native readers should call `markdown_render::render(source, renderer)` once for
an entire document, then virtualize its returned sections. This resolves
reference links, duplicate heading IDs, TOC markers, footnotes and back-links
across editor blocks. The configured parser supports CommonMark 0.31.2 and GFM,
plus the explicit article extensions listed in
[native Markdown coverage](../../lab/article-editor/native-markdown.md).

Parsing, sanitization, source positions and `render::Renderer` are reexported
from Makepad’s `makepad-markdown` crate. This crate retains the article document
model, lossless import/export, resource collection and `render_editor` mapping
from whole-document fragments to editable blocks. It explicitly opts into
Editor.md compatibility for legacy emoji aliases.

The host `render::Renderer` supplies code, math, emoji and image presentation.
Source HTML is sanitized before those trusted substitutions. Parsing never
loads network resources or runs JavaScript. Original source stays editable;
legacy block-level HTML methods remain available for compatibility.

The original component extraction is described in
[ADR 0003](../../docs/adr/0003-shared-article-components.md).

`markdown::inspect(source)` reports features needing source blocks, and
extensions that remain literal, with one-based source lines. Inspection never rewrites
the source and does not replace document validation. From the application root:

```sh
cargo build --locked --offline -j 1 -p article-core --example markdown_compatibility
target/debug/examples/markdown_compatibility path/to/article.md
```

The JSON report includes the source hash, imported block count, exact source
round-trip result and any import error. The example exits unsuccessfully on an
import error; successful parsing does not imply support for every extension.
