# Desktop 0.5.0 发布

更新：2026-10-16。

- 用户确认发布目标为 hagency-desktop，要求版本 0.5.0，取代继承的 1.1.2；Cloud 不修改版本。
- hagency 依赖改为 https://github.com/chrislearn/hagency-client.git 的 main 分支；Cargo.lock 锁定 39e990c8d483d6bd29b4135742f486e7b83ad137，14 个引擎 crate 均来自 Git。删除 sibling checkout CI action 和全部调用，构建仍用 --locked。
- Windows cargo check --locked --lib 通过；i18n 缺失 0、主题检查通过。五平台测试及七安装包由 GitHub release.yml 的 publish=true 手动运行验收，全部成功后自动打标签和发布，不能把 dispatch 当成发布成功。
- 发布说明在 packaging/release-notes.md，说明功能、版本回退及 Agent 加密 Room、签名和移动端限制。发布显式 --latest，保证低于 1.1.2 的 0.5.0 仍成为下载入口。
- Cloud 新增 docs-tenant/{en,zh}/agents.md 和 downloads.md，包含七个官方 latest/download 安装包链接；其发布进度在 Cloud dev-vibe/memory/desktop-downloads-and-agents.md 记录。

首轮安装包运行38832134391的macOS ARM64编译和459单测通过，但打包app失败：cargo-packager0.10.1 No matching IconType。此前品牌改动将普通1024 PNG加入icons但未列现成ICNS；上游create_icns_file对1024@1x转换报错，显式ICNS会优先原样复制。现把resources/icon.icns列入Cargo packager icons（Linux仅取PNG、Windows优先ICO，不受影响），新增tools/check-release-icons.py及CI／发布前置校验。Windows locked库编译与真实图标输入检查通过。必须用修复提交重新完整构建后发布，不能重用旧提交混合附件。
