[English](README.md) | [中文](README.zh-CN.md)

# Hagency

**把 Codex agent 借给 Palpo Matrix 服务器上的项目，并由所有者审批、按 token 预算运行。**

Hagency 是一个 Rust 服务：`hagency serve`。提供方把资源（模型、思考强度、每月 token 上限）发布到已连接的 Palpo homeserver。项目在这些资源上定义 agent 并申请 token；提供方批准一个额度后，Hagency 为该 agent 创建独立的 Matrix 身份，让它进入项目房间。之后成员 @ 提及 agent 来派活，并在一个加密的私密房间里审批它的高风险操作。

**通过 Palpo 服务器使用 Hagency？** 请先读[使用指南](docs/user-guide/README.zh-CN.md)。
**要修改服务？** 请先读[代码导读](docs/architecture-walkthrough.zh-CN.md)。

## 目录

| 章节 | |
| --- | --- |
| [功能](#功能) | 能力范围 |
| [架构](#架构) | 一个进程、它的线程与 crate |
| [构建与安装](#构建与安装) | 从源码构建，作为 systemd 或 launchd 服务安装 |
| [首次运行](#首次运行) | 控制台访问与连接 Palpo |
| [运维](#运维) | 健康检查、日志、备份、凭据轮换 |
| [配置](#配置) | 状态目录与 `agent-driver.json` |
| [安全状况](#安全状况) | 哪些是强制的，哪些是假设 |
| [开发](#开发) | 测试与 CI 检查 |

## 功能

- **资源与目录。** 提供方在控制台配置资源，新资源会自动发布到 Palpo；上限、席位和内部 id 不对外公开。
- **由项目定义 agent。** 项目成员在 Palpo 网页端基于已发布的资源定义 agent，并填写申请的 token 数和每日速率。每份定义都会成为一条等待提供方决定的接洽。
- **批准时分配额度。** 提供方在资源上限、席位和资源池余量之内批准一个额度，也可以选“全部剩余”。agent 用完额度后会暂停，不会丢弃工作；追加 token 后继续。
- **自动创建。** 批准后会通过 App Service 创建 agent 的 Matrix 账号，并让它加入项目房间。同时还会创建它与所有者的加密私聊，并先上传 agent 的密钥，再邀请所有者。
- **按提及工作。** 在项目房间里，agent 只在被人提及时行动；它会把周围的讨论作为上下文，并在对话的讨论串里回复。在私聊中，所有者发来的每条消息都会送达它。
- **所有者审批。** 沙箱之外的命令和文件修改、协作类工具以及文件收发，都会以卡片形式发到加密的私密审批室，交给所有者。只有所有者本人已验证设备的裁决才有效；无人回应的卡片按拒绝处理。
- **一个控制台。** 资源、接洽、项目方、审批、邀请、任务、用量和告警，都由同一个二进制在回环端口上提供。

## 架构

```text
Palpo homeserver  <── outbound HTTPS ──  hagency serve (127.0.0.1:13300)
                                           ├─ Palpo long poll: requests, probes, catalog, statuses
                                           ├─ per-agent driver threads: Matrix /sync, intake, replies
                                           ├─ domain + custody SQLite, one writer thread each
                                           ├─ console and operator API
                                           └─ per dispatch: hagency guardian ─> codex app-server ─> hagency mcp
```

- **只有出站连接。** `hagency serve` 不对外开放任何端口；它长轮询 Palpo 的 fleet API，并为每个 agent 轮询 Matrix `/sync`，因此可以运行在 NAT 之后。
- **单一二进制。** 同一个可执行文件既是守护进程，也是掌管每个 runner 进程树的 guardian、为 Codex 提供任务工具的 MCP 助手，还是运维 CLI。
- **crate。** Rust 工作区位于 [native/](native/)。`hagency-core` 承载领域类型，`hagency-store` 承载持久化规则，`hagency-matrix` 承载 Matrix 副作用，`hagency-palpo` 是 fleet 传输层，`hagency-execution` 和 `hagency-runtime` 负责 Codex 运行，`hagency-platform` 负责进程监管。

[代码导读](docs/architecture-walkthrough.zh-CN.md)按流程逐一走读代码，并附有图示。

## 构建与安装

前置条件：

| | |
| --- | --- |
| Rust | [rust-toolchain.toml](rust-toolchain.toml) 中固定的工具链 |
| Node.js 22 | 仅在构建时用于导出控制台 |
| Codex CLI | runner；`agent-driver.json` 写明它的路径和 SHA-256 |
| 主机 | 带 systemd 的 Linux，或带 launchd 的 macOS |
| Palpo | 管理员可以执行 “Add Hagency” 的 homeserver |

构建二进制和控制台：

```bash
cargo build --release --locked -p hagency
(cd mockup && npm ci)
node mockup/scripts/build-native-console.mjs --output /abs/path/console-assets
```

控制台的输出目录必须是新建的，且只属于你本人。

安装为服务：

```bash
install/install-native.sh \
  --install-dir /abs/path/bin \
  --state-dir /abs/path/state \
  --console-dir /abs/path/console-assets \
  --config-dir /abs/path/config
```

- **它做什么。** 安装脚本先运行 `hagency init`，它要求状态目录为空，并生成 `operator.token`。随后把配置文件以 0600 权限复制进状态目录，再渲染并启动服务：Linux 上是 [deploy/hagency-native.service](deploy/hagency-native.service)，macOS 上是 [deploy/io.hagency.native.plist](deploy/io.hagency.native.plist)。只有 `/ready` 返回 200 时才算成功。
- **输入。** `--install-dir` 中必须有 `hagency` 二进制。`--config-dir` 提供 `agent-driver.json`，以及私有的 `matrix.*`、`palpo.*` 和 `approval.*` 文件。
- **拒绝情形。** 如果 unit 已存在，除非加上 `--overwrite`，否则会拒绝。

原生服务目前还没有正式发布的版本。[release-native.yml](.github/workflows/release-native.yml) 只在手动触发时构建各平台二进制和 `SHA256SUMS`。

## 首次运行

1. 打开控制台：

   ```bash
   hagency console-access --state-dir /abs/path/state
   ```

   它会打印一个 120 秒内有效的链接；打开后会换成一个 `HttpOnly` 会话 cookie。
2. 连接 Palpo，按[使用指南](docs/user-guide/README.zh-CN.md)操作：
   - Palpo 管理员执行 **Add Hagency**。
   - 所有者下载配置文件，并在 **项目方 → 连接 Palpo 项目服务器** 中导入。
   - 所有者再到 Palpo 网页端验证连接。
3. 在控制台配置资源。资源会出现在 Palpo 中，项目可以在其上定义 agent。
4. 在 **接洽** 页面批准申请。创建完成后，agent 会加入项目房间。

## 运维

| 任务 | 命令 |
| --- | --- |
| 存活 / 就绪 | `curl -s 127.0.0.1:13300/health` · `curl -s 127.0.0.1:13300/ready`（返回 503 时会写明未就绪的组件） |
| 服务状态（Linux） | `systemctl status hagency-native` · `journalctl -u hagency-native` |
| 日志（macOS） | `<install-dir>/logs/hagency-native.stdout.log`、`…stderr.log` |
| 日志级别 | `RUST_LOG`（默认 `info`） |
| 停止（macOS） | `launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/io.hagency.native.plist`（被 kill 的进程会被 `KeepAlive` 重新拉起） |
| 查看 | `hagency engagements`、`hagency resources`、`hagency alerts`（`--state-dir`、`--json`） |
| 在线备份 | `hagency backup --state-dir <state> --out <new dir>` |
| 恢复 | `hagency restore --state-dir <empty dir> --from <backup>` |
| 轮换运维令牌 | `hagency rotate --state-dir <state> operator-token` |

收到 SIGTERM 后，服务按顺序收尾（fleet、runner、Matrix 会话，最后是数据库），HTTP 服务器最后停止。systemd 给它 20 秒。

## 配置

`hagency serve` 不读取环境变量文件。它的配置就是 `--state-dir` 中的文件：

| 文件 | 用途 |
| --- | --- |
| `operator.token` | 运维 bearer 密钥，由 `hagency init` 生成 |
| `agent-driver.json` | runner 与 Matrix 设置，见下文 |
| `matrix.*`、`approval.*` | agent 身份与审批机器人身份的访问令牌、SDK 存储密钥和 CA 证书 |
| `palpo-transport.json`、`palpo.machine_token`、`palpo-appservice.json` | 所有者导入 Palpo 配置时写入 |
| `domain.sqlite3`、`custody.sqlite3` | 持久状态（WAL、`synchronous=FULL`）；一个状态目录只能由一个进程打开 |

`agent-driver.json` 是严格 JSON，未知字段会被拒绝。主要字段：

| 字段 | 含义 |
| --- | --- |
| `profile` | 驱动配置类型 |
| `executable`、`executable_sha256` | Codex 二进制及其预期哈希 |
| `workspaces` | 工作区 id → 绝对路径 |
| `operation_ms`、`response_ms` | 每个派发的操作预算和响应预算 |
| `approval_owner_wait_ms` | 卡片等待所有者答复多久后按拒绝处理。默认 1000 毫秒，请务必设置 |
| `send_file`、`receive_file`、`file_limit` | 开启文件工具，以及接收文件的大小上限 |
| `coordination_tools` | 开启委派与 agent 间协作工具，每次调用都需所有者批准 |
| `matrix`、`approval` | agent 与审批机器人的 homeserver 地址、身份、设备和房间 |
| `factory_service` | 负责为已批准 agent 执行创建的 coordinator |

权威定义见 [native/hagency/src/bootstrap/config.rs](native/hagency/src/bootstrap/config.rs) 中的 `Config`。

## 安全状况

Hagency **强制**的：

- **只监听回环地址。** `hagency serve` 拒绝任何非回环的监听地址。需要远程访问时，请用 SSH 隧道或反向代理。
- **控制台请求。** 控制台要求请求带正确的 `Host`，写操作带同源的 `Origin`，并且不带转发类请求头。运维 API 使用以常量时间比对的 bearer 令牌。
- **所有者审批。** 审批只接受所有者本人已验证设备发出的裁决，且必须在成员只有所有者和审批机器人的加密房间里。一旦出现第三名成员或失去加密，该房间即被停用。送达失败或等待超时都按拒绝处理。
- **runner 设栅栏。** 每个派发都有自己的凭证和栅栏编号。过期 runner 的调用会被拒绝，并且只有在证明其进程树已全部退出后，回复才会发布。
- **Codex 沙箱。** Codex 以 `workspace-write` 和 `on-request` 审批运行，禁用网络，不额外开放可写目录。Codex 回显的设置会被校验。
- **凭据只写不读。** 没有任何路由会返回已保存的令牌；控制台只显示指纹。
- **加固的 unit。** systemd unit 设置了 `NoNewPrivileges`、`ProtectSystem=full`、空的 capability 集合和系统调用过滤。

它所**假设**的：

- **同机信任。** 回环信任以整台机器为范围，任何本机进程都能访问该端口。请确保 `operator.token` 和状态目录只有你本人可读。
- **群聊对成员开放。** 任何已加入的成员提及 agent 都能给它派活。控制手段是房间成员、额度和所有者审批。
- **不启用联邦。** 所有成员都必须在 fleet 自己的服务器上。
- **沙箱资格验证未完成。** 沙箱设置已请求并校验，但其在各操作系统上实际效果的资格验证尚未完成。

## 开发

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

行为通过 [specs/](specs/) 中的任务契约与测试绑定，相应决策记录在 [knowledge/decisions/](knowledge/decisions/) 中。CI（[rust.yml](.github/workflows/rust.yml)）还会运行：
- `node native/scripts/check-rust-spec-bindings.mjs`：每个契约选择器都对应真实存在的测试。
- `node native/scripts/check-production-callers.mjs`：每一行 `Production caller:` 都能在生产调用图中找到。
- 控制台浏览器测试。

代码导读中的[如何修改](docs/architecture-walkthrough.zh-CN.md#14-如何修改)一节说明新规则、迁移、agent 工具和控制台路由应放在哪里。

## 文档

| 文档 | 内容 |
| --- | --- |
| [docs/user-guide/README.zh-CN.md](docs/user-guide/README.zh-CN.md) | 连接 Palpo、房间、谁能和 agent 对话、token |
| [docs/architecture-walkthrough.zh-CN.md](docs/architecture-walkthrough.zh-CN.md) | 按流程走读代码 |
| [knowledge/decisions/](knowledge/decisions/) | 架构决策记录 |
| [specs/](specs/) | 与测试绑定的任务契约 |
| [docs/LICENSING.md](docs/LICENSING.md) | fork 来源与 Apache 2.0 署名义务 |

## 许可证

**Apache License 2.0**：见 [`LICENSE`](LICENSE) 和 [`NOTICE`](NOTICE)。

Hagency 是 [agent-chat](https://github.com/shisuiki/agent-chat) 的 fork，后者于 2026-07-29 采用 Apache 2.0。`NOTICE` 记录了上游作者。再分发时请保留它和 `LICENSE`，并标注你修改过的文件。见 [docs/LICENSING.md](docs/LICENSING.md)。
