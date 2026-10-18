# Desktop progress

## 2026-10-13 — desktop issue #2

Final design was saved and independently reviewed before implementation, on branch `codex/issue-2-cross-device-config`. Added cross-device model and Agent-wide policy configuration to the existing native pages. Initial metadata automatically requests and polls the assigned device environment; cached snapshots remain read-only while offline. Device name, connectivity, heartbeat/observation time, provider installation/version/readiness/account verification and target workspace are displayed. Remote provider connect explains its owner-wide effect. Login/disconnect/runtime/approval/reset/Room/requester operations are blocked remotely.

Requests are fenced by identity, UI request, Agent and assignment generation/device. Switching Agent/account clears visible catalogs/profile/workspace. Separate background metadata polling preserves mutation feedback. Mutation intent keys survive same-panel refresh/selection on unknown or transport errors, and are removed after confirmed success or terminal failure. Keys are in-memory UI state; full Desktop restart does not restore a requester-side pending intent. Worker-side receipt persistence is provided by client.

Focused Makepad widget and routing tests cover target-only catalogs, precise Octos reference selection, offline cached settings, online saves, target workspace/version label, scope allowlist, stale metadata and idempotency intent transitions. Initial full library snapshot passed 458 tests with 2 existing ignored tests; final snapshot/build results will be recorded below. The English/Chinese catalog checker passes with no missing keys. Root owns final client CI SHA pin and integrated A/B-process validation; physical multi-device/provider-inference validation is not claimed here.

Final frozen-source verification:

- `cargo test --locked --lib`: exit 0; 458 passed, 0 failed, 2 ignored. The ignored tests are the pre-existing backend interop fixture and isolated Palpo Moments fixture; they are not counted as passing.
- `cargo build --locked --bin rinx`: exit 0. Existing Cargo patch/duplicate-package warnings and the macOS large unwind-table linker warning remain; no new compilation errors.
- `python3 tools/wechat-ux/check_i18n.py`: exit 0; 1821 catalog entries, 1430 translated call sites, 0 missing keys.
- `git diff --check`: exit 0.

Independent Desktop review corrected first-load metadata errors, preserved online state for validation/CAS errors and quiet metadata polls, and verified generation changes clear old catalogs/workspaces. Request intent fingerprints also include the execution device ID/generation. Client review observation about input+result envelope response bounds was forwarded to client owner and resolved there. No further Desktop-specific actionable review finding remains.

Root pinned CI to client `33c75d3a645f4732a086ece5fa8297fc9553a419`, and rebuilt the locked Desktop binary against that exact client source after its final Agent-specific busy correction. Cargo.lock has no dependency changes. Final real embedded Server/Pasion/Palpo with two NativeOwner processes passed, including B without any provider executables, A validation/persistence, both reopens, idempotent result and exact runtime Ledger resolver under a real active owner-direct binding. This uses a declared provider fixture and makes no physical-device or actual inference claim.
