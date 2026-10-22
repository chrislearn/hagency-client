# Mini apps 窗口生命周期排查

日期：2026-10-22。

- 当前桌面代码在 `crates/hagency-app`，不是已移除的 hagency-desktop 目录。MiniAppsWindowHost 是 Root 下按需创建的独立原生窗口。
- Windows 默认 `close_to_tray=true`。App 的自绘标题栏关闭拦截和 WindowCloseRequested 隐藏分支原来都跳过辅助窗口清理，所以主窗口隐藏后 Mini apps 仍留在桌面。
- 本轮将清理集中到 App::close_secondary_windows；标题栏关闭到托盘、关闭请求和实际 WindowClosed 共用。关闭请求先保存状态并清理辅助窗口，再决定是否拒绝销毁主窗口并隐藏到托盘。
- 初始现场 PID 44080 使用 `HAGENCY_APP_DATA_DIR=D:/BuildCache/ui-review-data`；残留 Mini apps 内的 App Hub/返回点击没有切换页面，但 Alt+F4 可以关闭。按同一测试 profile 再启动可唤回原 PID 的主窗口。重建 Mini apps 后导入按钮恢复，再关闭主窗口到托盘后返回也正常。因此无法把最初的输入故障直接归因于主窗口隐藏，旧窗口异常状态的具体来源尚未证明。
- 排查中默认 profile 的新实例与测试 profile 实例相互独立，不能把启动默认实例当作恢复原测试实例。未操作账号认证或发送消息。
- 新增离线测试覆盖动态窗口的面板打开/目录动作，以及主窗口关闭请求/实际关闭后面板停止接收排队动作。构建、测试缓存与临时文件使用 tools/dev.ps1 的 D:/BuildCache/hagency-client，脚本用 cargo metadata 校验 target_directory。
- 完整应用库测试 565 passed / 0 failed / 2 ignored；随后将关闭测试加强为先证明面板可以响应动作，再证明关闭后忽略动作，该测试单独重编译运行 1 passed / 0 failed。修复未替换正在运行的旧二进制。用户随后要求提交并推送到 origin/main。
- 排查结束关闭原测试实例的 Mini apps 并恢复其主窗口。额外启动的默认 profile 对照实例先通过正常关闭路径保存状态，再清理该次启动的进程。
