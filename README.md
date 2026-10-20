[English](README.md) | [中文](README.zh-CN.md)

# Hagency client

[Hagency](https://github.com/hagency-org/hagency-client) is a native desktop
application for Windows, macOS and Linux. It combines Matrix chat, Project
workspaces and locally executed AI Agents. Download it from
[Client Releases](https://github.com/hagency-org/hagency-client/releases/latest).

Sign in to your Hagency server through Pasion, then manage Projects and Agents
inside the application. See the [desktop quick start](crates/hagency-app/docs/hagency-quickstart.md)
and [application guide](crates/hagency-app/README.md).

The application and its local Rust engine share one Cargo workspace. The default
package is `hagency-app`, whose executable is `hagency`. `crates/hagency` is an
engine library used directly in process. The former CLI Web console, static
prototype pages and headless service installer have been removed. The desktop's
loopback OAuth callback remains; it exposes no management pages or APIs.

## Development

On Windows, build outputs, dependency caches and temporary files stay on D:.
The wrapper configures them and verifies Cargo's actual target directory:

```powershell
./tools/dev.ps1
./tools/dev.ps1 check --locked -p hagency-app --lib
./tools/dev.ps1 test --locked -p hagency-app --lib
```

On macOS/Linux, follow the [native dependencies guide](crates/hagency-app/docs/local-development.md)
and run `cargo run --locked -p hagency-app --bin hagency`. No Node page build or
separate local console process is required. See [workspace and packaging](docs/workspace.md).

The historical `hagency-org/hagency-desktop` repository preserves earlier sources
and releases; current development belongs here. Historical Fleet and CLI documents
record retired behavior and are not current setup instructions.

## License

Apache 2.0. Retain [LICENSE](LICENSE) and [NOTICE](NOTICE); see [licensing](docs/LICENSING.md).
