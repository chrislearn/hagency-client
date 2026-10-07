[English](local-development.md) | [中文](local-development.zh-CN.md)

# Local client/server development

Use the checked-out `hagency-client` and `hagency-server` repositories. Each
repository selects its own Rust toolchain. The client console needs Node.js 22+
at build time; its HTTP server is Rust.

## Server

In the server repository, follow its README for first-time setup, including
Pasion resources and the administrator account. On subsequent starts:

```sh
just db-up
just dev
```

The local server uses `http://127.0.0.1:8088`, with management at `/login`
and Pasion registration at `/_pasion/register`. The private files are
`config/dev/hagency.toml`, `palpo.toml` and `pasion.toml`. Use three separate
PostgreSQL databases: `hagency`, `palpo`, `pasion`; the provided Compose
PostgreSQL listens on `127.0.0.1:55438`.

To allow client enrollment, set `[fleet_access] allow_self_service = true` in
`hagency.toml`. For local test registration, Pasion supports
`[experimental] fixed_verification_code = "123456"` and blackhole email/SMS
providers. These options belong in `pasion.toml`.

## Client

```sh
just init-dev  # first time: install console build dependencies
just dev
```

The watcher builds the static console and Rust executable, initializes private
state if needed, and starts outbound transport on `127.0.0.1:13300`.
In another terminal:

```sh
just console
```

Open the private link printed in that terminal; do not save it in a log.
On the Server login card enter `http://127.0.0.1:8088` and an installation name,
then sign in through Pasion. The client creates/reuses its Fleet, imports its
configuration and verifies a real Matrix event receipt automatically.
No manual configuration download or public client address is needed.

The console is at `http://127.0.0.1:13300/console/`. State, SQLite databases and
private machine credentials live in `.run/dev-state/`; console build output is
under `.run/dev-console-*`. These directories are ignored by Git. Server
binding/installation identity persist across restarts. Pasion browser sessions
are kept in memory, so sign in again after restarting the Rust host.

Rust or console source changes rebuild and restart the client. Failed builds
preserve the previous running host. The server's `just dev` also watches its
Rust, frontend and component configuration files. Use `Ctrl+C` to stop each
watcher. Custom client state/listener:

```sh
just dev --state-dir .run/another-client --listen 127.0.0.1:13301
just console .run/another-client 127.0.0.1:13301
```

When Google Fonts is unreachable, `just dev --font-cache /path/to/.next`
reuses font files from a previous successful console build. The default cache
location is `.run/font-cache`; it contains the previous build's `static/chunks`,
`static/css` (when present) and `static/media` directories.

Connecting the services does not configure a coding agent or publish a model
resource. Configure the intended Codex/Claude runtime and resource quotas in
the client before assigning agent work.
