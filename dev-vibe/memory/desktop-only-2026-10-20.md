# 仅保留桌面产品（2026-10-20）

- 用户明确要求移除 CLI Web 控制台和无用旧页面，同时保留桌面共用引擎及 OAuth 回调。该指令覆盖上一轮暂时保留可选 Web 控制台的判断。
- 删除 mockup 全部受版本控制的页面、原型、JS/Node 配置与构建/浏览器脚本；删除 CLI main、service 安装器、静态资源编译 build.rs、资源加载/服务模块、CLI deploy 模板和 release-native/Pages 工作流。没有删除桌面 Mini app/内置浏览器/聊天页面。
- hagency 包作为库保留，autobins=false；显式 probe 二进制仍是原 SDK 测试依赖，非产品 CLI。桌面包和 hagency 程序名、数据目录、账号与权限协议保持原样。
- OwnerHost 仅保留私密状态锁及进程内 API router；取消管理 Web listener、CLI 私人 IPC、静态页面入口。NativeOwner 仍仅在随机 loopback 端口接受 console/server-login/callback；桌面通过 native_owner::open_with_matrix 调用原 Matrix 身份钉住的进程内业务服务。
- 保留所有共享业务模块及 SDK API 测试。删除 CLI/页面专属测试，在规格中记录 Retired-Test，不能当通过或增加 ignore。原页面断言改为旧路径不存在，API 权限、凭据脱敏、写者隔离和状态机断言保留。旧配置/凭据拒绝检查扩展到引擎状态单测。
- 新回归以真实 HTTP 请求检查回调监听器：页面、管理 API、health/ready 均无 endpoint（框架可能返回 404/405）；无有效状态的回调由回调 handler 拒绝。不是浏览器管理服务的模拟替代。
- CI 去掉网页构建/浏览器 lane 和 CLI 二进制产物，保留引擎测试、cgroup、release 库检查及桌面各平台构建/安装包。Dependabot 去掉已删除的 npm 工程。调用图检查从真实桌面 adapter 入口追踪至引擎，并新增“无 CLI 仍可达/未调用辅助函数仍不可达”的回归。
- README、引擎说明、工作区/开发/登录指南改为桌面入口；旧 CLI 专用 justfile 删除。历史设计/ADR 保留作记录，不作为当前安装路径。
- 全部本地 Cargo、target、临时文件、测试和日志在 D:/BuildCache/hagency-client，metadata 已确认真实 target_directory。未安装/卸载用户机器的现存 OS 服务，未删除用户数据，未实际登录、发送消息或调用模型。
- 当前本地 Windows 共享 SDK 全套初跑：116 passed/14 failed/6 原 ignored；其中旧页面断言已针对功能移除更新，其余包含既有 PlatformUnavailable/工厂 home 失败和网络连接诊断超时，不宣称通过，也不弱化平台拒绝。桌面和跨平台 CI 的最终结果后续追加。
- 实际监听端口回归 1 passed，私密状态/旧配置拒绝 2 passed；33 项 Node 检查器自测、真实桌面入口调用图、28 包 workspace 边界、workflow actionlint、241 文件样式、主题守卫、1990 翻译项/1498 调用/0 缺失均通过。
- Windows 全编译测试库存已完成；绑定 1116 项，14 个缺失项均是原有 Unix-only SDK 选择器（源文件定义保留），未为它们新增 skip 或虚报完整绑定通过。最终以 Linux CI 核对完整绑定。
- 桌面完整库测试：501 passed/0 failed/2 原 ignored；当前引擎 lib + console 严格 Clippy 通过；针对页面删除后权限/API/选择参数的 6 项实际业务测试全部通过。重复核对 Node 33 项自测和真实桌面调用图通过。
- 本地桌面 bin build 因既有运行中的 target/debug/hagency.exe 被 Windows 锁定而拒绝删除，未终止该用户进程，也不将本地最终二进制构建记为成功；桌面跨平台构建以远端 CI 为准。
- 工作区另外出现 rust-toolchain.toml 删除和 rustfmt.toml/typos.toml 新增，已询问归属，默认保留并排除在本次提交之外。格式核对显式使用原有默认配置及 Rust 1.98.0。上一轮 ci-fix memory 的未提交更新也排除。
- 用户随后明确确认工具链/格式配置改动属于其他工作，应保留并排除；代码清理已于 b390d51ef2b60e73f7bb9104d3081bef4d06de71 推送 main。桌面 bin cargo check 通过；本地空目录删除被自动工具策略拦截，不影响 Git 删除结果。
- 远端 b390d51e：App CI 39350428360 success（桌面 504 passed/0 failed/2 原 ignored；build、no-default-features check、主题/i18n/样式/工作区等全部通过）；App Build CI 39350428369 success（Windows、Ubuntu x86_64/ARM64、macOS x86_64/ARM64 五目标）。主 CI 39350428329 的格式、全目标 Clippy、规格绑定、Node 自测和生产调用图已通过，全套测试仍在运行。
- 最终主 CI 39350428329 success：所有 206 个成功测试汇总合计 2022 passed/0 failed（174 ignored，未新增忽略策略）；规格绑定 count=1128/missing=[]，真实桌面生产调用 wired=1/missing=[]。b390d51e 的三套自动工作流全部成功。完整日志保存在 D:/BuildCache/hagency-client/desktop-only-remote-{main,app}-ci.log；本地验证后的 memory 追加未混入用户已排除的其他工作。
