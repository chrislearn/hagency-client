# 关于页链接与产品资料

2026-10-20：用户提供 About 页截图，指定官网为 https://hagency.org，要求核对其余链接、修改错误并补齐缺项。

- 按旧 Desktop 迁移记忆，在当前 Client 的 `crates/hagency-app` 修改；旧 `hagency-desktop` 源码仓库没有改动。起点 44d3cdf4，工作区干净。
- 新增 `src/project_links.rs` 集中管理官网、源码、隐私、许可、NOTICE、指南、版本发布和反馈链接。官网为 https://hagency.org/；源码使用 package repository。OAuth 客户端主页也使用同一官网。
- About 的旧隐私路径 `/blob/main/docs/privacy.md` 不存在，改到实际的 `/blob/main/crates/hagency-app/docs/privacy.md`。新增许可证、致谢、使用指南及下载入口，沿用共享按钮和自动换行布局，补齐四项英文/简体中文翻译。保留 Robrix 上游归属。
- GitHub API 确认 `hagency-client` 为 fork，has_issues=false，当前凭据无 admin/maintain；没有尝试修改远端设置。`hagency-desktop` 的 has_issues=true、archived=false，暂用其 Issues 作为反馈入口，并在新增 docs/support.md 明确当前源码仍在 Client。后续若管理员启用 Client Issues 或归档旧仓库，应同步更新集中链接、AppStream 和反馈文档。
- 应用版本截图中的 50fa85d6 提交通过 GitHub commits API 确认存在，当前版本链接算法无需变更。注册表来源的 Matrix SDK 版本链接改为 docs.rs 对应版本 API 文档，0.19.1 页面实测 200；Git 来源的精确提交链接保留。
- 更新英文隐私资料并补充中文版本，说明 Hagency/Pasion 登录、本地账号/加密/缓存/工作目录/账本、数据目录及兼容别名、模型/工具/语音数据流、小应用权限、加密房间 Agent 限制及公开诊断注意事项。支持文档补齐源码、下载、指南、反馈、许可证与第三方归属入口；中英应用 README 加入官网、支持和隐私链接。
- Cargo/packager 的 homepage、documentation 与 publisher，Windows CompanyName、AppStream 的主页/反馈/帮助/开发者和产品描述同步修正。移除旧安装包描述中任意 Matrix 登录、完整移动端能力、无网页引擎和不实性能宣称，改为当前桌面能力。历史版权和许可文本未改。
- Debian Source/Contact、AUR 验证默认仓库、release secrets 示例与上传脚本默认仓库改为现行地址。只做源码修改及语法/夹具验证，没有执行凭据上传、Docker 构建、实际问题提交或发布。
- 11 个公开 URL 全部实测 200：官网根/英文/中文、Client 源码/发布、隐私/指南/LICENSE/NOTICE、Desktop issues/new、Matrix SDK docs。issues/new 正常跳转 GitHub 登录页，不表示已经提交问题。结果：`D:/BuildCache/hagency-client/about-links-audit.json`。新中文隐私与支持文档目前仅在工作区，尚未推送上线。
- Rust 1.98.0 严格 Clippy 通过：`cargo +1.98.0 clippy --no-deps --locked -p hagency-app --features agent_chat,octosense-module --all-targets -- -D warnings`。完整应用库测试同 features：501 passed、0 failed、2 原 ignored。日志 about-links-clippy.log / about-links-tests.log 位于 D:/BuildCache/hagency-client。
- i18n 1994 项、1502 调用、0 缺失；242 源文件样式 0 违规；主题守卫、打包/发布资产 5 项测试、修改脚本 bash -n、typos 1.51.1、XML/TOML 解析、本地文档链接、diff --check 均通过。新增 project_links.rs 与 build.rs 的 Rust 1.98.0 rustfmt 检查通过。没有新增镜像实现的链接单元测试。
- Cargo metadata 确认实际 target_directory=D:/BuildCache/hagency-client/target；Cargo/cache/tmp 均使用该任务 D 盘目录。现有 Cargo duplicate/unused patch 提示保留。未修改 Cargo.lock、schema 或用户配置；没有重启用户正在运行的桌面应用、实际屏幕验收、提交或推送。

2026-10-20 后续：用户明确要求将 Server 与 Client 两轮改动分别提交并推送远端 main。提交前 fetch 确认与 origin/main 无分叉；typos 和 diff 检查再次通过。推送后的 CI 以本次提交的 GitHub Actions 为准，状态记录保存在 D:/BuildCache/hagency-client/about-links-push-ci-*，不为追加 CI 记录重复触发完整构建。
