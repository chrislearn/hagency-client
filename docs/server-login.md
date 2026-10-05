[English](server-login.md) | [中文](server-login.zh-CN.md)

# Pasion login and automatic Fleet enrollment

Use the local `hagency-client` checkout. The executable is still named `hagency`.
Start it with `hagency start`; for a source build with separate console assets,
use `hagency serve --state-dir <state> --palpo-transport --console-assets <assets>`.
Open the local console access link once using `hagency console-access` with the
same state directory and listen address.

In Setup or Project sides, enter the Hagency Server address and Fleet name,
then click **Sign in and connect**. Sign in on the server's Pasion page. The
Rust host exchanges the authorization code with PKCE, verifies the account,
requests its own Fleet, writes the returned configuration and starts outbound
transport. Connection verification retries automatically, for about three
minutes after the initial attempts. A timeout is shown as failure, not success.
No coding agent runs merely because a Fleet is connected; configure the local
coding runtime and offer resources separately.

The server requires delegated Pasion authentication. In its own `hagency.toml`:

```toml
[fleet_access]
allow_self_service = true
max_per_user = 3
```

Self-service defaults to disabled. A valid account can still log in locally when
enrollment is disabled. The server owns App Service registration authority;
users receive only their Fleet credentials. One installation ID per local
state directory makes repeated logins idempotent. Each Fleet has its own
App Service. The client needs no public IP or incoming callback from the server;
its OAuth browser callback uses a loopback IP. Remote servers must use HTTPS;
HTTP is allowed only for loopback IPs in development.

The first binding requires existing local operator access. Afterwards the
server origin, Pasion subject and Matrix account are pinned in private
`server-login.json`. Another user cannot log in and control this client. The
Pasion token stays in Rust memory, never in browser storage or this binding
file. The host verifies it through the pinned server, which introspects Pasion
using its private service credential. Successful verification is cached for at
most 30 seconds; expired, revoked or unverifiable tokens are refused. Local
sessions last at most 15 minutes, and require another login after a restart.
The ordinary `console-access` command remains local recovery authority.

`palpo-transport.json`, `palpo.machine_token` and `palpo-appservice.json` remain
private runtime files. The machine credential is separate from the human login,
so a logged-out browser does not stop an already configured Fleet. Manual
import remains available under **Import an existing configuration (optional)**.
An installation already configured with another Fleet refuses automatic
retargeting. Use its existing configuration, or initialize a separate state
directory for the new automatic enrollment.

Native server API: `GET /_hagency/client/v1/discovery`, `GET .../identity`,
`POST .../fleets`, and `POST .../fleets/{id}/connect`. Authenticated requests
use a Pasion bearer token without browser cookies or Origin headers. Enrollment
accepts only `installationId` and `name`; the owner comes from verified identity.
