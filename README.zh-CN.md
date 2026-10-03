[English](README.md) | [中文](README.zh-CN.md)

# Hagency

**把 Codex agent 借给 Palpo Matrix 服务器上的项目，由所有者审批，按 token 预算运行。**

Hagency 是一个 Rust 服务：`hagency serve`。*运维者*（operator）指运行 Hagency、并把它的资源提供给 Palpo 的人。运维者把资源发布到已连接的 Palpo homeserver。一个资源由模型、推理档位和每月 token 上限组成。项目在这些资源上定义 agent，并申请 token。运维者批准一个额度。随后 Hagency 为该 agent 创建独立的 Matrix 身份，让它进入项目房间。成员 @ 提及 agent 来派活。所有者在一个加密的私密房间里审批 agent 的高风险操作。

本仓库包含 [native/](native/) 下的 Rust 服务，以及 [mockup/](mockup/) 下的控制台源码。控制台是一个 Next.js 应用，构建时导出为静态文件，由 `hagency serve` 提供。

**通过 Palpo 服务器使用 Hagency？** 请先读[使用指南](docs/user-guide/README.zh-CN.md)。
**要修改服务？** 请先读[代码导读](docs/architecture-walkthrough.zh-CN.md)。

## 目录

| 章节 | |
| --- | --- |
| [功能](#功能) | 能力范围 |
| [架构](#架构) | 一个进程、它的线程与 crate |
| [构建与安装](#构建与安装) | 从源码构建，作为 systemd 或 launchd 服务安装 |
| [首次运行](#首次运行) | 启动服务、连接 Palpo、批准 agent |
| [运维](#运维) | 健康检查、日志、备份、凭据轮换 |
| [配置](#配置) | 状态目录、`fleet-runtime.json` 与 `agent-driver.json` |
| [安全状况](#安全状况) | 哪些是强制的，哪些是假设 |
| [开发](#开发) | 测试与 CI 检查 |
| [文档](#文档) | 指南、设计记录与历史文档 |
| [许可证](#许可证) | Apache 2.0 与 fork 来源 |

## 功能

- **资源与目录。** 运维者在控制台配置资源。Hagency 把新资源发布到 Palpo。上限、席位和内部 id 不对外公开。
- **由项目定义 agent。** 项目成员在 Palpo 网页端基于已发布的资源定义 agent。定义中写明申请的 token 数和每日速率。每份定义都会成为一条*接洽*（engagement），即某个项目的一个 agent 的记录。它等待运维者的决定。
- **批准时分配额度。** 运维者批准一个额度，最多可选“全部剩余”。额度必须在资源上限、席位和资源池余量之内。agent 用完额度后会暂停，不会丢弃工作。运维者追加 token 后，它继续工作。
- **无需协调者（coordinator）即可创建 agent。** 对于在控制台导入的 Palpo 车队（fleet），Hagency 通过车队的 App Service 自行创建所有账号：审批机器人、每个 agent，以及 agent 与所有者的加密私聊。不需要单独的协调者 agent（ADR-187）。
- **按提及工作。** 在项目房间里，agent 只在有人 @ 提及它时行动。它把周围的讨论作为上下文，并在对话的讨论串里回复。在私聊中，所有者发来的每条消息都会送达它。
- **在其他房间工作。** agent 会加入所有者邀请它进入的房间。其他人的邀请在控制台中等待决定。在已加入的房间里，agent 遵循 [agent 在哪些房间工作](#agent-在哪些房间工作)中的规则（ADR-188）。
- **所有者审批。** 以下操作会以卡片形式发到加密的私密审批室，交给所有者：
  - 沙箱之外的命令和文件修改；
  - 协作类工具；
  - 文件收发。

  只有所有者本人已验证设备的裁决才有效。无人回应的卡片按拒绝处理。
- **一个控制台。** 同一个二进制在回环端口上提供控制台。控制台涵盖资源、账号、agent、接洽、项目方、审批、邀请、任务、用量和告警。

### agent 在哪些房间工作

| 房间 | 什么会唤醒 agent |
| --- | --- |
| 它与所有者的私聊 | 所有者的每条消息 |
| 项目房间（未加密） | 提及它的真人消息 |
| 已加入的未加密群聊房间 | 提及它的真人消息 |
| 已加入、成员只有所有者和 agent 的房间 | 所有者的每条消息，无论是否加密 |
| 已加入、有其他人的加密房间 | 无。agent 不在那里工作。 |

agent 只为所有者的设备加密。加密房间里的其他人无法读到它的回复。因此在有其他人的加密房间里，agent 保持加入状态，并发一条通知，说明它无法在此工作。只有在别人发言之后，它才会重发通知，且每 15 分钟最多一次。Hagency 每一轮都会重新检查房间。如果其他人离开，agent 就开始在那里工作。

在已加入房间里的工作消耗同一份额度。审批卡片仍然发到所有者的审批室。

## 架构

```text
Palpo homeserver  <── outbound HTTPS ──  hagency serve (127.0.0.1:13300)
                                           ├─ Palpo transport: requests, probes, catalog, statuses
                                           ├─ fleet service: provisioning loop, one approval pump per owner
                                           ├─ per agent: a driver thread (Matrix /sync, intake, replies) and an invite poller
                                           ├─ domain + custody SQLite, one writer thread each
                                           ├─ console and operator API
                                           └─ per dispatch: hagency guardian ─> codex app-server ─> hagency mcp
```

- **只有出站连接。** `hagency serve` 不对外开放任何端口。它轮询 Palpo 的车队 API，并为每个 agent 轮询 Matrix `/sync`，因此可以运行在 NAT 之后。
- **单一二进制。** 同一个可执行文件同时是：
  - 守护进程；
  - 掌管每个 runner 进程树的 guardian；
  - 为 Codex 提供任务工具的 MCP 助手；
  - 运维 CLI。
- **车队服务。** 导入 Palpo 车队后，由车队服务运行它。它创建车队的账号，为已批准的 agent 完成创建，并为每个所有者运行一个审批泵。每一步失败都会退避重试。一条接洽失败不会影响其他接洽。
- **crate。** Rust 工作区位于 [native/](native/)。主要的 crate 有：
  - `hagency-core`：领域类型；
  - `hagency-store`：持久化规则；
  - `hagency-matrix`：Matrix 副作用；
  - `hagency-palpo`：车队传输层；
  - `hagency-execution` 和 `hagency-runtime`：Codex 运行；
  - `hagency-platform`：进程监管。

  [native/README.md](native/README.md) 列出了全部 crate。

[代码导读](docs/architecture-walkthrough.zh-CN.md)按流程逐一走读代码，并附有图示。

## 构建与安装

前置条件：

| | |
| --- | --- |
| Rust | [rust-toolchain.toml](rust-toolchain.toml) 中固定的工具链 |
| Node.js 22 | 仅在构建时用于导出控制台 |
| Codex CLI | runner。运行时配置写明它的路径和 SHA-256。 |
| 主机 | 带 systemd 的 Linux，或带 launchd 的 macOS |
| Palpo | 管理员能执行 **Add Hagency** 的 homeserver |

构建二进制和控制台：

1. 构建二进制：

   ```bash
   cargo build --release --locked -p hagency
   ```

2. 安装控制台的构建依赖：

   ```bash
   (cd mockup && npm ci)
   ```

3. 把控制台导出到一个新目录：

   ```bash
   node mockup/scripts/build-native-console.mjs --output /abs/path/console-assets
   ```

   脚本拒绝已存在的目录，并以 0700 权限创建新目录。

### 导入的车队

这是新安装推荐的方式（见[服务模式](#服务模式)）。安装脚本没有导入车队的模式。可以用以下两种方式之一：

- **在前台运行。** 按[连接导入的车队](#连接导入的车队)直接运行 `hagency serve`。
- **作为服务运行。**
  1. 按[协调者安装（已有安装）](#协调者安装已有安装)运行安装脚本，不提供 `agent-driver.json`。它运行 `hagency init`，然后写入并启用单元。由于单元带 `--agent-driver` 运行，随后 `/ready` 检查失败。单元仍保留在原处。
  2. 编辑单元（Linux 上是 `/etc/systemd/system/hagency-native.service`，macOS 上是 `~/Library/LaunchAgents/io.hagency.native.plist`），删除 `--agent-driver`。
  3. 把 `fleet-runtime.json` 以 0600 权限复制到状态目录（见[配置](#配置)）。
  4. 重启服务：Linux 上运行 `systemctl daemon-reload && systemctl restart hagency-native`；macOS 上先 `launchctl bootout` 再 `launchctl bootstrap` 该 plist。

  然后从[连接导入的车队](#连接导入的车队)的第 4 步继续。导读在[已知缺口](docs/architecture-walkthrough.zh-CN.md#15-已实现尚未实现与已知缺口)中列出了这一点（“部署导入的车队”）。

### 协调者安装（已有安装）

安装脚本只部署协调者安装。已有的协调者安装按原配置继续工作。要把协调者安装设为服务，运行安装脚本：

```bash
install/install-native.sh \
  --install-dir /abs/path/bin \
  --state-dir /abs/path/state \
  --console-dir /abs/path/console-assets \
  [--config-dir /abs/path/config] [--overwrite]
```

- **步骤。** 安装脚本会：
  1. 运行 `hagency init`，它要求状态目录为空，并生成 `operator.token`；
  2. 把配置文件以 0600 权限复制到状态目录；
  3. 在 Linux 上渲染 [deploy/hagency-native.service](deploy/hagency-native.service)，在 macOS 上渲染 [deploy/io.hagency.native.plist](deploy/io.hagency.native.plist)，并启动它；
  4. 只有 `/ready` 在 60 秒内返回 200 才算成功。
- **输入。** `--install-dir` 中必须有 `hagency` 二进制。`--config-dir` 可以提供 `agent-driver.json`、`development-driver.json`、`palpo-transport.json`，以及私密的 `matrix.*`、`palpo.*` 和 `approval.*` 文件。其他文件名一律拒绝，包括 `fleet-runtime.json`。
- **服务模式。** 单元模板运行 `serve --agent-driver --palpo-transport`。这是协调者模式。没有 `agent-driver.json` 时服务无法启动，`/ready` 检查失败，安装也随之失败。
- **拒绝情形。** 已存在的单元会被拒绝，除非传入 `--overwrite`。

目前还没有发布版本。[release-native.yml](.github/workflows/release-native.yml) 只在手动触发时构建各目标平台的二进制和 `SHA256SUMS`。

## 首次运行

### 服务模式

| 模式 | `serve` 参数 | 配置 |
| --- | --- | --- |
| 导入的车队（ADR-187） | `--palpo-transport` | `fleet-runtime.json`，以及控制台导入写入的文件 |
| 协调者安装 | `--agent-driver --palpo-transport` | `agent-driver.json` 及其 `matrix.*` 和 `approval.*` 文件 |

`--agent-driver` 和 `--development-driver` 互斥。不带 `--palpo-transport` 时，控制台导入会被保存，等下次带该参数启动时生效。

### 连接导入的车队

1. 创建状态目录：

   ```bash
   hagency init --state-dir /abs/path/state
   ```

2. 把 `fleet-runtime.json` 以 0600 权限写入状态目录。参见[配置](#配置)。也可以稍后再写：车队服务会等待这个文件。
3. 启动服务：

   ```bash
   hagency serve --state-dir /abs/path/state --palpo-transport \
     --console-assets /abs/path/console-assets
   ```

4. 打开控制台：

   ```bash
   hagency console-access --state-dir /abs/path/state
   ```

   命令输出一个链接，在你生成新链接之前一直有效。打开链接后，它会换成一个 `HttpOnly` 会话 cookie；重启服务不会让你退出登录。
5. Palpo 管理员在 Palpo 网页端执行 **Add Hagency**。
6. 用拥有这个 Hagency 的账号登录 Palpo 网页端，打开 **My Hagency access**，下载 Hagency 配置。
7. 运维者在控制台打开 **Project sides → Connect a Palpo project server**，选择该文件，并填写 homeserver 的 Matrix 地址。服务无需重启即可启动 Palpo 传输。一个服务只运行一个 Palpo 车队。
8. 拥有这个 Hagency 的 Palpo 账号在 Palpo 网页端点击 **Verify connection & create reception**。车队服务随后创建车队代表的设备和本地密钥。审批机器人为每个所有者各建一个设备，在车队服务第一次为该所有者准备已批准的 agent 时创建（前提是该所有者已有交叉签名密钥）。
9. 在车队 agent 能找到的位置登录 Codex。`fleet-runtime.json` 中有 `local_codex` 时，Codex 运行时 `HOME` 为 `local_codex.home`，`CODEX_HOME` 为 `local_codex.codex_home`，因此用 `CODEX_HOME=<local_codex.codex_home> codex login` 登录。没有 `local_codex` 时，两者都设为 `<state>/runtime-home`。第 8 步之后，车队服务加载 `fleet-runtime.json` 时，如果该目录不存在，会以 0700 权限创建它。在那里登录：

   ```bash
   CODEX_HOME=/abs/path/state/runtime-home codex login
   ```

   托管账户和 `hagency account login` 只用于协调者安装。
10. 用运维 API 创建第一个资源。控制台只能复制已有资源来创建新资源，所以第一个资源无法在控制台里创建。把状态目录换成你自己的；如果改过默认监听地址 `127.0.0.1:13300`，也一并替换：

    ```bash
    curl -s -X POST http://127.0.0.1:13300/api/native/v1/resources \
      -H "Authorization: Bearer $(cat /abs/path/state/operator.token)" \
      -H 'Content-Type: application/json' \
      -d '{"presetId":"local_codex","seatId":"local_codex_seat","framework":"codex",
           "model":"gpt-5.6-sol","provider":"openai","reasoning":"medium",
           "ceiling":{"tokens":20000000,"period":"monthly"},"published":true}'
    ```

    不需要事先登记席位。响应是该资源的公开目录条目。

    - **只发布有资格的组合。** 只有当资源的 `model` 和 `reasoning` 组成的组合在 [native/hagency-core/role-capacity.json](native/hagency-core/role-capacity.json) 中至少对一个角色有资格时，Palpo 才能看到它。对 Codex 来说，这些组合是 `gpt-5.6-sol` 搭配 `low`、`medium` 或 `high`。其他组合会被保存，但不会发布。
    - **与登录匹配。** 有 `local_codex` 时，`seatId` 必须等于 `local_codex.seat`，`framework` 必须是 `codex`，`provider` 必须是 `openai` 或省略。API 不检查这一点。不匹配的资源会被接受并发布，但它的 agent 会在运维者批准后、Hagency 创建它们时被拒绝。

    有资格的资源随后会出现在 Palpo 中，项目可以在上面定义 agent。要提供其他模型、推理档位或上限，在控制台复制这个资源。
11. 在 **Engagements** 下批准申请。agent 创建完成后会加入项目房间。

[使用指南](docs/user-guide/README.zh-CN.md)从 Palpo 一侧介绍第 5 到第 8 步。

## 运维

| 任务 | 命令 |
| --- | --- |
| 检查存活 | `curl -s 127.0.0.1:13300/health` |
| 检查就绪 | `curl -s 127.0.0.1:13300/ready`。返回 503 时会列出每个未就绪的组件。 |
| 服务状态（Linux） | `systemctl status hagency-native` · `journalctl -u hagency-native` |
| 日志（macOS） | `<install-dir>/logs/hagency-native.stdout.log` 和 `…stderr.log` |
| 设置日志级别 | `RUST_LOG`（默认 `info`） |
| 停止（macOS） | `launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/io.hagency.native.plist`。被杀掉的进程会由 `KeepAlive` 重新拉起。 |
| 查看 | `hagency engagements`、`hagency resources`、`hagency alerts`（带 `--state-dir`；加 `--json` 输出原始内容） |
| 在线备份 | `hagency backup --state-dir <state> --out <new dir>` |
| 恢复 | `hagency restore --state-dir <empty dir> --from <backup>` |
| 轮换运维令牌 | `hagency rotate --state-dir <state> operator-token`。重启服务后新令牌才生效。 |

**车队服务进度。** 车队服务每次切换阶段都会记录日志 `fleet service stage`。阶段如下：

1. `awaiting_runtime_config`：缺少 `fleet-runtime.json`。
2. `awaiting_reception`：Palpo 的 **Verify connection** 还没有绑定接待房间。
3. `identities`：服务正在创建车队的账号和密钥。
4. `running`：创建循环和审批泵正在运行。

失败的阶段会退避重试。服务拒绝的配置显示为 `refused_config`。

**所有者密钥。** Hagency 第一次需要某个所有者的交叉签名主密钥时，会从 homeserver 读取该密钥，并固定（pin）在存储中。没有开启交叉签名的所有者还没有密钥，因此该所有者的 agent 会等待。已固定的密钥不会被 homeserver 之后报告的密钥替换。控制台和 CLI 目前还不提供重新固定的操作。因此，重置了交叉签名的所有者在固定的密钥被更改之前无法得到服务。即便重新固定，也修复不了已经注册的 agent：它们冻结的密钥列表仍保留旧密钥，因此在每个 agent 重新创建之前，它们发给该所有者的消息都会失败；该所有者也会得到一个新的审批设备（ADR-187 修订；见[已知缺口](docs/architecture-walkthrough.zh-CN.md#15-已实现尚未实现与已知缺口)）。

**关闭。** 收到 SIGTERM 时，服务先停止 runner、Palpo 传输和车队服务，然后关闭 Matrix 会话和数据库，最后停止 HTTP 服务。systemd 给它 20 秒。

## 配置

`hagency serve` 不读取任何环境文件。它的配置就是 `--state-dir` 中的文件：

| 文件 | 用途 |
| --- | --- |
| `operator.token` | 运维 bearer 密钥，由 `hagency init` 生成 |
| `fleet-runtime.json` | 导入车队的 Codex 运行时设置，见下文。需要手写。 |
| `palpo-transport.json`、`palpo.machine_token`、`palpo-appservice.json` | 由 Palpo 导入写入 |
| `representative.identity.json`、`matrix.representative_token`、`matrix.appservice_token`、`matrix.provisioning_key` | 车队代表和创建 agent 所用的凭据。车队服务只创建一次。 |
| `approval-<owner>.*`、`approval-sdk-<owner>/` | 每个所有者一个审批机器人设备：它的记录、令牌、身份和密钥文件，以及 SDK 存储。`<owner>` 是由所有者 Matrix ID 派生的标识。在车队服务第一次为该所有者准备已批准的 agent 时创建（前提是该所有者已有交叉签名密钥）。 |
| `agent-driver.json`、`matrix.*`、`approval.*` | 仅用于协调者安装：runner、Matrix 和审批机器人设置 |
| `agent-matrix-provision_<engagement>/` | 每个 agent 一个：它的加密创建记录和房间记录，以及它的 Matrix SDK 存储 |
| `runtime-home/` | `fleet-runtime.json` 中没有 `local_codex` 块时，Codex 的 `HOME` 和 `CODEX_HOME`；存放 Codex 登录 |
| `fleet-workspace/` | 车队执行主机的私密占位工作区；不会有派发在其中运行 |
| `factory-task-contexts/` | 预热任务桥使用的每次派发的任务上下文文件 |
| `console-logins.json` | 控制台访问链接和登录的 SHA-256 哈希，因此重启不会让你退出登录 |
| `domain.sqlite3`、`custody.sqlite3` | 持久状态（WAL，`synchronous=FULL`）。每个状态目录只能有一个进程使用。 |

配置文件必须仅所有者可读写（权限 0600）。`fleet-runtime.json` 和 `agent-driver.json` 拒绝未知字段。

`fleet-runtime.json` 的字段：

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `profile` | 是 | 必须是 `palpo_fleet_runtime_v1` |
| `executable` | 是 | Codex 二进制：不含符号链接的绝对路径 |
| `executable_sha256` | 是 | 该二进制的 SHA-256，64 个小写十六进制字符 |
| `local_codex` | 否 | 绑定运维者自己的 Codex 登录（子字段见下）。不设置时，Codex 使用 `<state>/runtime-home`。 |
| `file_limit` | 是 | 文件工具的文件大小上限，单位字节：1 到 4,194,304。即使两个文件工具都关闭，也会检查。 |
| `operation_ms` | 是 | 每次派发的操作预算，100 到 1,200,000 毫秒。不得小于 `approval_owner_wait_ms` + 5000。 |
| `response_ms` | 是 | 每次派发的响应预算，10 到 2000 毫秒 |
| `idle_ms` | 是 | 预热的运行时可以空闲多久，范围 100 到 1,200,000 毫秒 |
| `approval_owner_wait_ms` | 否 | 卡片等待所有者多久后按拒绝处理。默认值为 1000 毫秒，所以请设置它。加上 5000 毫秒的回复预留后不得超过 600,000 毫秒。 |
| `send_file`、`receive_file` | 否 | 启用文件工具。默认 `false`。 |
| `coordination_tools` | 否 | 启用委派和对等工具。每次调用都需所有者批准。默认 `false`。 |
| `matrix_request_interval_ms`、`matrix_sdk_timeout_ms` | 否 | Matrix 请求节流，以及 SDK 预算（10 到 60,000 毫秒） |
| `home` | 是 | agent 主目录，含三个必填子字段：`root`，一个已存在、仅所有者可访问的目录；`task_client`，`hagency` 二进制的绝对路径；`projects`，最多 16 项，每项含 `project_id`、`source` 和 `mode`（`copy` 或 `symlink`）。`projects` 可以是 `[]`：没有对应条目的项目会得到一个不复制源码的主目录。 |

`local_codex` 的子字段，出现该块时全部必填：

| 字段 | 含义 |
| --- | --- |
| `profile` | 必须是 `provider_owned_codex_v1` |
| `preset` | 一个资源 preset id，记录在这个登录所服务的每个认领上 |
| `seat` | 这个登录服务的席位 id。`seatId` 与之相同的 Codex 资源在这个登录下运行。 |
| `home` | Codex 的 `HOME` 目录 |
| `codex_home` | Codex 的 `CODEX_HOME` 目录，存放 Codex 登录 |

`home` 和 `codex_home` 必须是不含符号链接的绝对路径，属于服务用户，且组和其他用户不可写。

一个最小示例。每个 `/srv/hagency/...` 路径都是占位符，请换成你自己的路径：

```json
{
  "profile": "palpo_fleet_runtime_v1",
  "executable": "/srv/hagency/bin/codex",
  "executable_sha256": "<64 个小写十六进制字符：codex 二进制的 shasum -a 256>",
  "local_codex": {
    "profile": "provider_owned_codex_v1",
    "preset": "local_codex",
    "seat": "local_codex_seat",
    "home": "/srv/hagency/codex-user",
    "codex_home": "/srv/hagency/codex-user/.codex"
  },
  "file_limit": 4194304,
  "operation_ms": 600000,
  "response_ms": 2000,
  "approval_owner_wait_ms": 300000,
  "idle_ms": 600000,
  "home": {
    "root": "/srv/hagency/agent-homes",
    "task_client": "/srv/hagency/bin/hagency",
    "projects": []
  }
}
```

这里不出现 Matrix 身份。它们来自导入的车队。

`agent-driver.json` 用于配置协调者安装。它与上表共用 Codex、预算和文件相关字段，另外增加 `workspaces`、`matrix` 和 `approval` 身份块，以及 `factory_service`。

权威定义是 [native/hagency/src/bootstrap/config.rs](native/hagency/src/bootstrap/config.rs) 中的 `FleetRuntimeConfig` 和 `Config`。

## 安全状况

Hagency **强制执行**的：

- **仅限回环地址。** `hagency serve` 拒绝任何非回环的监听地址。远程访问请使用 SSH 隧道，或发送 `Host: 127.0.0.1:13300` 且不添加任何 `Forwarded` 或 `X-Forwarded-*` 头的反向代理。
- **控制台请求。** 控制台要求完全匹配的 `Host` 头，并拒绝转发类请求头。写操作需要同源的 `Origin`。运维 API 同样要求完全匹配的 `Host`，并拒绝 `Forwarded`、`X-Forwarded-For`、`Origin` 和 `Sec-Fetch-Site`，然后以常数时间比较 bearer 令牌的 SHA-256。
- **所有者审批。** 审批只来自所有者已验证的设备。审批在一个加密房间中进行，房间成员只有所有者和审批机器人。出现第三个成员或失去加密时，该房间会被停用。投递失败或等待超时都按拒绝处理。
- **每个所有者一个审批设备。** 每个所有者的审批机器人设备只信任该所有者。不同所有者的审批相互隔离。
- **受隔离的 runner。** 每次派发都有一个能力凭证和一个隔离编号（fence）。过期 runner 的调用会被拒绝。只有证明其进程树已经消失后，回复才会发布。
- **Codex 沙箱。** Codex 以 `workspace-write` 和 `on-request` 审批运行，没有网络，只有自己的工作区可写。Hagency 会检查 Codex 回显的设置。
- **凭据只写不读。** 没有任何路由会返回已存储的令牌。Palpo 导入的响应只包含公开信息。
- **加固的单元。** systemd 单元设置了 `NoNewPrivileges`、`ProtectSystem=full`、空的 capability 集合和系统调用过滤。

Hagency **假设**的：

- **主机内共享信任。** 回环信任覆盖整台机器，所以任何本地进程都能访问该端口。请保持 `operator.token` 和状态目录私密。
- **所有者密钥首次使用即信任。** 如果 homeserver 在 Hagency 第一次读取所有者密钥时作假，Hagency 会信任错误的密钥。之后 homeserver 的回答不会替换已固定的密钥。
- **群聊房间对成员开放。** 任何已加入的成员只要 @ 提及 agent，就能给它派活。房间成员资格、额度和所有者审批是控制手段。
- **加密的多人房间无法使用。** agent 不能在有所有者以外其他人的加密房间里工作。
- **不使用联邦。** 所有成员都必须在车队自己的服务器上。
- **沙箱资格验证尚未完成。** Hagency 会请求并检查沙箱，但各操作系统上沙箱实际效果的资格验证仍未完成。

## 开发

运行与 CI 相同的检查：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
node native/scripts/check-rust-spec-bindings.mjs
node --test native/scripts/check-production-callers.test.mjs
node native/scripts/check-production-callers.mjs
cargo test --workspace --all-targets --locked --no-fail-fast -- \
  --skip native_codex_real_app_server \
  --skip native_two_agent_qualification_records_its_evidence
```

- `check-rust-spec-bindings.mjs` 检查每个规格选择器都对应一个真实的测试。
- `check-production-callers.mjs` 检查每一行 `Production caller:` 都能在生产调用图中找到。`check-production-callers.test.mjs` 测试这个检查器本身。
- 跳过的两个测试需要真实的、已通过资格验证的主机（ADR-140、ADR-144）。它们仍留在规格中，绑定检查照常覆盖它们。

[specs/](specs/) 中的任务契约把行为绑定到测试。[knowledge/decisions/](knowledge/decisions/) 记录了背后的决策。

以下两个工作流运行这些检查：
- [ci.yml](.github/workflows/ci.yml) 在 Ubuntu 上运行上述检查，覆盖拉取请求、推送到 `main` 和手动触发。只改动 `docs/`、`knowledge/` 或两份 README 的变更不会触发它。
- [rust.yml](.github/workflows/rust.yml) 增加 macOS 和 Windows、控制台浏览器测试和发布构建。它每晚运行，也在手动触发和推送 `nv*` 标签时运行；带 `full-ci` 标签或改动其所列路径的拉取请求也会触发它。

导读中的[如何做改动](docs/architecture-walkthrough.zh-CN.md#16-如何做改动)一节说明新规则、迁移、agent 工具和控制台路由应放在哪里。

## 文档

| 文档 | 内容 |
| --- | --- |
| [docs/user-guide/README.zh-CN.md](docs/user-guide/README.zh-CN.md) | 连接 Palpo、房间、谁能和 agent 对话、token |
| [docs/architecture-walkthrough.zh-CN.md](docs/architecture-walkthrough.zh-CN.md) | 按流程走读代码 |
| [native/README.md](native/README.md) | Rust 工作区：crate、命令、测试（英文） |
| [knowledge/decisions/](knowledge/decisions/) | 架构决策记录 |
| [specs/](specs/) | 绑定到测试的任务契约 |
| [docs/guides/](docs/guides/) | 中文指南：Matrix 对话、文件、agent 状态、Palpo 出站传输 |
| [docs/history/](docs/history/) | 已移除的 TypeScript 产品的设计文档，仅作历史保留（大部分为英文） |
| [docs/LICENSING.md](docs/LICENSING.md) | fork 来源与 Apache 2.0 署名义务 |

## 许可证

**Apache License 2.0**：见 [`LICENSE`](LICENSE) 和 [`NOTICE`](NOTICE)。

Hagency 是 [agent-chat](https://github.com/shisuiki/agent-chat) 的一个 fork，上游于 2026-07-29 采用 Apache 2.0。`NOTICE` 记载了上游作者。再分发时请保留 `NOTICE` 和 `LICENSE`，并标明你修改过的文件。见 [docs/LICENSING.md](docs/LICENSING.md)。
