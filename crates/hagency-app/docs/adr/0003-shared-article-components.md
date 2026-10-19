# ADR 0003: Article components independent of their host

Status: accepted, implementation in `wechat-ui`. Renderer ownership is superseded
by [ADR 0004](0004-independent-makepad-html.md).

## Context

Robrix must remain an independently installed Matrix client. Sharing its article
editor with OctoSense must not introduce a dependency on the OctoSense shell,
ROM, account service, or a companion process. Previously article documents,
native rich input, image processing, storage and Matrix operations lived in one
application module. Even document IDs and local storage were tied to Matrix.

## Decision

Split reusable Rust libraries from host orchestration:

- `article-core`: versioned documents, themes, formatting, document history,
  normalized images/covers, optional admitted L0 bindings, account-scoped local
  storage, consent and host/publication contracts. No Matrix, Makepad, Robrix or
  OctoSense dependency. Octoscript L0 is an optional static library dependency.
- `article-makepad`: native rich text input, layout, shared editor/reader styling
  and optional Apple PingFang setup. No application or Matrix dependency.
- `apps/article-editor/native`: Robrix-specific screens, navigation, localization, image
  picker, consent UI, Matrix wire format, target selection, E2EE media,
  publication retries, replacement events and withdrawal. Robrix supplies
  `RobrixArticleHost` and `RobrixPublisher` to the shared code.
- `makepad-html`: separately validated, optional HTML/CSS preview renderer.
  Rendering arbitrary HTML does not make the v2 structured editor an arbitrary
  HTML WYSIWYG editor. The document schema is not expanded by this extraction.

Robrix exposes the renderer through the optional `article_blitz` Cargo feature.
The adapter generates HTML from a validated document, builds a fresh resource
map from grant-checked assets, renders on a worker and rechecks the grant,
instance and preview request before displaying a native Makepad texture. The
editor, full native reader and publication path remain available. A clipped
bitmap is explicitly identified and the user can return to the full reader.
The renderer has no WebView, JavaScript engine, network transport or document-
controlled file access. It is not a sandbox for arbitrary hostile CSS.

Each application links the components into its own executable. A reusable code
library is not a running service or an OctoSense installation requirement.

## Authority and storage

`ArticleHost` supplies an opaque current account, a trusted storage root and a
host-owned `SessionAuthority`. A grant is bound to its issuing authority,
account, login epoch, instance, expiration and explicitly granted capabilities.
It cannot be serialized into a shared card. Hosts invalidate epochs on logout
or account replacement. The shared store checks authorization before access and
again before committing edits. Import accepts bytes selected by the host, not
document-controlled filesystem paths. The Robrix adapter still performs Matrix
membership/permission checks and rechecks login state around asynchronous work.

Opening a received article issues a read-publication grant; it does not grant
draft access or publishing. Opening the editor retains the existing consent
screen. Publication remains a host operation; the shared library never sees a
Matrix client, access token, encryption key or password. Room/document-specific
publication confirmation and Matrix validation remain in the Robrix adapter;
the shared capability enum is not a complete generic app-manifest permission
system.

Storage keeps the existing `mini-apps/<account-hash>/org.octosense.article-editor`
directory and schema 2. Generic `Library<P, O>` preserves host-defined
publication/outbox records without interpreting them. Existing IDs, assets,
transactions and revisions are unchanged. New local IDs use random bytes and
no longer require Ruma. Unsupported legacy Markdown remains preserved.

## Validation and consequences

The standalone Makepad example is a second, local-only host. It exercises the
same native input, document model, styles, fonts and store without linking
Robrix, Matrix or OctoSense. Core tests cover account/issuer isolation, epochs,
revocation/expiry, capability denial, pre-commit invalidation, image integrity,
legacy migration and host publication metadata preservation.

Robrix's complete native publication lifecycle is tested separately with
disposable Palpo accounts. See `lab/article-components/` for the actual executed
checks; the original article-editor-v2 evidence remains a historical record.

This does not package an OctoSense app yet. Its host still needs an AppModule,
account/picker/publisher adapters and validation against its pinned Makepad
runtime. Robrix's own consent/library/publication screens remain host UI; only
shared primitives are claimed portable by this change.
