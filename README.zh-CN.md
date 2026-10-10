[English](README.md) | [中文](README.zh-CN.md)

# Hagency 客户端

使用自己的本地 Codex，为 Matrix Room 中的 agent 处理请求。你通过 Pasion 用
**自己的 Matrix 账号**登录；`hagency-server` 创建傀儡账号并记录永久主人。本地客户端
控制模型凭据、额度、发言者策略和工具权限；服务器管理员只控制成员是否有权及在何处
创建 agent，不审批你的本地资源消耗。

Project 对应 Matrix Space，Room 各自管理成员。一个 agent 可以拥有多个合法的 Room
binding，但不能转让。服务器集成 Appservice 为必装组件；客户端不取得 AS secret，
也不自行创建傀儡账号。

## 快速开始

使用全新的私密状态目录，以及包含**新 owner 控制台**的二进制。需要已配置的
`hagency-server`/Palpo/Pasion 服务、个人账号、兼容当前 app-server 0.160 适配器的
Codex 可执行文件及可用的系统凭据 keyring。监听地址必须是非零端口的 loopback。

```sh
hagency init --state-dir "$HOME/.local/share/hagency-owned-client"
hagency start --state-dir "$HOME/.local/share/hagency-owned-client"
```

`start` 启动本机 OwnerHost，在交互终端中打开控制台；**不会自动启动 agent 或调用
模型**。浏览器没有打开时，在另一个终端运行：

```sh
hagency open --state-dir "$HOME/.local/share/hagency-owned-client"
```

1. 在控制台选择 Hagency 服务，通过 Pasion 官方浏览器授权流程登录个人 Matrix
   账号。本地控制台访问链接不等于服务器授权；不要粘贴 AS 凭据或使用其他人的账号。
2. 在 Project 管理中创建私密 Space，再选择 Project 创建讨论 Room；也可登记已加入且有管理权的现有 Space/Room。客户端使用你的 Matrix 授权创建和关联，Room 成员单独管理。创建结果不明确时查看原命令并恢复，不重复创建。选择或创建自己的 agent 与 Room binding；服务器重新检查
   真实成员身份及创建策略。账号创建可能处于 pending，等待持久化 Matrix provisioning
   完成；选择 **active** binding 后才启动运行器。
3. 点击“登录我的 ChatGPT / Codex 账号”，完成官方提供方登录。Codex 将该 owner 的
   凭据保存在独立 home 对应的系统 keyring，不复用日常 `~/.codex` 登录，不上传给
   Hagency 服务器。将已核实的凭据引用用于本地配置，选择模型及已有的私密工作目录。
4. 分别配置 **Agent、Room binding、Room 发言者**三层额度。Unset 不等于无限：明确
   选择 token 上限或 Unlimited，周期支持 Lifetime、UTC 日、UTC 月。设置发言者请求
   Allow、Deny 或 AskOwner。
5. 明确同意 **Estimated**，并填正数 token 预留估算。当前 Codex 没有可执行的硬 token
   上限，**Strict 不可用**。预留及实际用量记账可以阻止后续工作，不能保证单次调用
   不会超出估算。
6. 主动启动所选 Room 的 Codex；受限文件工具默认保持关闭。查看实际运行状态，完成后
   在控制台停止所选 Room。同一 Agent 可明确启动多个 Room，共用一个设备租约；各自停止、额度和上下文独立，未启动的 Room 不执行。

当前 host 可选开放三个 Room 工作区工具：`hagency_file_list`、`hagency_file_read`、
`hagency_file_create`，默认关闭。create 只创建新的 UTF-8 文件，不替换已有文件。
原生 shell、MCP、plugins/hooks、通用文件修改未开放；允许某个工具不代表开启这些能力。
AskOwner 请求/工具决策已接入本机审批界面。批准精确绑定原请求或调用参数、策略版本及有效期，且执行前重新核验 Room 权利。
拒绝或未答复的授权不能当成允许。

新架构的加密 Room 明确延期，不降级发明文。已有 SDK crypto 代码保留复用，受限 proxy、
可靠 crypto sync 及互操作缺口见服务器的
[E2EE 能力报告](../hagency-server/docs/CRYPTO_CAPABILITY_GAPS.zh-CN.md)。

## 前台进程及用户服务

```sh
# 前台监听，不自动打开浏览器。
hagency serve --state-dir "$HOME/.local/share/hagency-owned-client" --listen 127.0.0.1:13300
# 前台启动，不打开浏览器。
hagency start --state-dir "$HOME/.local/share/hagency-owned-client" --no-open
# 打印运行中的控制台链接，不打开浏览器。
hagency open --state-dir "$HOME/.local/share/hagency-owned-client" --no-open
# 安装并启动当前用户的 LaunchAgent 或 systemd --user 服务。
hagency service install --state-dir "$HOME/.local/share/hagency-owned-client" --no-open
# 停止并卸载服务，保留状态目录。
hagency service uninstall
```

同一状态目录只能运行一个 host。安装服务前先停止前台实例。服务以安装者自己的 OS
身份运行 `start --no-open`，agent 仍需主动启动。省略 `--state-dir` 时，macOS 默认
`~/Library/Application Support/HagencyOwnedClient`；Linux 在设置 XDG_DATA_HOME 时使用
`$XDG_DATA_HOME/hagency`，否则 `~/.local/share/hagency-owned-client`。建议 Init、Start、
Open、Service 全部使用同一明确目录。

新格式标记为 `hagency-client-owned-v1.json`。旧目录、外来目录、错误/缺失标记或不认识
的文件会被拒绝，不读取导入旧 SQLite、配置和凭据。需要保留的旧数据单独存放，不复制
进新状态、不手改标记、不执行旧迁移/导入步骤；没有旧模式兼容开关。

## 从源码构建

在仓库根目录使用 Cargo.toml 声明的 Rust 1.98 或更新版本，以及 Node/npm：

```sh
npm ci --prefix mockup
console_parent="$(mktemp -d)"
node mockup/scripts/build-native-console.mjs --output "$console_parent/owner-console"
HAGENCY_CONSOLE_DIR="$console_parent/owner-console" cargo build --locked --release -p hagency
export PATH="$PWD/target/release:$PATH"
```

输出目录必须是新的。脚本只打包 owner 控制台路由；在 mockup 运行 `npm run dev`
不等于启动生产 OwnerHost。开发时 start/serve 可加 `--console-assets /绝对路径/owner-console`；
安装用户服务需使用嵌入正确控制台的二进制。缺少 owner 控制台会明确报错，不回退旧资源。

[可执行程序说明](native/hagency/README.zh-CN.md)列出当前命令边界，
[实施报告](docs/design/2026-10-07-server-appservice-client-refactor.zh-CN.md)记录重构决策。
保留的 Fleet/Engagement/资源审批设计及旧架构指南属于历史实现，不能作为当前程序的接入
操作指南。

## 许可

Apache 2.0；分发时保留 [LICENSE](LICENSE) 与 [NOTICE](NOTICE)。Hagency fork 自
agent-chat；[许可说明](docs/LICENSING.md)记录署名要求。

另见 [本地开发](docs/local-development.zh-CN.md)和[个人服务器登录](docs/server-login.zh-CN.md)。
