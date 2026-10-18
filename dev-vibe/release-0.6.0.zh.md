# Hagency Client 0.6.0

2026-10-18：用户明确要求发布 0.6.0，并将官网下载链接部署上线，取代之前 0.5.0 的暂停状态。

- Desktop 与本地引擎 workspace 版本均设为 0.6.0；独立组件保持自己的 API 版本。
- 发布入口 `.github/workflows/desktop-release.yml`，支持匹配版本的 v* 标签或 main 手动 publish=true。五平台各自测试、构建；统一核验七安装包与 SHA256SUMS 后发布 Latest，拒绝覆盖已公开 Release。
- Windows 发布构建输出、Cargo 缓存及临时文件使用 D:/BuildCache/hagency-client；metadata 实际目录检查加入 CI。发行文件名保留 hagency-desktop-*，源仓库为 hagency-org/hagency-client。
- 本机 D 盘 locked 库检查通过；actionlint 检查通过。远端安装包运行与最终发布结果待补记。
- 没有扩大到 Octos 上游依赖改造；历史依赖评估仍有效，在发布说明中公开链接，不能声称审计干净。签名、移动端、加密 Room 的限制保留。
