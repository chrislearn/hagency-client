# Local desktop and server development

Keep Client and Server as sibling checkouts. The desktop is in `crates/hagency-app`;
see [workspace builds](workspace.md) and [native dependencies](../crates/hagency-app/docs/local-development.md).
On Windows use `./tools/dev.ps1`; all outputs, caches and temporary files stay on D:.

Configure the server with [its README](../../hagency-server/README.md). Enter its
origin in the desktop login page and complete Pasion authorization. The desktop
receives the OAuth redirect on an ephemeral loopback callback; no CLI management
server or page build is needed. OAuth integration tests run with
`./tools/dev.ps1 test --locked -p hagency-app --lib login::oauth_tests`.
