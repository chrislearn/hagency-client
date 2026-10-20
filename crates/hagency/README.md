# Desktop owner engine

This crate is a library dependency of `hagency-app`. The desktop constructs
`native_owner::NativeOwner::open_with_matrix` and invokes typed commands in
process. Login, account isolation, Agent execution, budgets, approvals, recovery,
provider handling and speech remain shared engine functionality.

The only actual HTTP listener is the loopback OAuth callback on an ephemeral
port. It accepts `/console/server-login/callback`; it does not serve console
pages or management APIs. Internal request routing preserves the engine's
existing authentication and policy checks without exposing them over a socket.

There is no product CLI, static Web bundle, headless user service installer or
CLI release workflow. Explicit probe binaries under `tests/fixtures` are test
harnesses, not desktop product entry points.

See [the desktop guide](../hagency-app/README.md) and [workspace guide](../../docs/workspace.md).
