# 联系人右侧详情

更新：2026-10-22。

- 用户从 Cloud 聊天提供桌面联系人截图，要求点击联系人后在当前页面右侧显示详情，不再切换整个页面。实际实现位于合仓后的 Client `crates/hagency-app/src/home/mobile.rs`。
- `MobileHub` 改为 `master_pane`、分隔线和 `detail_pane`。桌面联系人列表固定 300px，右侧占剩余空间；未选中时右侧留空，点击后显示联系人卡片和原有消息、朋友圈、复制链接、屏蔽操作。列表保持可见、搜索条件保留，按用户 ID 高亮当前选中行。
- 详情组件从列表的 `PageFlip` 移出，由左右区域共用同一份实例；窄窗口隐藏列表、详情全宽，详情标题的返回按钮恢复列表。发现和个人页面继续使用完整宽度。
- 新增真实 Makepad 注册/绘制与分组点击动作回归：桌面连续切换与搜索过滤后选择、右侧位置和宽度、清理账号后隐藏详情；窄窗口全宽详情及返回。加原有新聊天测试共 3 项通过。主题字面量、翻译检查（2219 条，0 缺失）及 `git diff --check` 通过。
- 同一已编译测试程序的 `home::` 回归共 93 项通过，包括桌面联系人导航、主页面响应式布局、侧栏与聊天详情布局。
- 编译通过仓库 `tools/dev.ps1` 执行，脚本通过 `cargo metadata --no-deps --format-version 1` 确认 `target_directory=D:/BuildCache/hagency-client/target`，Cargo home 和临时目录亦在 D 盘。
- 本次修改仅涉及上述源码和本记忆；仓库已有其他任务的未提交变更，未覆盖。未提交、推送、发布、重建正式程序或重启用户正在运行的客户端；自动化布局检查不等于真实窗口目测。

后续用户要求重新编译并重启查看：已向旧 PID71356 正常请求关闭，日志确认窗口几何与应用状态保存成功，再结束其托盘驻留。通过 `tools/dev.ps1 build --locked --offline -p hagency-app --bin hagency` 在 D 盘构建成功（1m27s），沿用 `HAGENCY_APP_DATA_DIR=D:/BuildCache/ui-review-data` 启动新版 PID73060，主窗口句柄非零。构建及启动日志为 `D:/BuildCache/hagency-client/contacts-detail-build.log`、`contacts-detail-app.stdout.log`、`contacts-detail-app.stderr.log`。未提交、推送或发布安装包。

用户实窗确认效果可以，并明确要求提交推送远端 main。本次提交仅包含联系人分栏源码及本记忆，其他任务的未提交改动不纳入。
