# From generated atlas to native Robrix

The exact `gpt-image-2` source and twelve measured crops passed upstream image-to-appcard-flow `intake,prepare`. The adapter currently accepts only a 406×776 artboard; source crops are reversibly letterboxed to its 812×1552 backing references. Actual Robrix captures use 430×820 and 1440×960 windows.

Each scene has `robrix-mapping.json` linking source intent to real Makepad widget IDs, state and actions. This is an explicit Robrix host adapter, not a claim that the stock adapter compiled a rich editor or Matrix backend. Generic observe/map/semantic/compile/bundle/WASM stages are unrun. The generated atlas is never mounted as clickable UI.

Runtime modules:
- `document.rs`: bounded structured document, Unicode marks, Markdown interchange and safe HTML.
- `rich_input.rs` / `rich_layout.rs`: pinned native input implementation and shared mixed-face layout for display, selection and caret.
- `model.rs` and `crates/article-core/resources/app.card`: actual bounded L0 title/source realization and setter dispatch; builtin package identity and per-open grant.
- `storage.rs`: per-account atomic document, media and resumable operation storage.
- `backend.rs`: host-only Matrix identity, encrypted media, durable publish/update/redaction operations and bounded native reader downloads.
- `ui.rs`: navigation, native widgets, review and explicit confirmations.
- `mini_app.rs` / `home/room_screen.rs`: package cards and original-event article reader routing.

Reviewed deviations from image pixels: the model-generated publication record interchanged field labels/values, so native labels use actual destination/version data. Cover focus uses accessible sliders rather than an unimplemented drag overlay. The native source mode and authorization/error states extend the twelve reference scenes. Theme samples use real document text and an optional illustration. Similarity is not scored by the functional test.
