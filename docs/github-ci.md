# GitHub CI and desktop installers

The repository inherited Rinx workflows. Before this repair, main CI failed to
find `../hagency-client/native/hagency`, and the inherited release required a
RINX_RELEASE token and Apple signing configuration. There were no Hagency
GitHub Releases at the 2026-10-10 audit.

## Source dependency

Every Cargo workflow now invokes `.github/actions/setup-hagency-client` after
checking out Desktop. It checks out `chrislearn/hagency-client` at the exact
published commit `7da298a93f2f8d48c8f518d189b76c16f995d943`, without persisted
credentials, and places it at the sibling path required by Cargo.toml. It refuses
to overwrite an existing sibling tree. Review/update this pin and Desktop's
Cargo.lock together whenever the native client API changes. It intentionally
cannot consume unpublished changes from an operator's local sibling repository.

Publication update: the native runtime/provider additions have now been published
in the pinned client commit above. Desktop Cargo.lock was checked with `--locked`
against this source. Keep future pin and lock updates synchronized; do not remove
`--locked` or silently resolve a moving branch to hide mismatches.

The pin also includes the native API migration to `/hagency/api/v1`, matching
the server namespace and Desktop OAuth fixtures. This path-only update requires
no Cargo.lock dependency changes; locked builds still verify that contract.
Discovery responses rejected with 401/403 are reported as request rejections,
so reverse-proxy configuration errors are not mislabeled as ordinary Matrix servers.

## Checks and artifacts

`main.yml` retains the application/unit/portable crate and resource checks.
`builds.yml` now checks native Linux amd64/arm64, macOS Intel/Apple Silicon, and
Windows amd64 on main/PR. Mobile compilation remains manual-only. Build summary
output is informational; protect main using the actual build/test jobs.

`release.yml` runs on `v*` tags or manual dispatch using Rust 1.98.0 and pinned
cargo-packager 0.10.1 / robius-packaging-commands 0.4.1. Each platform runs library
tests, then packages the app with its existing resource and Octos runtime hooks:

| Platform | Architecture | Artifact |
|---|---|---|
| Linux (Ubuntu 22.04 baseline) | amd64, arm64 | DEB and AppImage |
| macOS | Intel amd64, Apple Silicon arm64 | DMG |
| Windows | amd64 | NSIS EXE installer |

Manual runs upload Actions artifacts only by default (14-day retention).
An explicit `publish=true` dispatch builds and verifies every installer first,
then creates the version tag at that exact source SHA and publishes the release.
This uses GITHUB_TOKEN and does not trigger a duplicate tag build. Tagged runs require
the tag to match the package version, and publish a GitHub Release
only when all five platform jobs pass. Seven installers are verified against
per-platform checksums and a combined SHA256SUMS.txt. A rerun refuses to overwrite
an already published release. Assets are uploaded to a draft first; the workflow
then publishes it after the complete upload succeeds.
The standard GITHUB_TOKEN suffices; RINX_RELEASE is no longer required.

## Signing

Default macOS/Windows packages are unsigned development builds; release notes
state this. To enable macOS Developer ID signing, notarization and stapling:

- Repository variable `MACOS_SIGNED_RELEASE`: `true`.
- Repository variable `APPLE_SIGNING_IDENTITY`: your Developer ID Application identity.
- Secrets: `APPLE_CERTIFICATE` (base64 P12), `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), `APPLE_TEAM_ID`.

The signing script derives bundle paths from `product_name` instead of the
inherited Rinx.app name and accepts the identity from the environment. The
workflow stamps the bundle identifier/display name/version. Failed signing is a
failed package job, with no unsigned fallback or partial release.
Windows code signing, Windows ARM64 installers, automatic updater metadata,
install-and-launch testing on each OS, and mobile store publishing remain future
work. They are not established by compilation or packaging alone.

The inherited Rinx AUR workflow is manual-only; publishing a Hagency release
must not publish the unrelated Rinx AUR package. The inherited license refresh
workflow remains manual and retains its explicit push-token requirement.
Dependabot proposes monthly GitHub Actions updates without auto-merge.

## Native CI repair (2026-10-12)

Strict native builds exposed unused feature/test-only Desktop helpers and two
Client values used only by Unix host-file callbacks. The latter are fixed in
the pinned Client revision above. Keep `-D warnings` enabled so platform-specific
mistakes continue to fail the build. Matrix ID examples and empty text resets
are literal values, while the four missing UI messages now have catalog entries.

The repair passed local strict library compilation, 450 Desktop library tests
(2 ignored), 196 Client runtime tests (5 ignored), and 70 agent-local tests
(1 ignored). The [native GitHub CI run](https://github.com/hagency-org/hagency-desktop/actions/runs/38201853621)
also passed its build, test, translation and theme checks. The final cross-platform
results are tracked on [PR #1](https://github.com/hagency-org/hagency-desktop/pull/1/checks).
Installer/AUR publishing and mobile build workflows were reviewed but not executed.

## Verification checkpoint (2026-10-10)

New/modified workflows passed actionlint syntax validation; new release/main and
server workflow scripts also passed actionlint's ShellCheck integration. The AUR
workflow has an existing nonblocking ShellCheck style finding (SC2129). macOS
signing script passed `bash -n` and ShellCheck error-level validation. No actual
GitHub runner build, signed package, installer launch or release upload is claimed.
The previously unpublished client prerequisite is resolved by the published pin
above. Real GitHub runner and installer verification still remain pending.

Publication validation: Desktop library tests passed locally (447 passed,
2 ignored). Client native binary compilation and Agent-local/runtime library
unit tests passed before publishing the pinned client source. Ignored tests
are not counted as passing. Full platform builds remain GitHub CI results.

## Windows checkout integrity (2026-10-13)

The first tagged installer run found Windows CRLF conversion changed embedded
contract SHA-256 fixtures and the example miniapp bundle digests. Repository
`.gitattributes` now keeps text files at LF on every checkout, preserving the
original producer hashes without disabling verification. A fresh checkout with
`core.autocrlf=true` verifies all 20 contract hashes and keeps every example
miniapp file byte-identical. The failed v1.1.0 tag is retained; the repair ships
in v1.1.1. The final v1.1.2 candidate also installs `libva2` and
`libva-drm2`, required by the packaging dependency scanner for Makepad video.
Failure caches are retained so packaging repairs can reuse compiled dependencies.
