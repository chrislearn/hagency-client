# GitHub CI and desktop installers

The repository inherited Rinx workflows. Before this repair, main CI failed to
find `../hagency-client/native/hagency`, and the inherited release required a
RINX_RELEASE token and Apple signing configuration. There were no Hagency
GitHub Releases at the 2026-10-10 audit.

## Source dependency

Every Cargo workflow now invokes `.github/actions/setup-hagency-client` after
checking out Desktop. It checks out `chrislearn/hagency-client` at the exact
published commit `1f529d9f1af2d16f1cd08a7e7ea1df5d2216a060`, without persisted
credentials, and places it at the sibling path required by Cargo.toml. It refuses
to overwrite an existing sibling tree. Review/update this pin and Desktop's
Cargo.lock together whenever the native client API changes. It intentionally
cannot consume unpublished changes from an operator's local sibling repository.

Publication update: the native runtime/provider additions have now been published
in the pinned client commit above. Desktop Cargo.lock was checked with `--locked`
against this source. Keep future pin and lock updates synchronized; do not remove
`--locked` or silently resolve a moving branch to hide mismatches.

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

Manual runs upload Actions artifacts only (14-day retention). Tagged runs require
the tag to match the package version, and create/update a **draft** GitHub Release
only when all five platform jobs pass. Seven installers are verified against
per-platform checksums and a combined SHA256SUMS.txt. A rerun refuses to overwrite
an already published release. Publishing the draft is a separate operator action.
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
