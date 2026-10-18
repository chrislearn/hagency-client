# Hagency Client 0.6.0

2026-10-18 最新指示：用户在实现和构建期间明确说“先不发布上线”。已停止正式发布；不能自动恢复 publish=true 或打标签。以下实现保留。

- Desktop 与本地引擎 workspace 版本均设为 0.6.0；独立组件保持自己的 API 版本。
- 发布入口 `.github/workflows/desktop-release.yml`，支持匹配版本的 v* 标签或 main 手动 publish=true。五平台各自测试、构建；统一核验七安装包与 SHA256SUMS 后发布 Latest，拒绝覆盖已公开 Release。
- Windows 发布构建输出、Cargo 缓存及临时文件使用 D:/BuildCache/hagency-client；metadata 实际目录检查加入 CI。发行文件名保留 hagency-desktop-*，源仓库为 hagency-org/hagency-client。
- 本机 D 盘库检查通过；actionlint 检查通过。源码 c3fabb9d99a2d9e26b4dedd17eee5787e9711bae 的 Desktop CI 39130936653通过。首轮39130756689因Ubuntu Python3.10缺少tomllib失败并取消，改用兼容的版本提取后重跑39130940057。Linux x64通过测试进入打包，其余平台构建中时按用户新指示取消，不能称七包验收完成。
- 39130940057 最终已确认completed/cancelled，全部五平台均cancelled，下游发布没有执行。GitHub API复核没有v0.6.0标签和任何Client Release。用户明确恢复前不重跑正式发布。
- 没有扩大到 Octos 上游依赖改造；历史依赖评估仍有效，在发布说明中公开链接，不能声称审计干净。签名、移动端、加密 Room 的限制保留。
