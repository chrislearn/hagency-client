# Agent 开始处理反馈

## 行为

Agent 在本地 Codex 的 `turn/start` 返回有效 turn ID 后，以自己的 Appservice 傀儡账号，在原请求消息上添加一个 `👀` 表情回应。它表示已开始处理，完成后的答案与用量图标仍沿用现有回复展示。反馈不会修改原消息或用量账本。

只有通过请求策略、额度预留和设备运行权限校验，并且 Codex 已接受本轮处理，才发开始通知。排队、等待主人批准、配额不足、历史费用待确认以及未调用模型的拒绝均不产生这个标记。标记是一次开始回执，结束后保留；它不代表此刻仍在运行。

## 原实现与新链路

原 `hagency-matrix/src/presence.rs` 的 `ack_request` 使用固定 `👀` 和确定的 Matrix transaction ID。当前设备不能直接持有 Appservice 凭据，改为通过服务端设备 API 请求固定回执。

`POST /api/hagency/v1/execution/events/processing` 只接受 lease、dispatchId、executionId；客户端不能选择傀儡账号、Room、事件、表情或任意 Matrix 正文。服务端从当前 running execution 推导固定 `m.reaction`，其 `m.relates_to` 为 `m.annotation`、原请求 event ID 与 `👀`。

客户端并发执行模型与通知接收，模型结算本身不等待提示发送。通知每次最多 3 秒，最多尝试 2 次，授权拒绝不重试；短暂失败只记录无敏感内容的错误码，不改变模型结果或重放模型。快速完成也先消费开始通知，再继续回复持久化与 Finish。

## 服务端持久化与权限

专用 `processing_outbox` 与答案回复 outbox 独立，每个 dispatch 只允许一条不可改写的开始意图。稳定 Matrix transaction ID 用于重试去重；未知网络结果继续使用同一个 ID。

设备 API 校验主人、当前设备、指定执行设备、lease epoch、绑定 generation、精确 execution、请求状态和最新 Room 权限。新意图只允许 running；已经存在的同次已完成 execution 仅可读取原回执。worker 每次发送前再次验证原设备的有效 lease、会话、绑定与权限；同次 execution 快速 completed 后可发送已有回执，失权、取消、替换和旧 lease 不获得新的投递授权。

schema 为版本 4，启动严格拒绝旧版本。为本地现有版本 3 测试库提供专门的一次性维护 SQL，只新增回执结构并更新版本，不引入运行时旧结构兼容。Palpo 默认行为不变。

## 验证与部署

- 本地 Agent 库：64 通过、1 项原有忽略；Native console：176 通过、4 项原有忽略；Desktop：347 通过、1 项原有忽略。
- Native 和服务端严格 Clippy 均通过；服务端 PostgreSQL 51 项、OpenAPI 51 + 4 项、10 项契约守卫通过。
- 隔离真实 Pasion / Palpo 链路验证 Project Room 和主人 DM 的固定反应目标，未调用付费模型；包括重复请求、未知网络重试、403 禁止、旧 lease、换设备、schema 升级，以及事务等锁后观测过期不永久封禁。
- 当前本地数据库已先备份，再离线执行版本 3 → 4 专用 SQL。新版 server healthy，discovery 声明 `processing-reaction-v1`。
- 新版 Desktop 复用了原账号和 Codex 登录。用户 9:42 在 `Agent flow check` 发出的“介绍自己”消息得到一个 `👀 1`，并收到 Agent 的线程回复；服务端回执和答案均为 sent，完成事件从 10 增至 11。用量为 4467 Token，开始意图早于答案意图。未另发测试消息。
- testgeny 的 Project Room 与主人私聊均恢复 Ready，沿用预留 15000、low effort、无宿主文件工具、无强制接管，lease epoch 为 11。

通知是尽力而为的开始反馈：模型接受后、通知之前崩溃可能漏掉；启动后立刻失败或中断而未送出的回执也可能不显示。不会通过重跑模型补偿这个标记。
