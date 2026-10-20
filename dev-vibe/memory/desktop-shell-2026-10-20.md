# Windows 启动、托盘与本机设置

更新：2026-10-20。

- 用户从 Cloud 聊天要求桌面应用启动、最小化后的右下角图标、开机启动和对应设置。先读取旧 Desktop 迁移记忆，实际代码修改在本 Client 的 crates/hagency-app；Cloud 与旧 Desktop 未作本轮代码修改。
- 新增 desktop_integration：独立 Windows 托盘图标，点击恢复，右键显示主窗口/设置/退出；Explorer 的 TaskbarCreated 后重建图标，创建或恢复失败时保留可见窗口，不把用户困在后台。直接使用 windows 0.61.3 的 Win32 API，嵌入现有派生品牌 PNG，不修改图标源。
- 设置 → 通用 → 启动与系统托盘：Windows 登录启动、启动后隐藏默认关闭；最小化到托盘、关闭到托盘默认开启。后三项原子写入本机 app_data_dir/desktop-settings.json，不依赖 Matrix 登录，不随退出账号重置。开机启动读取/修改 HKCU Run 的 Hagency 项，正确引用带空格的 exe 路径，失败通知；保留系统 Startup apps 的禁用权并提供设置入口。未为用户的真实安装开启开机启动。
- Windows 独立 main 用按数据目录区分的本会话 mutex 防止重复实例，再次启动广播唤回已有窗口；guardian 入口先于 mutex 分支，嵌入宿主不创建托盘或领取实例锁。macOS/Linux 仍维持原有行为，隐藏 Windows 专属设置。
- 注意 Makepad 自绘关闭按钮不经过 WindowCloseRequested，而直接生成 CloseWindow。App 在该按钮的 MouseUp/TouchUpdate 命中后先保存状态、投递隐藏并结束该次指针事件转发，防止默认按钮随后生成销毁操作；Alt+F4/WM_CLOSE 则通过 accept_close=false。没有访问 Makepad 私有 platform_ops，也没有丢弃整个异步 Actions 批次。托盘退出仍走原有 Shutdown 清理。
- 现有 settings_parity 离线夹具接入同一适配器。新增默认配置兼容、隔离注册表启停、真实 Win32 消息/图标解码和窄窗口设置布局测试。默认测试不依赖 Explorer；交互测试另行验证真实注册与窗口操作。
- Rust 1.98.0 最终验证：完整应用库 505 passed / 0 failed / 2 原 ignored；agent_chat,octosense-module 的 all-targets 严格 Clippy 通过；正式程序与 settings_parity 构建通过；i18n 2007 项/1516 调用/0 缺失，主题守卫、typos、diff --check 通过。Cargo metadata 确认 target_directory=D:/BuildCache/hagency-client/target，Cargo/cache/tmp 与测试数据均在 D 盘。
- 使用 computer-use 在独立空数据目录检查：中文四项设置显示正确，启动后隐藏开关成功保存；正式程序最小化、标题栏关闭、Alt+F4 后进程保留且 MainWindowHandle=0；再次启动保持同一 PID 并恢复窗口；start_hidden=true 的新进程初始隐藏，再次启动可恢复；close_to_tray=false 时 Alt+F4 正常执行 Shutdown 并退出。全部测试实例已停止或正常退出，未登录、发送消息或修改真实账号数据。
- 日志位于 D:/BuildCache/hagency-client/tray-*.log，GUI 对照使用复制的正式二进制且 SHA256 与构建结果一致。未执行真实 Windows 重启、Explorer 重启或安装包发布；托盘菜单由源代码和消息路径检查，未声称完成实际菜单点击验收。无提交或推送。

后续：用户明确要求 git push 到远端 main。提交前 fetch 确认 main 与 origin/main 无分叉；diff、typos、翻译和主题守卫再次通过。本轮变更整体提交推送至 hagency-org/hagency-client 的 main；跨平台远端验证以该提交触发的 CI 为准，不视为发布安装包。
