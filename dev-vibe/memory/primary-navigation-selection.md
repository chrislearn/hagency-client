# 主菜单选中背景

更新：2026-10-22。

- 用户从 Cloud 聊天提供桌面截图，要求最左侧主菜单选中背景更明显。实际代码在合仓后的 Client `crates/hagency-app`。
- `theme.rs` 新增 `RINX_NAV_SELECTED`：原 `selected` 与当前主题 `accent` 混合 30%，随浅色/深色、青色/紫色及主题重应用解析。`home/rooms_sidebar.rs` 的 `DesktopNavButton` 使用该背景；展开和折叠主导航共用此样式。
- 保留文字/图标、悬停、布局和选择逻辑；聊天/项目上下文列表继续使用原 `RINX_SELECTED`。
- 验证：现有侧栏布局/导航测试 9 项、明暗/双色主题检查 1 项通过，主题字面量守卫与 `git diff --check` 通过。Cargo metadata 确认 target 为 `D:/BuildCache/hagency-client/target`，Cargo home 与临时目录也在 D 盘。
- 本轮只修改源码并编译测试，未重建或重启正式桌面程序，未做实际窗口目测，未提交/推送或发布安装包。

后续用户要求重新编译并重启查看：已请求旧 PID 47364 正常关闭并核对状态保存日志，结束其托盘驻留后，通过 `tools/dev.ps1 build --locked --offline -p hagency-app --bin hagency` 构建成功。继续使用 `HAGENCY_APP_DATA_DIR=D:/BuildCache/ui-review-data` 启动新版 PID 20404，确认 Makepad 主窗口可见、原账号和窗口状态恢复，stderr 为空。日志在 `D:/BuildCache/hagency-client/nav-selection-{build,app.stdout,app.stderr}.log`。

用户随后确认效果可以，并明确要求提交推送至 `hagency-org/hagency-client` 的远端 `main`。此次提交范围仅为主菜单选中背景及本记忆，不发布安装包。
