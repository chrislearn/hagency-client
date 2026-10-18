# ADR 0008: Rinx system apps and repository boundaries

Status: Accepted; implemented in this branch.

## Decision

Rinx ships a build-owned catalog of native and OctoScript apps. The root
`system-apps.json` selects directories under `apps/`; each directory has an
OctoSense App Hub `bundle/manifest.json`. Script bundles contain `main.splash`
or the existing L0 `page.card`. A native entry is selected in the trusted
catalog, outside the portable manifest, and must name a compiled registry
variant with its exact stable app ID. An imported bundle cannot register a
native entry or claim any bundled app ID.

The first system app is the article editor. Its UI and workflows live under
`apps/article-editor/native/`. Its existing `org.octosense.article-editor` ID,
L0 source hash on Matrix share messages, library paths and consent grants stay
compatible. It remains native Makepad. `article-core` and `article-makepad`
remain reusable libraries. Native code is compiled as a Rinx module; it is not
a separately downloadable executable or an independent Rust crate yet.

`src/miniapps/` owns the catalog screen, import/review, Splash instances and
lifecycle. Existing `octoscript_apps` Rust paths remain compatible through the
module declaration. `src/host/matrix/` contains SDK operations and their account,
room and service gates; `src/host/octos.rs` contains scoped assistant contexts.
`src/host/service.rs` retains the existing feature-gated standalone/local/remote
and injected-host lifecycle, exposed through `rinx::octos_service`.

## Admission and packaging

`rinx-system-apps` is a small Makepad-free packaging library and CLI.
`build.rs` uses it to validate the catalog, parse the canonical App Hub manifest,
resolve requested capabilities and compute the canonical bundle digest. Empty
source digests are filled in the embedded artifact, never in the developer's
source files. Both deployment modes embed the same catalog and bundle bytes.
Repository provenance points to this Rinx repository; the enclosing Rinx release
or pinned Git revision identifies the app source. Native executable code is
covered by that release, while a bundle digest covers its bundle files.

Duplicate IDs, unknown native entries, a native entry with the wrong ID,
script/native entry collisions, traversal, symlinks, stale digests and unknown
capabilities fail packaging. Embedded script bundles are extracted into a fresh
private directory, pass the existing package review/run path, and are checked
against the embedded manifest and digest again before running. Local imports
remain visibly unsigned and cannot impersonate built-ins. Built-in status
never supplies an account, a room grant, or approval for publishing.

Native article operations continue to use `article_core::host::ConsentGrant`
and the article adapter's fixed operation set. Generic manifest capabilities do
not replace those native operation checks or issue a script lease to the
editor. In particular, publishing/withdrawal still require their existing
native confirmation flow. Script apps retain exact-name capability checks.

## Deployment and Octos

Standalone Rinx owns its configured runtime (or explicitly configured remote
connector), credentials and provider settings. Hosted Rinx receives OctoSense's
scoped service and shares OctoSense's kernel and provider configuration. It
cannot start its standalone runtime or show a second provider-configuration
form. Mini-app instances use scoped contexts of Rinx's peer; this catalog does
not introduce a kernel or peer per mini-app.

OctoSense pins Rinx as one native module and consumes its embedded catalog. It
does not copy Rinx's app sources into the OctoSense repository. System apps ship
with Rinx releases. Independently published apps use the existing App Hub
format and publication mechanism; this change does not create another store,
add an App Hub download UI, or claim store signatures for local imports.

## Validation

The portable packager tests digest/materialization, reserved native identity,
entrypoints, duplicate IDs, traversal/symlinks, stale digests/capabilities and
non-overwrite behavior. Native package tests reject imported copies of the
built-in editor. The offline Makepad catalog harness exercises both standalone
and injected-host modes: catalog/import navigation, provider-form visibility,
launching the native editor, and refusal to authorize without a Matrix account.
It never logs into Matrix or starts a kernel.

Local validation on macOS:

- `cargo test --locked --manifest-path crates/system-apps/Cargo.toml`: 7 passed.
- `bash tools/package-system-apps/check.sh`: embedded catalog admitted.
- `cargo test --profile fast --locked --lib`: 208 passed, 1 existing test ignored.
- `cargo check --locked --no-default-features --features octosense-module`: passed.
- `cargo build --profile fast --locked --example system_app_catalog`, then
  `python3 tools/wechat-ux/live/native_system_apps.py`: both deployment modes passed.
- `python3 tools/wechat-ux/check_i18n.py`: no missing translation keys.

Device installation of this catalog change is unverified. The previous editor
SVG integration was tested separately on the OnePlus 6; those results do not
claim device coverage of this new catalog.
