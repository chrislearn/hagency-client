# macOS Sparkle prototype (issue #4)

This contains the native prototype, Agent lifecycle and product-preference slices of [issue #4](https://github.com/hagency-org/hagency-client/issues/4).
It exercises the real Hagency application, Makepad/AppKit event loop, a custom
Sparkle user driver, a real downloaded DMG, application replacement and relaunch.
Stable release CI now requires the signed Sparkle configuration described below.
No production release has been executed or qualified; the remaining acceptance
matrix is still tracked in issue #4.

## Prototype isolation and limits

- Requires macOS 12+, a macOS GUI session, Cargo/Rust, Command Line Tools and Python 3.9+.
- Optional `macos-sparkle` feature. Ordinary builds/tests require no Sparkle SDK.
- SDK fixed to Sparkle 2.10.0, SHA256
  `c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c`.
- Prototype initialization requires a debug binary, Bundle ID
  `org.hagency.desktop.sparkle-prototype`, a marked private harness root and the
  exact installed executable path. Release builds refuse this prototype identity.
- A per-run profile under `~/Library/Application Support/HagencySparklePrototype/`
  is selected before application threads start, including on Sparkle relaunch.
  Authenticated profiles now use the real owner retirement barrier. The automated
  bundle fixtures remain signed out; they do not copy a production account.
- Only a loopback IPv4 HTTP feed is accepted. Its local-network ATS configuration
  is added only to disposable bundles, never to the product Info.plist.
- Uses ad-hoc app signatures and disposable Ed25519 update-signing keys generated
  with OpenSSL. It does not read or write the user's signing keychain. Private test
  keys are removed when the harness completes or reports a handled failure.
- No Developer ID, notarization, Intel/Rosetta, macOS 12 hardware, production feed,
  live Matrix-account upgrade or production data migration is qualified here.
- The two fixture bundles use the same compiled application source, versions
  `1.0.0` and `2.0.0`, independent code signatures and different resource markers.
  The source Cargo version remains unchanged. This validates replacement and
  restart; it is not a published product release or a new feature-version test.
- Entire application bundles are replaced by Sparkle. The fixture packages
  application/widget resources and the Sparkle framework; it does not package or
  validate an Octos runtime. The profile sentinel verifies that profile data stays
  outside the replacement unit.

## Run

Run these commands from the repository root:

```sh
python3 crates/hagency-app/tools/sparkle-prototype.py sdk
HAGENCY_SPARKLE_SDK="$PWD/.run/sparkle-sdk" \
  cargo build -p hagency-app --locked --features macos-sparkle --bin hagency
python3 crates/hagency-app/tools/sparkle-prototype.py run --scenario success --keep-open
python3 crates/hagency-app/tools/sparkle-prototype.py run --scenario bad-signature
python3 crates/hagency-app/tools/sparkle-prototype.py run --scenario busy-cancel
```

The success scenario leaves the updated test window open with `--keep-open`.
Other scenarios request termination only of PIDs recorded for their exact harness
executable. Test reports/logs remain in the printed root; duplicate candidate
bundles and served packages are removed after passing. Keep failed artifacts for
diagnosis. Close any retained test window before deleting its installed bundle.

For feature-enabled library checks, the native test binary also needs the SDK:

```sh
mkdir -p target/debug/Frameworks
ln -s "$PWD/.run/sparkle-sdk/Sparkle.framework" target/debug/Frameworks/Sparkle.framework
HAGENCY_SPARKLE_SDK="$PWD/.run/sparkle-sdk" \
  cargo test -p hagency-app --lib --locked --features macos-sparkle
HAGENCY_SPARKLE_SDK="$PWD/.run/sparkle-sdk" \
  cargo clippy -p hagency-app --lib --no-deps --locked --features macos-sparkle -- -D warnings
```

The symlink is a local test artifact. If it already exists, verify it resolves to
the same pinned SDK rather than overwriting it. The repository's system-shell
Cargo runner strips DYLD environment variables, so those variables alone cannot
make the test binary load Sparkle. Bundled applications use their own Frameworks
directory and never depend on this test symlink.

## What the harness verifies

`success` observes the actual HTTP appcast and DMG requests, Sparkle download and
ready callbacks, the preparation acknowledgement, original-process exit, a new
PID at version 2.0.0, the expected executable SHA256 and resource marker at the
installed path, intact profile data and a successful new-process update check.
The new process must initialize the bridge and complete that check; merely seeing
a replacement file or a process launch is insufficient.

`bad-signature` corrupts the Ed25519 signature after signing a real DMG. It requires
Sparkle to reject the downloaded package, dismiss the update, release admission,
keep the original executable/process and never become ready or relaunch.

`busy-cancel` holds a real `TaskGuard` (not a live provider task), authorizes an
update through the same Makepad action path, verifies that no DMG request occurs,
cancels Waiting, then releases the guard and verifies that downloading does not
begin accidentally. It does not qualify a real long-running Agent's cleanup.

`events.jsonl` records process/build identities, bridge callbacks, admission and
preparation. `http.json` records loopback requests. `report.json` contains pass/fail
and executable hashes. `app.log` captures the initial process's output. Sparkle's
relaunched process records directly to the same events file without depending on
the original shell environment or stdout.

## Implementation boundaries

The Objective-C bridge owns all objects/blocks on the main thread and exposes an
owned JSON event queue through a C ABI. Rust polls it from Makepad's UI timer;
native callbacks never borrow or mutate a Rust UI object. Commands carry the
current cycle, and reply blocks are removed before invocation. The private
`action.json` harness also binds commands to the process PID and cycle.

The user authorizes an update before `PendingInstall` immediately fences new work.
Already admitted Agent turns keep running through provider exit, ledger settlement
and reply handling. Exact existing approvals and inspection can continue while
original work is active; these observers cannot keep admitting each other after
that work drains. Canceling Waiting drops only the pending fence and resumes intake.
Once both original work and admitted observers finish, ownership transfers to
`InstallGuard` and Sparkle may download. Admission remains fenced through process
exit; QuitRequested/WindowCloseRequested cannot bypass waiting or preparation.
Download errors release admission only after Sparkle dismisses its installation.
Stage-specific cancellation uses Skip for a prepared installation, not Dismiss.

Before the final native installation reply, the UI saves state and starts an
unscoped, retained asynchronous preparation operation. It retires the actual
`Service`/`NativeOwner`, aborts and joins configuration workers, revokes local
capabilities, joins Agent supervisors and provider processes, retains durable
pending server revocations and closes the OAuth callback listener. Matrix sync
then stops under its lifecycle mutex and cannot be restarted by foreground events.
The pinned SDK's stopped state must also be Idle/Terminated/Error, which its
supervisor publishes after joining both sync workers; a unit return with
Running/Offline is rejected. Account jobs are drained and cannot reopen for this process. The UI verifies the
expected account generation and saves state a second time before authorizing
Sparkle to terminate. No long shutdown wait runs on the AppKit thread.

A 15-second UI deadline postpones installation and cancels Sparkle's pending
installer, while the original cleanup and admission fence remain owned. A late
positive cleanup receipt permits ordinary quit after installer dismissal; it never
auto-installs or recreates an owner. Failed or missing receipts remain failures
on retries, even if the current-owner slot/worker registry is empty. Once owner
retirement starts, a fresh process is required to try another update.

`shutdown::Task` monitors the original operation, `Registry` retains removed owners,
and manager-owned join queues retain removed workers across canceled waiters.
`RetainedChild` also retains and reaps provider children dropped during an aborted
initialization/request; `kill_on_drop` alone is not an exit acknowledgement.

The separately embedded standalone assistant has a remaining upstream limitation:
the pinned Octos `OctosAppService::shutdown()` returns no termination receipt, and
`Core::shutdown_within()` reports that a generation existed rather than proving it
exited. If that service was started (including subsequently turned off), this
prototype refuses installation before retiring the Agent owner. Restart with the
assistant off to update. The real Agent runtime's Codex/Claude/Octos adapters use
the checked process barrier instead. This deliberate refusal is not a successful
assistant-stop qualification.

The prototype sets Makepad's resource package root before resource registration
and packages resources inside the .app. A developer-source-path build can launch
initially yet stall in resource File::open after a LaunchServices relaunch; such a
launch alone is not treated as successful UI recovery.

## Local qualification, 2026-10-22

On this ARM64 Mac running macOS 27.0.1, the packaged-resource success run changed
PID 91899 to 92240 and installed Bundle version 2.0.0 at the same installed path.
The executable SHA256 changed from
`40186646ea73215c68c11b967c909f488f963fada8711543026f2ae419c4a060` to
`1e6515447256a7be850c8b544cb63e63374a8f71be661f71f5e44b7b3578235a`.
The new process initialized the bridge, checked the feed and reached Current;
native visual inspection also confirmed the actual login window rendered.

All three harness scenarios passed. The bad-signature run contains Sparkle
SUValidationError (3002), with no Ready or relaunch. The busy/cancel run performed
no DMG request even after its held TaskGuard was released. Reports remain in:

- `~/Library/Application Support/HagencySparklePrototype/success-dur195wj/`
- `~/Library/Application Support/HagencySparklePrototype/bad-signature-o1wsqdmw/`
- `~/Library/Application Support/HagencySparklePrototype/busy-cancel-6eftxgsw/`

Feature-enabled application library: **567 passed, 0 failed, 2 existing ignored**.
Strict feature-enabled library Clippy passed. The executable's actual Mach-O
minimum OS is 12.0. A normal feature-disabled library check also passed with
HAGENCY_SPARKLE_SDK removed from its environment. There is no Developer ID identity on this host, so no
Developer ID/notarization result is claimed.

Retained diagnostic outcomes: the initial developer-path relaunch stalled in
resource File::open before eventually recovering; the packaged-resource run
avoids those source paths. An initial feature-test launch could not find Sparkle
because the shell runner stripped DYLD settings; the documented test-only rpath
symlink resolves that loader prerequisite. A later complete test run had six
storage-full failures; moving test scratch space to the data volume produced the
full passing result above. Reproduce that run with
`TMPDIR="$PWD/.run/sparkle-test-tmp"` after creating that directory. These earlier
results remain diagnostic evidence, not passing tests.

Remaining issue #4 work includes production identity/configuration, checking and
preference migration, a confirmed standalone-assistant shutdown API, cancellation
across all installation stages, complete signed runtime packaging, Developer
ID/notarization, dual-architecture appcasts/publishing and the complete system/path/
error matrix. Real authenticated Matrix-profile upgrade/recovery still needs its
own end-to-end qualification.


## Agent lifecycle qualification, 2026-10-22

The explicit live test runs the installed local Codex with its provider-owned login,
a temporary ledger/workspace and local transport receipts. It sends no Matrix
messages and does not copy credentials. It requests the update during provider
execution, confirms the original task is not canceled, waits for the actual model
answer, settled accounting and durable Replied state, then verifies no further
poll after the fence and stops the now-idle worker. This qualifies that bounded
execution path, separately from a live Matrix-account upgrade.

```sh
TMPDIR="$PWD/.run/sparkle-test-tmp" \
HAGENCY_LIVE_CODEX="/absolute/path/to/codex" \
HAGENCY_LIVE_CODEX_HOME="$HOME/.codex" \
HAGENCY_LIVE_MODEL="your-available-model" \
cargo test -p hagency --lib --locked --no-default-features \
  live_codex_update_waits_for_original_turn_and_reply_before_shutdown \
  -- --ignored --nocapture
```

The live test is explicitly ignored in the ordinary suite because it requires a
local login and an actual model request; its separate result is reported only
when explicitly run. Other coverage includes pending/cancel/approval admission,
canceled waiters rejoining the original cleanup, panics and retained retirement
failures, configuration-initiated shutdown without self-join deadlock, dropped
child reaping, save/account checks and refusal to install after a preparation
timeout even when cleanup finishes later.

The new isolated native success run (`success-b31nbndw`) passed real DMG download,
preparation, installation and relaunch, PID 92540 → 92793, with version 2.0.0,
resource marker/profile sentinel and a Current check in the new process. The bundle
harness independently copies/signs each bundle and strips disposable executable
symbol tables before signing; source binaries are unchanged. APFS clone experiments
`success-axb94vuy` and `success-fz1vm08x` failed outer bundle signing with the Code
Signing subsystem's internal error and were never launched. Their reports remain
under the harness roots, with duplicate bundles moved to the paths recorded in
`artifacts-moved.txt`; those failures are not installation passes.


Final lifecycle-source results:

- Core library: **245 passed, 0 failed, 7 ignored** (six pre-existing plus the
  explicit live qualification). Agent-local library: **80 passed, 0 failed,
  3 pre-existing ignored**. Application library: **570 passed, 0 failed,
  2 pre-existing ignored**; rerun after the checked Matrix-stop change also passes.
- Explicit installed Codex `gpt-6.1-sol` test: **1 passed, 0 failed, 0 ignored**,
  including all retired child-process receipts. This is not live Matrix delivery.
- Final native success `success-6g3h6y_n`: PID **24507 → 24749**; Bundle **2.0.0**,
  executable SHA256 `7e87ac43417b385ced943a2a7a07517804c385a0074d0c0ed7766c73bce30af3`
  → `31d3498380f02fe6c8c8660d377627ad750789e6b1314d9564c6cd9fed56d42d`,
  real DMG/preparation/relaunch/Current/resource/profile/code-sign checks all pass.
- Bad signature `bad-signature-d8hfc9d2`: rejects with Sparkle 3002, original
  process and executable retained. Waiting/cancel `busy-cancel-iu6bqjyy`: no DMG
  request; a queued real AppKit termination request is blocked with the original
  task held, canceling Waiting leaves the original task owned, then normal AppKit
  quit exits zero after that task is released. This held-task probe is separate
  from the actual model execution test above.
- Strict no-dependency Clippy for all three libraries and tests passes. Ordinary
  app library check with the Sparkle SDK variable unset passes. Catalogs:
  **2182 keys, 1682 translation sites, zero missing**. Python compile/whitespace
  checks pass; the final Mach-O minimum OS is **12.0**.

The configuration worker's direct `execute_locked` path and resident service
startup also participate in admission. A dedicated configuration test confirms
fencing happens before authorization/claim and canceling the wait restores normal
admission. The final core suite includes that test; earlier combined logs have
244 core passes before its addition.

Logs: `/tmp/hagency-update-final-core-tests-5.log`,
`/tmp/hagency-update-configuration-fence-test.log`,
`/tmp/hagency-update-final-tests-4.log`,
`/tmp/hagency-update-final-app-tests.log`,
`/tmp/hagency-update-final-live-codex.log`,
`/tmp/hagency-update-final-clippy-4.log`,
`/tmp/hagency-update-final-default-check.log`,
`/tmp/hagency-update-final-build-4.log`,
`/tmp/hagency-update-final-native-success-2.log` and
`/tmp/hagency-update-final-native-{success,bad-signature,busy-quit}.log`.
The private harness roots contain reports, events and HTTP evidence. Installed
bundles moved off the system volume are located in each root's
`artifacts-moved.txt`; the final success bundle is retained at its installed path.
No harness process is intentionally left running.

Additional preserved failures: two new native-owner tests initially passed an
already-existing temp directory instead of a new state subdirectory and failed
with `native_owner_state_unavailable`; their corrected fixtures pass. One HTTP
unknown-outcome fixture stopped its worker after server acceptance but before the
local Replied commit. It now waits for both original receipts (without extending
deadlines or weakening assertions), and the full suite passes. A later staging
run (`busy-cancel-602ipe7b`) exhausted system-volume space while stripping a second
binary, before launching the app. Stripping now happens once in a temporary data-
volume staging directory, each bundle is independently copied/signed, and candidate
staging is removed only after its exact DMG signature verifies. These failures
remain in their original reports/logs, separately from subsequent passes.


## Product configuration and preferences, 2026-10-22

The optional feature now recognizes a deliberately enabled product bundle as well
as the isolated prototype. The product path requires a packaged release executable,
`org.hagency.desktop`, a valid Apple Developer ID signature, the compiled signing
team and Sparkle public key, the stable Cargo bundle version, and the exact HTTPS
feed for the executable architecture. Running an Intel executable under Rosetta
therefore selects the amd64 feed. Debug/development/embedded/guardian identities
cannot activate product installation. An invalid enabled product configuration
leaves the updater disabled with an error; it does not fall back to a second
GitHub update worker. The prototype retains its original marked-root checks.

Before the updater starts, the old `updates/macos/preferences.json` auto_check is
imported once into Sparkle's host defaults. A completion marker prevents reading
the JSON again. An existing Sparkle choice takes precedence even before that
marker exists. Subsequent UI changes use Sparkle's own persisted property and
schedule; there is no second 30-second GitHub polling loop. Auto-download is not
migrated or exposed, and both unattended-update Info.plist options remain false.
See [Sparkle settings semantics](https://sparkle-project.org/documentation/customization/).
Unreadable historical settings during a required first import refuse updater
initialization and leave migration incomplete; they are not silently reset.

The native driver assigns a cycle to automatic and manual checks, reports release
notes as text (external notes are accepted only as bounded UTF-8 plain text or
Markdown), and exposes the SDK's actual check availability. Previously staged
updates are skipped rather than adopted without admission. Canceling a waiting
update preserves the found candidate. Canceling a download waits for native
dismissal before releasing admission; canceling during extraction defers Skip
until Ready. Starting cleanup remains irreversible within that process, so late
completion after a timeout/cancel cannot install or recreate the owner. Ordinary
exit and preparation still use the same retained Agent/Matrix barrier.

### Prepare a signed candidate

Python 3.11+ is required for candidate preparation (the native prototype harness
continues to support Python 3.9+). First fetch the fixed SDK using the existing
`sdk` command. With the normal Apple signing/notarization variables provisioned,
set these public build inputs and run the existing build script:

```sh
export HAGENCY_SPARKLE_SDK="$PWD/.run/sparkle-sdk"  # absolute extracted SDK path
export HAGENCY_SPARKLE_PUBLIC_KEY="your Sparkle Ed25519 public key"
export HAGENCY_MACOS_SPARKLE=1
cd crates/hagency-app
bash packaging/build-macos-dmg.sh
```

The script derives HAGENCY_SPARKLE_TEAM_ID from APPLE_TEAM_ID and backs up both
manifest and plist before modification. `prepare-macos-sparkle.py` adds the feature
to the temporary default list (the pinned Robius hook builds the workspace without
feature arguments), embeds the fixed framework/license, stamps the correct
architecture feed and disables unattended installation. The script signs the
Sparkle nested code from inside out without app entitlements, then signs the app,
requires app notarization/stapling/Gatekeeper before embedding it, and requires
DMG notarization/stapling/Gatekeeper. Its exit traps restore both source files,
including a failed packager or a later disk-image step.

All stable macOS version sources now use the Cargo x.y.z value: committed plist,
release workflow stamping and the signed script agree. A timestamp or Actions run
number can no longer override the build version.

The same candidate path is wired into stable release CI when the configured
Apple/Sparkle signing inputs are present. Published stable releases refuse missing
configuration or an unsigned macOS fallback. Manual unsigned build-only runs and
prereleases retain their existing notification path. Prereleases use the numeric
base version for CFBundleVersion and are never offered through the stable feed.

`macos-update.py sign` inspects the actual app mounted read-only from the final,
stapled DMG, verifies its bundle settings, executable architecture, Apple team,
framework signature and offline tickets, then invokes the fixed Sparkle signer
with an explicit private file. No Keychain fallback is allowed. It signs both the
final DMG and a receipt binding its actual version/architecture/hash to the exact
feed bytes. The portable release verifier independently checks both signatures
with the configured Ed25519 public key, the immutable tag URLs and all24 expected
assets (the original seven installers, their checksum files, Windows sidecars,
and both macOS feeds/signature/receipt sets). Unexpected files are refused before
upload. The release is assembled as a draft, downloaded back and compared before
publication/latest changes. After publication, anonymous latest feed redirects,
Content-Type, cache bytes and immutable DMG bytes are checked; a failed public
check leaves the job failed and does not assert successful qualification.

The actual Developer ID/notarization/public GitHub-feed execution, complete
bundled runtime, authenticated Matrix-account recovery, standalone-assistant stop
receipt, Intel/Rosetta and macOS12 hardware acceptance remain unqualified. No
production-update availability claim is made. Nothing in this development run
provisioned secrets, accessed the production signing Keychain or published a tag
or release. See [the release qualification runbook](macos-update-release.md).

Additional native scenarios:

```sh
python3 crates/hagency-app/tools/sparkle-prototype.py run --scenario preferences
python3 crates/hagency-app/tools/sparkle-prototype.py run --scenario download-cancel
```

The preference scenario uses a unique SUDefaultsDomain, signed-out profile and
ordinary AppKit exits. It proves first import, changed preferences over relaunch,
ignoring malformed old JSON after migration, and precedence of a pre-existing
Sparkle choice before a marker. It installs no update. The download-cancel
scenario throttles a real loopback DMG response, cancels through the same Rust/UI
action, requires native dismissal/admission release, and checks that the original
binary/process remain and a new update check is possible. These scenarios remain
separate from full successful installation and from live Agent qualification.

Candidate/DMG staging is now on the data volume under
`.run/sparkle-product-artifacts/<scenario-root>/`, recorded in artifact-root.txt.
Only the installed test application runs under Home, avoiding the earlier TCC
resource issue. Stopped fixture bundles moved after validation have their location
recorded in artifacts-moved.txt; logs/events/reports remain in the private Home
root. These paths are ignored runtime artifacts, not release inputs.


### Final product/publishing source qualification

- Feature-enabled application with octosense-module: **574 passed, 0 failed,
  2 existing ignored**; strict all-targets app/miniapp Clippy passes. Ordinary
  app check with HAGENCY_SPARKLE_SDK unset passes.
- Three candidate configuration tests and seven portable/signature tests pass
  with no skips on this native host, including actual pinned Sparkle signatures
  over archive and JSON bytes verified independently by OpenSSL. These offline
  fixtures do not establish Developer ID or notarization.
- Native preference scenario `preferences-w0tqw4jq`: five distinct application
  PIDs, one-time import, explicit choice persistence, ignoring damaged old JSON,
  pre-existing Sparkle choice precedence, and an **actual automatically scheduled
  loopback check reaching Current**. No installation or DMG request is performed.
- Final native DMG success `success-ni0lg1u5`: **PID28863→29034**, source executable
  SHA256 `0e34c20811331f3c1e0ede0d02aa07ffcc6252385685d1654f301ea5895370e0`,
  target `49e9ad3bb690fa5972c260583bb6bf0e1e13d39c7ab72da7e5d4a13047fd2d1d`.
  New resources, unchanged profile sentinel, native preparation and a new-process
  Current check all pass. Signing remains ad-hoc app + disposable Ed25519 DMG.
- Final native download cancellation `download-cancel-zwicfv15`, wrong signature
  `bad-signature-9wmsgom2` and busy/AppKit-quit cancellation `busy-cancel-hlstvn84`
  pass separately. Their stopped installed bundles are retained on the data volume
  at the paths in artifacts-moved.txt, with reports/events remaining under Home.
- i18n:2186 keys/1682 sites/missing0. YAML and all42 workflow shell blocks parse;
  Bash signing/config-upload scripts pass syntax checking. Actual release job
  execution and public redirect/CDN checks are not represented by those tests.

Logs: `/tmp/hagency-sparkle-product-qualified-tests.log`,
`/tmp/hagency-sparkle-product-qualified-clippy.log`,
`/tmp/hagency-sparkle-product-default-check.log`,
`/tmp/hagency-sparkle-product-automatic-build.log`,
`/tmp/hagency-sparkle-product-native-preferences-auto-fixed.log`, and
`/tmp/hagency-sparkle-product-qualified-{success,download-cancel,bad-signature,busy-cancel}.log`.

Preserved failures: `download-cancel-ooqg8z1m` and `download-cancel-2q20cpec`
failed codesigning an independent test executable before app launch. The system
volume was low on free space; no more specific signing cause is asserted. Moving
candidate/DMG staging to the data volume avoids those failed runs and the final
independent scenarios above pass. `preferences-4ztc43cz` successfully performed
an automatic HTTP check but timed out waiting for Current: unlike manual checks,
background no-update checks do not invoke the user-driver not-found UI callback.
The native update-cycle completion delegate now handles that result; the fresh
five-launch automatic-check scenario above passes. None of these original
failures is relabeled as a pass.


### Signal-wake refinement and latest native evidence

Native events now wake Makepad through a static SignalToUI callback; that callback
never borrows Rust UI objects. Product idle/check/download states do not retain
an updater poll timer. A short timer is used only while draining tasks or awaiting
retained preparation, and ordinary prototype file-command runs also use it.
State changes redraw the existing controls. The isolated `signal` scenario turns
off the prototype idle timer and requires an actual automatic HTTP check and
Current event, rather than inferring delivery from a poll.

All six fresh scenarios pass after this refinement:

- signal-f1n0pid3: no updater idle timer, actual signal-driven check reaches Current.
- preferences-nwqxwxnz: five launches with one-time/persisted/pre-existing choices
  and an actual automatic check; no download.
- download-cancel-q6xjn0g2: real throttled DMG canceled, admission released after
  native dismissal, original executable/process retained and a new check works.
- success-c_dxxger: actual replacement/relaunch **PID77517→77662**, SHA256
  `1db977cc8b0c6762494080d358709e087cd64ea20eb0b971b5cee7a41046722e`
  → `084506516261dfb492ea0fcb156baeed07344ee2cc25df8a79d50f2c3bd7ef4e`,
  new resources, preserved profile and new-process Current check.
- bad-signature-r9i1jf5d: actual signature rejection, original process/binary retained.
- busy-cancel-civ76045: waiting cancellation and real AppKit quit remain guarded.

Source validation:574passed/0failed/2existing ignored at
`/tmp/hagency-sparkle-product-ready-tests.log`, strict all-targets Clippy at
`/tmp/hagency-sparkle-product-ready-clippy.log`, and ordinary no-SDK check at
`/tmp/hagency-sparkle-product-ready-default.log`. Focused update22 and strict
review Clippy after the redraw refinement are logged under
`/tmp/hagency-sparkle-product-review-{update-tests,clippy}.log`.
Native logs: `/tmp/hagency-sparkle-product-ready-{signal,preferences,download-cancel,success,bad-signature,busy-cancel}.log`.
The stopped installed bundles were moved to their recorded data-volume artifact
paths. None of these ad-hoc fixtures qualifies a production signature or login.

The first signal-c43u5u3x run exited before Makepad: Objective-C boxed a C logical
expression as JSON integer1 while Rust correctly expected a boolean. Casting to
BOOL fixes that bridge schema. The harness now reports early process exit rather
than waiting for a generic startup timeout; the original failure/report remains.

The pinned Robius0.4.1 packaging hook itself adds the executable Frameworks
LC_RPATH with install_name_tool. Feature builds therefore emit that rpath directly
only outside CARGO_PACKAGER_FORMAT; duplicating it would fail the real packaging
hook. The actual packaging-mode build-script receipt contains Sparkle linking and
packaging_build, with no duplicate rpath directive
(`/tmp/hagency-sparkle-product-packaging-rpath-check.log`).

An actual ad-hoc signed DMG was separately rejected by the Apple identity
requirement before mount, Sparkle signing or feed creation
(`/tmp/hagency-sparkle-product-reject-adhoc.log`). Publication/candidate/packaging
Python suites21passed/0failed/0skipped with the pinned SDK
(`/tmp/hagency-sparkle-product-publishing-tests.log`). This is a negative signing
gate check, not a successful Developer ID/notarization qualification.
