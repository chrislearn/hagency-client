# Desktop 合仓与统一 crates 工作区

日期：2026-10-18。

按用户要求将原 hagency-desktop 合入 hagency-client，所有 Rust package 集中在根
`crates/`。Desktop 保留 `rinx` package 和二进制名，产品名称、数据目录与账号格式不变。
现有 Web Owner 控制台继续保留；本次不扩展为 Web 退役或正式安装包发布。

## 结构和历史

- 原 Desktop `52d88b2020a322d9c1883b412e74968c1a60ce01` 通过保留完整历史的 subtree merge 导入。
- `crates/hagency-desktop`：桌面界面、资源、应用、平台打包与相关工具。
- 18 个原 Client 引擎 package 从 `native/` 移到 `crates/`。
- Desktop 的 6 个子库、miniapp-package 工具及 2 个本地第三方库移到根 `crates/`。
- 共 28 个 package，统一 workspace 和根 Cargo.lock，移除嵌套 workspace 与独立锁文件。
- `native/scripts` 改为 `tools/native`，公共向量改为 `tests/fixtures`；Desktop 协作记录保存在 `dev-vibe/desktop`。
- Desktop 引擎依赖改为 `../hagency`，不再从 Git 下载同一引擎的第二份源码。

## 构建和发布

根 workspace 集中管理 patches、profiles、Makepad 环境及 macOS runner。默认成员为
Desktop；引擎 CI 显式选择原有 18 个 package，保持无 GUI 检查的独立性。Desktop 的主 CI、
五平台编译、七安装包发布及手动工作流迁移到根 `.github/workflows/desktop-*.yml`。

Desktop `build.rs` 从根锁文件和 Git 历史读取版本。修正共享测试素材与库间的相对路径。
资源收集和 Octos 构建通过 Cargo metadata 使用实际 target 目录。新增
`tools/dev.ps1`，将 Windows 构建、Cargo/npm 缓存和临时目录放在
`D:/BuildCache/hagency-client`，并在调用 Cargo 前核对实际输出位置。

统一锁文件保留 Desktop 的 Rustls 0.23.46、webpki 0.103.15、AWS-LC 1.18.2/sys 0.46.0、
Quinn-proto 0.11.18；tokio-rustls 按引擎精确约束统一为 0.26.4。随包 Octos 的独立
上游锁文件和既有发布审计不因合仓而变成已解决。

根 rusqlite 依赖显式启用 `fallible_uint`，使存储层的无符号计数器在单独选择 crate
时也能使用检查转换，不再隐式依赖 Matrix SDK 的 feature 合并。

历史安装包继续从原仓库下载，新仓库未正式发布安装包前不改 Cloud 下载链接。
原仓库迁移说明应解释：界面和引擎共同演进，合仓消除跨仓库提交配对、重复 CI 与版本漂移，
后续开发和问题反馈转到 chrislearn/hagency-client，原源码历史和 Releases 保留。

## 验证

- locked Cargo metadata：28 个 package，全部在 `crates/`，Desktop 使用本地引擎；target 位于 D 盘任务缓存。
- Windows `cargo check --locked -p rinx --lib` 通过。
- 32 项生产调用图和 spec 脚本回归通过，实际当前产品调用图 4 个入口均接通。
- 静态 Rust include 文件 249 处全部存在。
- 翻译 1823 个条目、1432 处调用检查通过；主题、真实图标检查通过。
- Octos 打包 6 项、外部 target 资源收集 2 项回归通过。
- 根 GitHub Actions 工作流通过 actionlint 1.7.12 语法检查。
- Windows Desktop 原生库单元测试 456 通过，2 忽略；12 个共享和引擎 package 的 47 组测试共 288 通过，41 忽略。
- 实际调用 Windows `tools/dev.ps1` 的 `-p` 参数转发与无默认特性 Desktop 库检查通过；存储层独立 `cargo check --locked -p hagency-store --lib` 通过。
- Windows 全量引擎集成测试无法作为通过结论：`ops` 引用了仅 Unix 提供的 `stop_clean`，两个相关文件与导入前逐字一致；其余 Windows 全量探测也出现原有绝对 Unix 路径及平台执行测试失败，已停止无效的平台全量探测，没有将失败或忽略项记为通过。
- 预合并手动 CI 的 Linux 严格 Clippy、格式、编译测试清单及调用图检查通过；Owner Web 实际浏览器检查通过。合并后自动检查启动，已取消旧分支的两次手动运行，避免与主分支重复占用 runner；不把手动运行记为整套成功。

## 远端迁移

[PR #2](https://github.com/chrislearn/hagency-client/pull/2) 已合并到 Client main，
合并提交 `0622ce8aad384fc21efd98f1661552c5b96f979d`，保留 Desktop 原始历史。
本地 Client 已切回 main 并同步该提交。

原 Desktop 的中英文 README 和 MIGRATION.md 已通过提交
`cd616d95749af8560a90743c316da679de578077` 推送至 main，说明迁移原因、新地址和
历史安装包保留安排。随后执行 `gh repo archive hagency-org/hagency-desktop --yes`，
GitHub 明确拒绝：`chrislearn does not have the correct permissions to execute ArchiveRepository`。
实际 API 核对 `archived=false`，当前账号只有 push/WRITE，无 admin/maintain；
归档本身仍需原仓库管理员完成，不能声称已归档。

合并后的检查：CI `39128551810`、Desktop CI `39128551856`、五平台构建
`39128551885`。这些运行不发布安装包。
