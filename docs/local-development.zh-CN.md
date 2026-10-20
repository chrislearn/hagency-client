# 桌面客户端与服务端本地开发

Client 和 Server 保持为相邻仓库。桌面端位于 `crates/hagency-app`，构建与缓存要求见[工作区指南](workspace.md)。
Windows 使用 `./tools/dev.ps1`，macOS/Linux 的依赖和启动见[原生开发指南](../crates/hagency-app/docs/local-development.md)。

服务端环境见[Server README](../../hagency-server/README.md)。在桌面登录页输入自己的服务地址并完成 Pasion 授权。
桌面在随机 loopback 端口接收 OAuth 回调；不再运行 CLI 管理服务器或构建网页。
测试登录协议可运行 `./tools/dev.ps1 test --locked -p hagency-app --lib login::oauth_tests`。
