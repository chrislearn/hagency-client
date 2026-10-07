[English](local-development.md) | [中文](local-development.zh-CN.md)

# 客户端与服务器本地联调

使用 `hagency-client` 和 `hagency-server` 两个源码仓库。两边分别使用各自指定的
Rust 工具链。客户端控制台构建需要 Node.js 22+，HTTP 服务由 Rust 提供。

## 服务器

首次初始化按服务器 README 配置，包括 Pasion 资源和管理员账号。后续启动：

```sh
just db-up
just dev
```

本地地址为 `http://127.0.0.1:8088`，管理登录在 `/login`，Pasion 注册在
`/_pasion/register`。私人配置位于 `config/dev/` 下的 `hagency.toml`、
`palpo.toml`、`pasion.toml`。三个 PostgreSQL 数据库分别为 `hagency`、`palpo`、
`pasion`；Compose 提供的 PostgreSQL 监听 `127.0.0.1:55438`。

客户端自助接入要求 `hagency.toml` 的 `[fleet_access]` 中
`allow_self_service = true`。本地测试注册可以在 `pasion.toml` 配置
`[experimental] fixed_verification_code = "123456"`，邮件和短信 provider
使用 `blackhole`。

## 客户端

```sh
just init-dev  # 首次安装控制台构建依赖
just dev
```

开发监视器构建静态控制台和 Rust 程序，按需初始化私人状态目录，并在
`127.0.0.1:13300` 启动客户端，开启到服务器的出站通信。另开一个终端：

```sh
just console
```

打开终端打印的私人登录链接，不要将链接保存到日志。服务器登录卡片填写
`http://127.0.0.1:8088` 和安装名称，再通过 Pasion 登录。
客户端自动创建或复用自己的 Fleet、导入配置，并验证真实 Matrix 事件的接收回执。
不需要手工下载配置，也不要求客户端有公网地址。

控制台地址为 `http://127.0.0.1:13300/console/`。状态、SQLite 数据库和私人机器凭据
保存在 `.run/dev-state/`，控制台构建产物位于 `.run/dev-console-*`。这些目录均已
被 Git 忽略。服务器绑定和安装身份在重启后保留；Pasion 浏览器会话只在内存中，
Rust 服务重启后需要重新登录。

修改 Rust 或控制台源码后自动重建并重启客户端，构建失败时保留原来的运行服务。
服务器的 `just dev` 同样监视 Rust、前端和组件配置。各终端按 `Ctrl+C` 停止。
自定义客户端状态目录和端口：

```sh
just dev --state-dir .run/another-client --listen 127.0.0.1:13301
just console .run/another-client 127.0.0.1:13301
```

如果 Google Fonts 不可访问，使用 `just dev --font-cache /path/to/.next`
复用此前成功构建的真实字体文件。默认缓存位置为 `.run/font-cache`，包含此前
构建的 `static/chunks`、`static/css`（如果存在）和 `static/media` 目录。

两端互联不会自动配置编程代理或发布模型资源。需要运行 Agent 时，再在客户端
配置实际使用的 Codex/Claude 运行环境和资源配额。
