# Sign in with your Matrix account

For matrix.org, enter your username or `@name:matrix.org` and password, then
choose **Sign in with password**. Or choose **Continue in browser** and finish
on matrix.org's authentication page. The **Google** and **GitHub** icon buttons
open the same server-hosted provider chooser. Password entry stays in the app or the
server's browser page; do not put credentials in command-line arguments.

Leave the homeserver empty to discover it from a full Matrix ID. A bare username
uses matrix.org. An explicit server name or HTTP/HTTPS URL overrides discovery.
Registered email/password sign-in uses the Matrix email identifier and requires
an explicit homeserver. **Check server** reports its resolved URL and advertised
password/SSO methods without submitting credentials or creating a local database.

Browser SSO uses the server's provider chooser, so custom provider IDs work.
On macOS it uses the system browser and an SDK-owned loopback callback; Cancel
aborts the task and closes the callback listener. The existing iOS authentication
sheet uses the same generic server chooser; iOS remains untested without an SDK.

Sessions restore on restart, request refresh tokens where supported, and save
rotated tokens. Token-bearing session files are replaced atomically with mode
0600 on Unix. Storage still uses Robrix's file-based session model, not Keychain.
A wrong password or unsupported SSO returns to the same editable form. Pending
sign-in disables duplicate submissions; password sign-in waits for its response.

Supported here: existing-account password and Matrix `m.login.sso`/`m.login.token`
flows, discovery, saved-session restore, refresh and expired-session handling.
Direct OAuth-only authorization, QR/device-transfer login, and native account
registration are not implemented. matrix.org advertises a compatible SSO flow;
its browser may itself use OAuth/MFA. Your actual account's browser authentication
must be completed by you. This does not claim all Matrix APIs or sign-in methods.
Servers must also support the native sliding sync required by Robrix's SDK UI.

Validation:

- `cargo test --locked --features agent_chat --lib`: includes server selection,
  malformed URLs, email identifiers, advertised methods with custom providers,
  SSO callback/token exchange, refresh-token request, and cancellation cleanup.
- `python3 tools/wechat-ux/live/native_login.py`: hidden native Makepad input;
  public matrix.org discovery, invalid URL, unsupported SSO, rejected password,
  successful Palpo fixture login and room sync, private session permissions,
  then restart into the same account/device. No real-account credentials used.
- Private run receipts and screenshots: `evidence/live/native-login.json` and
  its immutable `login-check-*` directory. Previous failed runs remain recorded.

Run `packaging/run-matrix-account.sh` on macOS to reopen the personal window.
It stores its profile at `~/Library/Application Support/Robrix Live`.

The interactive live-account profile is separate from the Palpo fixture and
runs without Makepad's remote instrumentation bridge.
