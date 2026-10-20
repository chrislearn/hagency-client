# Shared desktop engine

The `hagency` Cargo package is a library used by `hagency-app`, the native desktop
application. The former standalone CLI, Web console, static page bundle and user
service installer have been removed. Start with the [workspace guide](workspace.md)
and [engine README](../crates/hagency/README.md).

## Boundary

The desktop adapter in `crates/hagency-app/src/hagency/backend.rs` opens
`hagency::native_owner::open_with_matrix`. It pins the current Matrix account,
uses the authenticated owner API in process, and keeps private state under an
exclusive owner lock. The only network listener in this owner facade accepts the
OAuth login callback on an ephemeral loopback port. It exposes no management
pages or management API endpoints.

The `console` module name is retained for shared business logic. It does not
imply a running Web console. Test probe executables and the process guardian are
engine infrastructure, not a standalone client product.

## Libraries

| Package | Responsibility |
| --- | --- |
| `hagency` | Owner facade, account authorization, Agent operations and SDK composition |
| `hagency-core`, `hagency-store` | Domain rules, private state and durable records |
| `hagency-matrix`, `hagency-matrix-format` | Matrix messaging, identity, encryption and formatting |
| `hagency-execution`, `hagency-runtime`, `hagency-platform`, `hagency-agent-local` | Agent execution, provider protocols and process custody |
| `hagency-files`, `hagency-media`, `hagency-media-store` | Files, attachments and private media storage |
| `hagency-metering` | Usage normalization |

## Verification

On Windows, run the repository wrapper to keep all build outputs and caches on D:.

```powershell
./tools/dev.ps1 check --locked -p hagency --lib
./tools/dev.ps1 test --locked -p hagency --lib native_desktop_listener_exposes_only_oauth_callback
./tools/dev.ps1 test --locked -p hagency-app --features agent_chat,octosense-module --lib
```

The exact engine package selection, host qualification exclusions and lint gates
are recorded in [CI](../.github/workflows/ci.yml). The compiled test inventory and
production caller checker run there alongside the SDK tests. The current product
contract is [native-owner-client.spec.md](../specs/native-owner-client.spec.md).
