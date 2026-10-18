# Client workspace

Hagency Desktop and the Agent engine share one Cargo workspace and one reviewed
`Cargo.lock`. The desktop package is the default member; use explicit packages
for headless builds and tests.

| Location | Purpose |
| --- | --- |
| `crates/hagency-desktop` | Native Makepad application, resources, mini apps, packaging and desktop tools |
| `crates/hagency` and other `crates/hagency-*` packages | Local Agent engine and CLI |
| `crates/article-*`, `crates/miniapp-*`, `crates/system-apps`, `crates/theme-contract` | Shared editor, mini app and theme libraries and package tool |
| `crates/octosense-app-contract`, `crates/robius-file-picker` | Reviewed local third-party sources with their original notices |
| `tools/native` | Engine verification and development scripts |
| `tests/fixtures` | Shared engine test vectors |
| `mockup` | Existing browser Owner console and historical prototype |
| `dev-vibe/desktop` | Imported desktop collaboration records and release assessment |

## Build and test

Windows builds and caches must stay on D:. From the repository root:

```powershell
./tools/dev.ps1 check --locked -p rinx --lib
./tools/dev.ps1 test --locked -p rinx --lib
./tools/dev.ps1 test --locked -p article-core -p rinx-miniapp-core -p rinx-miniapp-catalog -p rinx-system-apps
./tools/dev.ps1 run --locked -p rinx --bin rinx
```

The wrapper sets `CARGO_HOME`, `CARGO_TARGET_DIR`, `TEMP`, `TMP` and npm's cache
under `D:/BuildCache/hagency-client`, then checks `cargo metadata`. For WSL,
verify `/mnt/d` is mounted from D: before selecting
`/mnt/d/BuildCache/hagency-client` for all these outputs. Docker builds additionally
require a builder whose data is on D: or remote storage; the existing Docker
Desktop disk on C: does not meet that condition.

On macOS and Linux, install the native dependencies listed in the
[Desktop build guide](../crates/hagency-desktop/docs/local-development.md), then run
`cargo run --locked -p rinx --bin rinx`. The workspace macOS runner creates a
development app bundle only for `rinx`; engine tools and test binaries run directly.

Headless engine examples use explicit selection:

```sh
cargo build --locked -p hagency --bin hagency
cargo test --locked -p hagency-runtime -p hagency-agent-local
python3 tools/check-workspace-layout.py
```

The CLI's browser assets are still built with
`node mockup/scripts/build-native-console.mjs --output <new-directory-on-build-disk>`.
Set npm cache and temporary staging directories to the build disk before running
it on Windows/WSL. Set `HAGENCY_CONSOLE_DIR` for an embedded-console CLI build;
Desktop needs no browser console assets.

## Desktop packaging

Run desktop tools and cargo-packager from `crates/hagency-desktop`.
`tools/package-desktop.py` discovers the unified Cargo output directory for
resource collection; `tools/package-octos.py` stages its independent pinned runtime
under that same build directory. They do not assume a crate-local `target/`.

Desktop CI, platform builds and installers are in
`.github/workflows/desktop-{main,builds,release}.yml`. Engine CI continues to select
the engine packages explicitly, so Linux's headless checks do not require Makepad.
The existing CLI `nv*` artifact workflow remains separate from Desktop's `v*`
installer workflow. Desktop and the engine workspace are currently 0.6.0.
To publish, run `desktop-release.yml` on `main` with `publish=true`, or push the
matching `v0.6.0` tag. All five platform jobs must test and build successfully;
only then does the workflow verify seven installers and their checksums and publish
the complete GitHub Release as Latest. Manual runs with `publish=false` only upload
CI artifacts. Published releases cannot be overwritten by a rerun.
The outstanding dependency assessment remains in
[`dev-vibe/desktop/release-0.5.0`](../dev-vibe/desktop/release-0.5.0/assessment.zh.md).

## Source history and downloads

Desktop history through `52d88b2020a322d9c1883b412e74968c1a60ce01` was imported
with its original commits as a merge parent. Subsequent path moves live in the
same history. Original Apache/MIT and third-party notices remain with their code.

Historical installers remain available from
[the original Desktop releases](https://github.com/hagency-org/hagency-desktop/releases).
Current installers are published to
[Hagency Client Releases](https://github.com/hagency-org/hagency-client/releases/latest).
Cloud uses its stable `releases/latest/download/` URLs and the unchanged
`hagency-desktop-{platform}-{arch}` asset names; version-specific links and
`SHA256SUMS.txt` are available on each release.
