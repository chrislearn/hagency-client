[English](README.md) | [中文](README.zh-CN.md)

# owner 客户端可执行程序

此 crate 构建 `hagency`。生产入口为 `owner_host::OwnerHost`，使用个人 Pasion 授权，
由主人明确启动本地 Codex。[根目录 quickstart](../../README.zh-CN.md)说明构建、登录和
策略配置步骤。

| 命令 | 实际行为与参数 |
| --- | --- |
| `init --state-dir PATH` | 初始化新的私密 owner 状态格式，必须指定目录 |
| `start` | 服务控制台，交互终端中打开浏览器；支持 `--state-dir`、`--listen`、`--no-open`、`--console-assets` |
| `serve` | 只服务控制台，不打开浏览器；支持 `--state-dir`、`--listen`、`--console-assets` |
| `open` | 通过私密本地 IPC 向运行中的 host 取得控制台访问链接；支持 `--state-dir`、`--no-open` |
| `service install` | 安装/启动用户级 macOS LaunchAgent 或 Linux systemd user 服务；支持 `--state-dir`、`--listen`、`--no-open` |
| `service uninstall` | 停止/卸载服务，保留状态 |

未指定子命令时执行 start，默认监听 `127.0.0.1:13300`。拒绝非 loopback 和零端口。
一个状态目录只允许一个进程持锁，前台 host 与用户服务不能并行占用同一目录。
安装服务运行 `start --no-open`，不会自动调用模型；agent 需经过登录、配置并主动启动。

新状态使用 `hagency-client-owned-v1.json`。旧 Fleet SQLite、配置、错误标记和外来文件
直接拒绝，不读入迁移。不要手改标记来复用旧目录。没有 setup、旧 enrollment、authority
import、guardian/MCP 子命令及兼容开关。其他位置保留的历史模块不是生产 CLI 入口。

控制台通过构建时的 `HAGENCY_CONSOLE_DIR` 嵌入，或 start/serve 的 `--console-assets`
提供。必须是 `mockup/scripts/build-native-console.mjs` 生成并通过 manifest 检查的 owner
bundle；缺失或旧 bundle 会报错，不回退。service install 没有 console-assets 参数，使用
正确嵌入资源的 release 二进制。

控制台访问和服务器授权是两层边界。使用个人 Matrix 账号通过 Pasion OAuth/PKCE 登录。
服务器记录 agent 永久主人，AS 凭据不发给客户端。Codex onboarding 使用独立 owner 的
系统 keyring/home，不导入日常 Codex 登录，不向服务器上传提供方凭据。

进入客户端先登录自己的 Matrix 账号。登录页可选择历史账号及服务器，也可添加其他账号；
设备名称自动设置。切换会停止当前账号任务并要求重新 Pasion 认证。各账号的配额、账本、
恢复记录与 Codex 资源独立保存，Agent 永远属于原创建者。历史服务器只记录地址，
不记录密码或 token。已登录时可从“我的 Matrix 登录”切换或退出，登录不会启动推理。

Agent、Room binding、发言者三层额度共同生效。当前 Codex 仅支持主动同意的 Estimated
预留/实际用量记账；Strict 硬上限明确拒绝。工具默认关闭。当前 host 可选文件工具仅在
所选 Room 工作区 list/read/create 新 UTF-8 文件，不开放 shell、MCP、hooks/plugins 或
替换现有文件。AskOwner 请求/工具决策通过本机界面精确批准一次请求或调用；批准后仍核验当前策略与 Room 权利。

加密 Room 延期，见服务器
[能力缺口报告](../../../hagency-server/docs/CRYPTO_CAPABILITY_GAPS.zh-CN.md)。
未知执行、未知用量及不确定回复需恢复；重启或取得新 lease 不会授权重跑旧任务，也不会
抹掉其费用记录。
