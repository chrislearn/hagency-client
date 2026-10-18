# Rinx system apps

English | [简体中文](README.zh-CN.md)

Apps listed in the root `system-apps.json` ship with both standalone and hosted
Rinx. The catalog is compiled into Rinx; a local import cannot change it.

- `article-editor/bundle/` contains the canonical App Hub manifest and catalog assets.
- `article-editor/native/` contains the native Makepad editor and publication workflows.
- Shared document/rendering code stays in `crates/article-core` and `crates/article-makepad`.
- Host Matrix and Octos adapters live in `src/host/`; grants remain account- and instance-scoped.

For a script app, create `apps/<name>/bundle/manifest.json`, `main.splash` and its
assets, then add `{"directory":"<name>"}` to the catalog. Declare exact service
capabilities. Do not collect Matrix credentials or AI provider keys. A native
app additionally needs a compiled registry variant; adding a JSON field alone
cannot introduce native code. Preserve existing app IDs when moving code.

Run from the repository root:

```sh
bash tools/package-system-apps/check.sh
cargo test --locked --manifest-path crates/system-apps/Cargo.toml
```

The check validates and computes digests without rewriting source manifests.
Normal Rinx builds embed the validated bytes automatically. Review source,
manifest permissions and tests in one Rinx PR; release built-ins with Rinx.
External apps remain App Hub packages. See [ADR 0008](../docs/adr/0008-rinx-system-app-catalog.md).
