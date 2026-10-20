# Client 自动更新考察

更新：2026-10-20。

- 用户从 Cloud 聊天要求考察 Client 检查更新、自动下载及安装的实现难度和工作量，仅授权调研。
- 当前实际产品位于本仓库 crates/hagency-app，基线 f5f62ffb，Rust/Makepad，0.6.0。已有五目标七安装包发布流程，无应用 updater；About 仅打开 Releases。优先验证 cargo-packager-updater 与 pinned cargo-packager 0.10.1，兼容性尚未实测。
- 建议自动检查、可选全量后台下载、用户点击重启安装；Agent 运行时延后安装。Windows 5–8 人日，跨 Windows/macOS/Linux AppImage 10–15 人日累计，含升级验收，证书等待另计。DEB/AUR 仅提示通过相应安装渠道升级。
- 候选 updater 0.2.3 在 Windows 安装分支直接 process::exit(0)，调用前必须完成可确认成功或失败的退出屏障；现有 App::handle_shutdown 超时仅日志后继续，不能原样视为安装安全屏障。关闭到托盘不等于退出。完整包还含 Octos 和资源。
- CI 需新增更新清单、签名、macOS .app.tar.gz 及附件收集。历史 secrets signing 字段未被当前 release workflow 消费，变量名也需按选定版本核实。包签名与系统代码签名不同。
- 数据库拒绝较新 schema；自动回滚不能只换旧二进制。旧版无 updater，首次需手动安装；历史包名称与 publisher 变化需真实升级验证。
- 详细依据、拆项、验收和来源：../client-auto-update-feasibility-2026-10-20.zh.md。本轮只有评估文档，无代码修改、编译、真实安装或发布。

2026-10-20 后续落地：Windows 自动更新和登录页独立更新弹窗已实现，PR https://github.com/hagency-org/hagency-client/pull/3 已按用户要求 squash 合并到 main，提交 bbcc0b39037774400b0bdabbbe8af4072b247780；本地 main 已 fast-forward，同名工作分支保留。应用 508 passed/2 原忽略，完整远端 CI 尚未全部通过，Windows Full matrix 有既有 Unix-only 测试辅助代码警告。

macOS/Linux 后续：当前没有应用内更新，macOS 发布 DMG，Linux 发布 deb/AppImage 并有 AUR 发布脚本。2026-10-20 查阅 Sparkle/AppImage/Debian/Flatpak 官方文档后，已在 D:/BuildCache/hagency-client/issues/macos-update-body.txt 和 linux-update-body.txt 写好两条评估 issue 正文。macOS 优先验证 Sparkle 2 桥接、嵌套签名/公证和 Agent 安装互斥；Linux AppImage 完整包自更新、系统包沿用各自渠道。当前保守估算每端 5–8 人日，共享状态机适配一次性 1–2 人日；签名或仓库建设另计。

创建 issue 的实际阻塞：Client 仓库 has_issues=false；当前 chrislearn 有 push/triage，无 admin/maintain，gh repo edit --enable-issues=true 返回 HTTP 404，仓库设置未改变。已向用户发异步选择，等待确认建到上游 hagency-org/hagency-rs，或由用户先启用 Client Issues；未经选择未在上游创建。issue 正文已可审阅，不需要重新研究或重复合并。
