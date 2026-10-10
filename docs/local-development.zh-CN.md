[English](local-development.md) | [中文](local-development.zh-CN.md)

# owner client 与服务器本地联调

`hagency-client`、`hagency-server`、可选的 `hagency-desktop` 放在同一父目录，
各自使用仓库指定的 Rust 工具链。owner 控制台构建需要 Node.js 22+ 与 npm；
本机 HTTP 服务由 Rust 提供。下面客户端命令在本仓库根目录运行。

## 服务器前置条件

按[server 安装说明](../../hagency-server/README.zh-CN.md)首次配置；之后在 server
仓库运行 `just db-up` 和 `just dev`。默认 origin 为 `http://127.0.0.1:8088`，
Web 管理入口 `/login`。必须使用 Pasion 委托 Matrix 认证与集成 Agent Appservice。
不再需要 Fleet 注册或 `[fleet_access]`。参考
[部署排障](../../hagency-server/docs/LOCAL_DEPLOYMENT.zh-CN.md)与
[隔离测试](../../hagency-server/docs/TESTING.zh-CN.md)。

## 构建当前 owner host

```sh
npm ci --prefix mockup
mkdir -p .run
console_parent="$(mktemp -d "$PWD/.run/owner-console.XXXXXX")"
node mockup/scripts/build-native-console.mjs --output "$console_parent/assets"
HAGENCY_CONSOLE_DIR="$console_parent/assets" cargo build --locked -p hagency
# 仅首次初始化：选择全新的 owner 格式状态目录。
target/debug/hagency init --state-dir "$PWD/.run/owner-dev-state"
target/debug/hagency start --state-dir "$PWD/.run/owner-dev-state" \
  --listen 127.0.0.1:13300 --console-assets "$console_parent/assets"
```

assets 输出路径必须尚不存在，运行期间保留资源目录。后续启动跳过 `init`，
另一个终端获取私人访问链接：

```sh
target/debug/hagency open --state-dir "$PWD/.run/owner-dev-state"
```

`open` 通过本机 IPC 访问运行中的 host，不需要 listener 参数。访问链接不要写入日志。
仅前台服务使用 `start --no-open` 或 `serve`，传入同一状态、listener、assets。
同一状态目录只能有一个 host，listener 必须为非零 loopback。

保留的 `just dev` / `just console` 尚未适配当前 owner host：
`native/scripts/dev.mjs` 仍传入已移除的 `--palpo-transport` 并检查旧 `operator.token`；
`just console` 调用已移除的 `console-access`。适配前使用上面的显式命令。
`just init-dev` 仍可作为 npm 安装依赖快捷方式。`mockup` 中的 `npm run dev`
是设计预览，不是实际 OwnerHost。

控制台修改后重新导出 owner assets，使用新 bundle 重启 host；Rust 修改后重新构建。
字体无法下载时，已有验证的缓存可用控制台 builder 的
`--font-cache /absolute/path/to/cache` 选项复用；这是构建输入，不是认证或运行状态。

## 登录与配置

输入 server origin 完成个人 Pasion PKCE 登录。Project 对应一个 Space，讨论 Room
保持独立成员关系。创建/采纳有权限的 Space/Room，选择 Agent 与活动绑定，配置
主人专用 Codex 凭据、模型、私人目录、Agent/Room/请求者预算及精确工具策略。
显式接受 Estimated 计费并设置正 reservation 后再启动范围，登录/连接本身不执行模型。

CLI host 使用独立主人 Codex home/keyring，Desktop 还允许显式关联本机已登录的
Codex 账号；都不向 server 上传模型凭据。见[登录说明](server-login.zh-CN.md)、
[根指南](../README.zh-CN.md)和
[Desktop 开发](../../hagency-desktop/docs/local-development.zh-CN.md)。

新 CLI 状态使用 `hagency-client-owned-v1.json`。旧 Fleet 状态单独保留，不复制
旧配置/SQLite/token 或修改 marker。未知执行/费用保留恢复 hold。Agent 加密范围
暂缓支持，Desktop 普通人类 Matrix 加密聊天是另一条路径。

## 验证

```sh
cargo test --locked -p hagency --lib
cargo clippy --locked -p hagency --all-targets -- -D warnings
git diff --check
```

这是本地 engine 检查；完整 Pasion/Matrix fixture 与 Desktop 原生验收见上面专用指南。
实际 Codex 执行需要显式启动并消耗模型资源，协议 fixture 不能记为推理通过。
