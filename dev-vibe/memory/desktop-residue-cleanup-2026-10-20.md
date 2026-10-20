# 桌面清理二次审计（2026-10-20）

- 用户要求继续清理残留 CLI、Web 控制台引用和无用依赖，并明确将所有其他未提交改动一并提交远端 main。此授权覆盖上一轮暂不提交工具链/格式配置和验证记录的决定。
- 删除无生产或测试调用方的 operator_cli、inspect、console/client 三个网络 CLI 适配模块，以及 bootstrap/accounts、bootstrap/registration 离线 CLI 包装器；移除 side_registration::run_cli。保留实际 SDK/API 注册、账号、存储规则及 Agent 执行逻辑。
- probe 仅删除 CLI 文件/标准输入读取及其专用 Matrix HTTP 绑定器；palpo_work 和协议测试使用的 receipt_from_event、decide、ProbeReceipt、ProbeError 保留。共享操作枚举不再派生 Clap 解析，不提供命令行参数属性。
- 引擎删除未使用的 cap-fs-ext 直接依赖、冗余 Matrix base/sqlite 开发依赖；这些加密功能仍由实际使用者 hagency-matrix 完整声明。Clap（手动资格验证示例）与 tracing-subscriber（日志脱敏测试）移到开发依赖。rinx-miniapp-core 删除未直接使用的 serde。Cargo.lock 仅删去这四条直接依赖边，无版本升级。
- robius-use-makepad 的平台配置作用、NAPI 宏依赖、桌面 Matrix 参数解析及第三方 Codex/ACP CLI 都有现存用途，不因文本扫描缺少直接标识符而删除。
- 架构导读（中英）、用户指南、引擎说明重写为桌面入口；旧控制台计划改为退役指引。清除规格中已删除源文件/现行浏览器资格声明；历史条件标明退役，只保留退休选择器名称。未执行过的旧注册计划记录为历史未实现义务，未改成通过或曾存在的测试。现行 Test/Filter 列表与 HEAD 逐项一致。
- 历史审计/归属与兼容向量保留原始来源，显式标记 CLI/Web 操作已退役。不能把所有历史字样替换成不存在的源码或删除合法归属。
- 一并纳入用户已有 rust-toolchain.toml 删除、rustfmt.toml、typos.toml 及两份先前验证 memory 追加。根 rustfmt 收敛为 stable 支持的规则，与引擎 CI 一致；CI 仍显式选用 1.98.0，workspace rust-version 仍为 1.98。未引入 nightly 要求。
- 全部编译/缓存/临时文件在 D:/BuildCache/hagency-client，metadata 实测 target_directory 正确。未停止用户正在运行的桌面应用，未修改凭据、用户数据或实际登录。
- 本地已通过：引擎库与 console 测试严格 Clippy；工作流同款 18 包格式检查；33 项 Node 检查器自测；生产调用 wired=1/missing=[]；28 包 workspace 检查；桌面 501 passed/0 failed/2 原 ignored，miniapp-core 30 passed/0 failed；diff --check。引擎协议、私密状态和真实回调回归的最终结果随后记录。
- 本地最终回归 6 passed/0 failed：三项共享连接协议规则、两项私密状态检查、真实 HTTP OAuth-only 监听器。当前 lib 测试二进制为 hagency-dee4ce2beea2d352.exe；既有 Windows owned_runtime 测试 Mode dead_code 警告保持原状，没有增加 allow 或忽略。
- 将本次清理和工作区所有已有改动一次提交 main；后续远端验证以该提交的 GitHub Actions 为准，原始日志放入 D:/BuildCache/hagency-client/residue-remote-*.log，避免为补写验证记录再触发一轮完整编译。当前改动不涉及真实账号/模型或对外消息。
