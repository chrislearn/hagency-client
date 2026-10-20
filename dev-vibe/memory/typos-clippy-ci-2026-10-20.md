# Typos 与桌面 Clippy CI（2026-10-20）

- 用户要求检查 typos/Clippy CI，缺失时补齐。起点 feeec7c2，工作区干净；此前用户授权持续将改动提交远端 main。
- 引擎 ci.yml、rust.yml 已有 18 个选定引擎包的严格 all-targets Clippy，保持原命令和阻断行为。桌面 app-main.yml 原先只有测试/build/check，新增桌面包及 miniapp-core 的 all-targets Clippy；便携编辑器/主题 job 也增加 all-targets Clippy。均明确安装 Clippy、使用 Rust 1.98.0 和 -D warnings，在运行测试前执行。
- 新增 typos.yml：main push、面向 main 的 PR、手动触发，无文档路径忽略。只读 contents 权限、5 分钟上限、按分支取消旧运行。固定 crate-ci/typos v1.51.1 的提交 7f9d0819ca1c008ade73f4b41fa9d258d48f605b，配置使用仓库 typos.toml，无自动写回。
- 以同版本官方 Windows 发布二进制实际扫描仓库，工具/缓存位于 D:/BuildCache/hagency-client。补充确切 API 名称、前缀、专有名词和哈希/UUID/编码 ID 例外；保持原有 JS/CSS/HTML 排除，排除 Git 对象和原文许可证/上游 Markdown 兼容语料。显式包含隐藏配置，禁用机器全局/父目录忽略，避免与 CI 的扫描范围不同。
- 修正实际普通拼写错误和含混的 bare-call 正则变量名。保留协议字段、fixture 字节、SQL 约束、系统 API 及上游数据，未为拼写检查更改这些约定。
- 桌面 lint 范围为所选包自身，使用 --no-deps；依赖仍编译，但不将继承的 UI 库样式当成目标包的告警。未修改现有引擎 lint 范围，也未新增 -A/allow 或降低现行 lint 级别。
- 为新门禁修复已存在的告警：冗余引用/拷贝 clone、可简化分支与迭代器、默认值初始化、类型别名；大型 UI action/worker payload 改为 Box，数据内容和持久格式不变；OAuth 测试对请求列表取快照，使 std mutex guard 不跨 await。没有放宽授权或改动 OAuth 产品流程。
- 本地通过：同款桌面/核心严格 Clippy（all-targets/-D warnings）和独立便携核心命令；完整库测试 559 passed/0 failed/2 原 ignored（桌面501、article-core21、theme7、miniapp-core30），便携核心含集成/文档的完整测试也通过；33 项 Node 自测；241 文件样式 0 违规；主题守卫；i18n 1990 项/1498 调用/0 缺失；引擎原格式命令；actionlint 与 diff --check。
- Typos 不仅全仓扫描通过，还以 D 盘临时文本实测：故意拼错时返回 2，纠正后返回 0。未将错误样例写进仓库。Cargo metadata 验证 target_directory 仍在 D:/BuildCache/hagency-client/target；未运行 Docker、实际登录或模型调用。
- 原始日志位于 D:/BuildCache/hagency-client/lint-ci-*.log；远端结果以本次 main 提交的 Spelling/App CI 为准，避免为补写验证记录重复触发构建。
