[English](server-login.md) | [中文](server-login.zh-CN.md)

# Personal server login

Build and run the current [owner client](local-development.md). `hagency start`
opens its local console; `hagency open --state-dir <same-state>` obtains access
through the running host's private IPC. Local console access is separate from
server authentication.

Select a saved account/server or enter the Hagency server origin. Use
`http://127.0.0.1:8088` for loopback development; remote or named servers require
trusted HTTPS. Do not append `/login`, `/_pasion/` or API paths. Complete the
server's official Pasion browser authorization with your personal account.

Pasion PKCE plus real Matrix whoami establishes your identity. Native management
uses `/api/hagency/v1`; server browser management uses the closed
`/api/browser/hagency/v1` BFF. Native APIs reject browser Cookie/Origin requests.
The integrated Appservice stays on the server: no per-installation Fleet, AS
registration, machine-token import or `[fleet_access]` configuration is required.

After sign-in, create/adopt a Project Space and authorized discussion Rooms, then
create your Agent or select an existing one. Permanent owner identity, independent
Room membership and creation policy are enforced by the server. The local owner
controls provider credentials, budget and tools. Sign-in does not start inference;
explicitly start an active scope after configuring its resources.

Saved profiles and recent server addresses support switching accounts. Switching
stops the previous owner's runtime and requires fresh personal authorization;
account state, workspaces and ledgers remain isolated. Provider login is separate
from Pasion login and never exports provider credentials to Hagency server.

For failures, check [server discovery/TLS/readiness](../../hagency-server/docs/LOCAL_DEPLOYMENT.md)
before retrying login. Keep unknown creation requests and execution outcomes for
explicit recovery rather than creating duplicates. Retired Fleet login records
are not setup instructions for the current owner format.
