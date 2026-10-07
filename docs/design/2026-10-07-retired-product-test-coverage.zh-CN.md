# 撤销旧产品验收与替代覆盖

日期：2026-10-07。新产品按用户要求删除 Fleet、旧数据兼容和服务器资源审批；历史测试不能通过恢复旧 CLI 或运行器来满足。独立 SDK 库测试继续保留，历史产品 selector 明确记为 Retired-Test，纯 Fleet 产品规格标记 superseded-by；这些不计为当前通过的测试。

完整 workspace 首轮实际执行中，configured_fleet 产品二进制套件 10/10 失败，使用旧 CLI、旧数据库及 Fleet 运行前提。已删除仅服务于该生产入口的 configured_fleet.rs 和私有 fixture，保留 Matrix/Codex 等独立 SDK 覆盖。

| 旧产品验证 | 当前对应证据或明确边界 |
|---|---|
| 两/三 Agent Fleet 启动、恢复及后加入 | OwnerHost 真实初始化、服务重启及登录门槛；独立 Agent supervisor、每 Room 显式启动、共享租约及独立暂停测试。不同 Agent 及真实模型任务按新协议另行验收 |
| Fleet 媒体及加密群聊 | SDK 独立媒体/crypto 测试保留；新傀儡加密凭据和端到端路径明确延期，保留 SDK 不证明新产品媒体/加密任务完成 |
| Fleet 本地 Codex 资源工厂 | Owner 专属 Codex home/keyring 与真实无推理握手、动态工具注册及冷恢复；真实用户模型任务仍需验收 |
| Project 提及、generation 与公平投递 | 真实 Pasion/Palpo 提及→poll/ACK/start→傀儡回复、跨 Project binding、成员撤权独立 generation、其他 Room 继续及原始入站 TTL |
| paced startup | SDK Matrix pacing 测试保留；新 server 必装 AS 真实启动往返、未 ready 创建/绑定 503，无资源 enrollment 启动链 |
| delegated task delivery | 旧工厂委派不是新产品承诺；有效 SDK 委派库测试保留，新产品逐 Room 请求及工具策略独立验收 |
| handoff diagnostics / worker outlives refusal | 新 host 失权取消、未知执行/成本保留、另一合法 binding 继续及 supervisor 关闭竞态；不计旧 Fleet handoff 为新能力 |

旧 bootstrap、CLI、安装和升级测试的替换由同轮发布审查逐项完成。selector 退休必须保留原因和对应关系；不把退休项计入 PASS，或把其旧能力宣称已交付。

完整首轮还证明旧 account 登录 CLI 4 项、旧 file_service 6 项生产流程、received_files 4 项和 task CLI 流程已失效；这些入口已撤销，相关产品测试与专属 fixture 移除，历史 selector 明确退休。file_service 中只检查旧 fixture 自身观测格式的测试随 fixture 移除，也不计为新产品覆盖。现有库中的媒体上传/接收、文件隔离与不确定性单元测试保留。新产品模型登录由 owner 专属 provider 控制，Pasion 登录和设备授权使用新 OwnerHost 实际闭环；群聊文件上传/接收不属于已交付的动态私密 list/read/create 通道。

MCP 协议、协调和旧 SDK factory 库测试使用明确命名的 `hagency-sdk-mcp-test-peer` 离线测试运输进程，直接复用既有 SDK stdio 实现。它不安装、不进入 OwnerHost CLI，不恢复旧 `mcp` 或 `task` 产品命令。这些通过只证明独立 SDK，不算新 Agent 的模型驱动任务验收。

旧 feature 浏览器模块中的 17 项产品测试（browser 13、live walkthrough 1、旧 status strip 3）依赖不再打包的 resources/enrollment/agents/tasks/usage 页面或旧 widget marker；已删除并逐条退休 14 个历史绑定，混合规格中的有效 SDK route Test/Filter 保留。新浏览器门槛为实际 OwnerHost 二进制的 OwnerRail 和真实 Chrome Owner 契约，两者再次通过。最终库存 1161 个有效绑定、82 条退休记录、2 份验证过替代目标的规格，missing 为 0；库存校验不代替执行。
