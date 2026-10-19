# Rinx deployment and Octos ownership

Rinx's deployment implementation is shared by the built-in catalog, App Hub
mini apps, and assistant. The host-owned service remains in `src/host/service.rs`;
each mini-app instance uses the scoped context adapter in `src/host/octos.rs`.

Standalone builds include the local and remote assistant connectors. Local mode
uses the packaged Octos executable and Rinx's own data directory. Remote mode
connects to the explicitly configured server; its token is not persisted.
Hosted builds disable default features and enable `octosense-module`. OctoSense
supplies the scoped service, owns the shared kernel, and controls AI settings.
Closing a mini app revokes that instance's work without stopping the host kernel.

See [ADR 0007](adr/0007-host-owned-octos-app-peers.md) for the ownership contract
and [Octos packaging](../packaging/README-octos.md) for build and deployment
instructions. App Hub installation and permissions are documented in
[ADR 0006](adr/0006-shared-app-hub-miniapps.md).
