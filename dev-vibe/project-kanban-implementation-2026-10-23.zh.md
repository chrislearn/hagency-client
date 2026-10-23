# Project 原生共享看板实施记录（2026-10-23）

本次在 Server、Client 分别创建 `codex/project-kanban`，以 Server
`f48717f7`、Client `038bfda9` 为基线实现报告中的原生共享 P0。
结论仍是：产品代码只需要 Server 和 Client。Cloud 沿用现有托管、镜像升级与
数据库运维机制，无新增 Cloud API、任务库、成员映射或套餐开关。

## 使用与界面

进入项目详情，选择「看板」。每个已登记且有效的 Project 有一块共享看板，
当前加入该 Project Space 的人工成员可以协作。

- 五个固定状态：待办、进行中、阻塞、待验收、已完成。
- 沿用 Rinx 明暗主题，以中性背景、轻边框与状态色点组织五列。每列宽 260，
  看板横向滚动，各列独立滚动并使用 PortalList 虚拟化。
- 卡片展示项目内编号、标题、优先级、标签、负责人和日期；标题最多三行。
- 详情在右侧 340 宽面板中打开；窗口窄于 850 时，详情占可用宽度，关闭后返回看板。
- 详情可编辑标题、纯文本描述、优先级、人工负责人、标签、截止日期；支持评论、
  带操作者与本地时间的活动记录、归档和恢复。历史采用有界分页，可返回第一页。
- 支持标题、状态、负责人、标签、逾期、归档筛选，数量是服务端匹配总数。
- 从卡片拖动把手移动，显示拖动预览、目标列与边缘自动滚动。完整且无内容筛选的
  目标列可按真实邻居精确插入；筛选或未完整加载时由 Server 追加到真实列尾。
  详情菜单另有目标状态及「移至顶部／底部」，不强制使用拖拽。

任务详情单独读取，未取得完整内容时禁用字段保存。保存以 Server ACK 为准。
冲突保留草稿；「放弃草稿并加载最新」是显式操作。详情关闭也需要再次确认丢弃
未保存修改。切换项目会清空该视图的普通草稿；未确认提交的原始请求在同一账号的
进程内按项目保留，返回后可以原请求重试。退出应用不提供草稿或未确认请求恢复。

可见、前台时约每五秒检查变更，读请求不重叠，失败退避至最多 60 秒；隐藏时停止
轮询。账号、项目、视图与详情代次阻止晚到结果进入当前界面。已发出的 NativeOwner
操作不保证物理取消，界面会拒绝其过时代次结果；写操作不会自动重发。

## Server 契约

Server 在现有 Hagency PostgreSQL 中增加独立 `hagency_project_v1` schema 和四连接池，
不改变 Agent 领域迁移版本。原生用户 session bearer、当前会话与可信 Space 成员事实
共同授权，Cloud 团队身份和 Agent 权限不授予看板权限。

Board 行锁串行化同板写入。任务版本、服务端 rank、提交序号、活动及幂等回执在同一
事务提交；不同板可以并行。跨项目 ID、失效会话、退出成员、非人工负责人均拒绝。
详细 API、容量、锁等待、备份与回滚见 Server 的 `docs/project-board.md`。

能力名为 `project-board-v1`。旧 Server 显示明确不可用消息。可选 Host TOML
`project_board_enabled = false` 关闭能力与端点并保留数据。默认开启是本次明确的实施
选择，避免要求 Cloud 或自托管配置新增必填字段；部署时可以先显式关闭再试点。

## 已执行验证

所有验证在上述实际编辑 checkout 内执行，使用 Rust 1.99.0。无生产迁移、部署或推送。

| 验证 | 结果与证据 |
| --- | --- |
| Server 完整 Agent 套件，独立全新 PostgreSQL，显式启用 DB 测试并串行执行 | 85 成功，0 失败，0 忽略；`/tmp/hagency-kanban-server-final-tests.log` |
| 真实 Server HTTP + PostgreSQL + 两个真实 NativeOwner | 1 成功，覆盖共享、回执重放、移动、陈旧版本、重开、退出和外部身份拒绝；`/tmp/hagency-kanban-native-e2e-final.log` |
| Client 引擎与 Core library | 248 + 3 成功，7 项原有忽略；`/tmp/hagency-kanban-client-engine-full.log` |
| Client App 完整 library | 584 成功，0 失败，2 项原有忽略；`/tmp/hagency-kanban-client-app-final-3.log` |
| 看板原生脚本／布局／代次／冲突测试 | 已包含在 App 套件；覆盖 600/1100/1500 宽度、窄窗详情、千张卡片虚拟化和实际邻居定位 |
| 受影响 crate 严格 Clippy | Server/backend/xtask all-targets 及 Client/core/app lib/tests/examples，均使用 `--no-deps -- -D warnings` |
| 格式、OpenAPI、翻译与绑定清单 | Server fmt；Client 现有 CI 格式范围；71 + 4 API、13 契约反例；2303 翻译项、0 缺失；1155 Rust 绑定、0 缺失 |

联调 fixture 只替代 Pasion introspection 与 Matrix 成员来源；Server、SQL、TCP、
NativeOwner 身份与命令路径实际运行。它不证明真实 Matrix 账号或两台桌面的产品验收。
可重复运行方法在 Server 文档中，测试 fixture 端点不进入生产 router。

保留失败记录：首次并行运行旧 Server 套件遇到领域初始化 DDL 死锁；复用污染测试库
又使两项旧全局 outbox 测试失败。随后按仓库既有流程使用全新库、串行执行，85 项通过。
Client 初次 App 回归发现看板内部 `workspace` 控件名干扰既有 Agent 设置查找，现已用
独立 widget 搜索边界封装并通过完整回归。拆分 UI 模块时曾有宏导入编译错误，已改为
显式 Makepad 宏导入。未用忽略测试、放宽断言或生产重试掩盖这些失败。

## 尚未取得的验收证据

开发用只读原生预览：

```sh
cargo +1.99.0 run --locked -p hagency-app --example project_board_preview -- --detail
# 可选 --narrow、--dark；数据目录默认隔离到 target/kanban-preview
```

预览已在本机 macOS 到达首个 Metal 呈现帧，日志为
`/tmp/hagency-kanban-preview-trace.log`。系统 UI 工具多次读取预览超时，未取得可供
检查的截图，不能记作视觉验收通过。Windows/Linux 实机、真实账号双桌面、高 DPI、
断网恢复、真实备份恢复与生产迁移耗时仍需发布前验证。未声称达到报告的延迟／吞吐目标。

本次不包含 Web、附件、通知、Agent 执行、自定义列、离线写入与 E2EE。

后续复核：以上原生视觉限制是首次实施时的记录。随后用户发现静态预览事件被冻结，
现已修复并通过真实 macOS 窗口操作与 SQL 持久化核对；详见
[功能复核与交互修复](project-kanban-feature-review-and-interaction-2026-10-23.zh.md)。
Windows/Linux、真实 Matrix 账号和生产恢复等未验收边界仍保留。
