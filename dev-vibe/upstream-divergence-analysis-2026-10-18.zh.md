# Client 与原仓库的提交差异分析

日期：2026-10-18。比较快照：Client `b6da949bb304c3b2a038905cedf0dc7c12bd3743`，
原仓库 main `1212833a7bfe3b17f6f1d69fc8d528aca6ac0f36`。

Client 当前地址为 [hagency-org/hagency-client](https://github.com/hagency-org/hagency-client)。
旧 `chrislearn/hagency-client` 地址已由 GitHub 指向同一个仓库，并不存在另一份尚未同步的旧 Client main。
GitHub 的 fork parent 是 [hagency-org/hagency-rs](https://github.com/hagency-org/hagency-rs)，
“落后原仓库”的比较对象应是这个上游。

两条分支已在架构上分开。上游的 101 个独占提交主要围绕旧 Fleet、资源提供者与
Coordinator Engagement 产品；当前 Client 使用 Pasion 个人身份、永久主人 Agent、
服务端执行设备授权和原生 Desktop 管理。建议按当前产品需求评估能力和修复语义，
不整体合并上游 main。本轮仅更新仓库地址并分析，没有应用上游功能或修复。

## 提交数字代表什么

| 项目 | 结果 |
| --- | --- |
| 共同祖先 | `e51a0b13`，2026-10-03，原仓库 PR #27 |
| Client 独占提交 | 3814，包含完整 Desktop 导入历史和合仓维护提交 |
| 上游独占提交 | 101，其中 79 个非合并提交、22 个 merge 提交 |
| 合仓前 Client `39e990c8` 与同一上游 | Client 独占 23，上游独占仍为 101 |
| 上游 main 末次提交 | `1212833a`，提交时刻为 2026-10-10 12:48:08，Asia/Shanghai |

因此，这 101 个提交在 Desktop 合仓之前就已分叉存在，不是此次搬目录遗漏了
101 个提交。GitHub 的 ahead/behind 计算提交祖先关系，不衡量功能是否已经重实现。
`git cherry` 把 79 个非合并提交都列为不同补丁，但目录迁移和不同实现会改变 patch ID，
不能据此把它们全部判为功能缺失。

## 上游独占提交的组成

下表按 79 个非合并提交的主题归类；22 个 merge 不重复计入功能数。

| 类别 | 数量 | 主要内容 | 对当前 Client 的判断 |
| --- | ---: | --- | --- |
| Coordinator 和资源借用流程及稳定性 | 37 | 资源发布、委托、借用审批、额度分配、原项目迁移、退役、保留策略、重启和 Fleet 服务恢复 | 主要属于已退役产品流程，不宜恢复到当前主人 Agent 架构 |
| Claude Code 接入 | 18 | ADR-192、运行会话、旧 Fleet 配置、审批、Setup、现场验收与文档 | 当前已有另一条原生本地运行链，不能视为整体未实现；旧 Fleet 绑定和验收证据并未照搬 |
| Octos 接入 | 17 | ADR-193、OUP 会话、设置和 profile、审批、计量、旧 Setup、任务 host tools 和现场验收 | 低层会话及部分能力已进入当前代码；旧资源绑定、专用任务工具与计量接入仍是不同实现 |
| 请求工作区安全和项目目录设计 | 2 | 拒绝请求者指定工作目录或 bootstrap；ADR-194 的项目目录、知识和审批等级 | 安全原则适用于当前架构，产品设计需重新映射；ADR 中有未实施的后续规划 |
| 最后的跨运行时修复 | 5 | Octos 卡片与工具计数、Claude 越界写审批、审批 applied 回执、run ceiling、Octos 分阶段诊断 | 逐项核对当前入口；部分问题已有不同处理，部分实现确实未移植 |

## 已有能力和实际差异

### Claude 和 Octos 基础运行已经存在

当前 `crates/hagency-agent-local/src/runtime.rs` 有 `Kind::Claude`、`Kind::Octos`、
本地 profile、guardian 进程托管、审批等待、拒绝、用量和工具调用计数。
`console/owned_runtime.rs` 调用这条桥接链，Desktop 通过 `NativeOwner` 管理它。

尤其是以下 4 个文件，在把上游 `native/` 映射为当前 `crates/` 后，与上游 main
逐字相同：

- `hagency-runtime/src/octos.rs`
- `hagency-runtime/src/octos/session.rs`
- `hagency-runtime/src/octos/session/control.rs`
- `hagency-runtime/src/octos/task_tools.rs`

这说明上游一部分 Octos 工作已经通过重整或移植进入当前代码，虽然原提交不在
Client 的祖先链中。当前还有 `tool_denial_and_exact_owner_verdict_control_real_fixture_effects`
等协议 fixture 测试，覆盖拒绝、主人决定、工作区外目标与工具计数；它们不能替代上游
特定版本 CLI 的真实模型现场验收。

### 旧 Coordinator 和 Fleet 流程没有整体接入当前产品

上游有 `server-engagements` 页面、Coordinator 审批和委托、旧资源的迁移及费用保留等能力。
当前生产 `OwnerHost` 使用 `owner_router()`，只挂载主人 Agent、provider、Project 和
个人身份相关 API，前端构建也只选择 Owner 页面。仓库中保留部分旧模块不代表产品仍使用它们。

对应的上游修复例如 `b9aee592`（单个 Palpo profile 失败隔离）、`b388e809`（借用额度保留）、
`29e44e87`（旧运行时托管释放后重新挂载）依赖原来的 Engagement 和资源借用状态机，
不宜直接套用到服务端设备租约与主人账本上。

### 值得单独评估的修复

| 上游提交 | 修复目的 | 当前静态核对 | 结论 |
| --- | --- | --- | --- |
| [`f75b19e5`](https://github.com/hagency-org/hagency-rs/commit/f75b19e5bf98a041d365b4f963ca1167c60c7d15) | Claude 自动模式下，工作区外写入通过 PreToolUse guard 转为主人审批 | 当前没有 `claude/write_guard.rs`；但当前启动显式限制为 Bash/Read/Write/Edit/Glob/Grep，并对这六类全部请求审批，开启 sandbox，禁用外部 hooks 和 MCP | guard 实现未移植，但不能直接判定相同漏洞仍存在。若以后扩大工具集合或放宽审批，应优先重新验证越界路径、符号链接与写工具覆盖 |
| [`5e0ec880`](https://github.com/hagency-org/hagency-rs/commit/5e0ec8806d609058686c8035bc9fc9f7d1476b05) | 观察到对应工具结束后，把已发送审批从 applying 记录为 applied | 当前没有旧 `approval/claude.rs`、`approval/octos.rs` 及其新增回执关联实现；新桥接已有独立的审批队列、决定写入和工具返回计数 | 旧审批回执状态机未移植。若需要可审计的“已批准”和“已实际执行”历史，应针对新账本设计，不可把工具计数当作完全等价 |
| [`9294ca83`](https://github.com/hagency-org/hagency-rs/commit/9294ca8362430394823f971c546a27acbf3ac0d9) | 旧 driver 的 claim 按运行上限取值，避免按请求预算提前切断一轮 | 保留的 `bootstrap/driver.rs` 仍调用 `limits.capability_ms()`，未用上游常量替代；当前产品另走 30 秒续租和 1200 秒单轮边界 | 对旧 driver 是明确差异；当前生产运行链不等同于旧 driver，应验证长任务/长审批等待的边界，不直接移植常量 |
| [`c67f944d`](https://github.com/hagency-org/hagency-rs/commit/c67f944dd5df97d46d9bdc6687a59b925a5dab11) | Octos 卡片可读、Claude/Octos 工具计数、失败尝试后继续工作 | 低层 Octos 文件已相同，新桥接已经计数；当前 `owned_runtime.rs` 有按事件隔离拒绝与失败的处理 | 部分能力已具备。旧卡片和旧运行状态投影仍不同，不能整笔列为缺失，也不能仅凭标题认定完全等价 |
| [`aaa52025`](https://github.com/hagency-org/hagency-rs/commit/aaa52025882c1c506c9c4cc55087ac677f0b0e98) | 区分 Octos host tool 的解析、调用、结果写回和答案发送失败 | 当前缺少该旧 task-tool 执行接入及对应日志，新产品使用自己的 provider 错误阶段和静态错误码 | 若增加 task host tools，可借鉴分阶段诊断，但需符合当前日志脱敏规则 |

### ADR 不等于已交付功能

上游 ADR-194 接受了：每资源的基础目录、每项目目录名和公共仓库初始化、项目内
`.hagency/` 知识、ask/no prompts/full access 等级及项目拒绝策略。
同一 ADR 的实施部分仍将它们列为后续切片。上游快照中没有依据证明这些设计已全量交付，
不能把文档存在当作当前 Client 缺少一整套成品功能。

当前 Client 的工作区来自主人设置的 Agent 模型配置，具有 canonical 目录检查；
上游 `7670af93` 的“请求者不能指定路径或 bootstrap”原则仍值得作为边界检查依据，
但它修补的旧请求模型与当前的 owner profile 不是同一个入口。

## 建议和本轮边界

1. 保持当前 Client 和 Desktop 的主人 Agent 架构，不为了清除 GitHub behind 数字恢复旧 Fleet/Coordinator 产品。
2. 若继续分析修复，先核对审批执行回执和 Claude 工具/路径边界，再核对运行期限及失败诊断；按当前生产入口评估，不直接 cherry-pick 全套旧实现。
3. 把 ADR-194 的目录和知识共享当作产品设计候选，单独确认是否采用。
4. 是否保留 fork 关系或今后解除关系，是仓库管理选择。本轮没有同步上游、建立合并提交、重写历史或更改 fork 关系。

本轮授权的修改仅为新仓库地址：本地 origin、项目 metadata、About 源码链接、OAuth
公开 client_uri 和匹配断言、当前文档与迁移目标。未修改业务控制逻辑，也未运行真实模型。
