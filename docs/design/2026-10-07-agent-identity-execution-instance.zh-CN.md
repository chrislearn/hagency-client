# Agent 身份、执行实例与 Room 接入实施细则

后续设备模型以[Agent 执行设备与跨设备设置细则](2026-10-08-agent-execution-device.zh-CN.md)为准：独立执行实例已合入 Agent 的执行设备字段，新建默认本机，设置支持改由本机处理；旧实例 ID、名称、表和 API 不再作为当前设计。内页顶部返回列表按钮也按用户后续要求恢复。下面保留此前阶段的设计和验收记录。

本轮对用户最新需求作出的实现调整。正式仓库均位于 `/Volumes/Data/Works/chrislearn`，本文保存在 hagency-client。本文关于 Agent 创建、设备执行权和默认联系人的规则替代早期报告中“创建 Agent 必须指定 Project/Room”的流程。

## 领域与权限

Agent 是 homeserver 范围的 Appservice 傀儡身份。创建接口只接收显示名和幂等键，owner 从当前 Pasion/Matrix 身份推导，不能由请求指定。固定 `_hagency_` 前缀属于强制安装 Appservice 的独占用户命名空间；客户端不能指定任意 MXID。创建者、Agent ID 和傀儡 MXID 永久不可转让，退役后也不释放。

Project 对应真实 Matrix Space；讨论 Room 各自管理成员。Project 的默认允许、allow/deny 和 Room 的继承/名单/关闭策略决定谁可以将自己的 Agent 接入该 Room，不再阻止服务器范围的身份创建。服务器同时检查当前 owner 的真实 Space/Room 成员资格、Room 登记与邀请权限。资源配额、模型登录和工具批准由主人在本机决定，无需管理员批准。

| 对象 | 作用域和数据 | 创建或修改入口 |
|---|---|---|
| Agent | 永久 owner、稳定 puppet MXID、身份状态、generation、可空主人私聊 ID | Agents → 创建，仅名称 |
| 执行实例（Execution instance） | 一个 Agent 至多一个实例；稳定实例 ID、owner、设备 ID、名称、generation | Agent 设置 → 执行实例 |
| Project Room binding | Agent、Project、Room、接入状态及 generation | Agent 设置 → 邀请加入 Room；或 Project → Room 信息 → Invite my Agent |
| 主人私聊 binding | scopeKind=`owner_direct`，Project 为空；独立 private Room，只处理主人请求 | 创建身份后的默认联系人流程；Agent 设置可恢复 |
| 本地执行配置 | 按完整 Matrix 账号身份和 Agent 隔离的 Codex 凭据引用、模型、工作区、全局额度；可显式复用本机 Codex 已有登录 | 指定执行设备上的 Agent 设置 |
| Room/requester 配置 | 每个 Room 及发言者独立的额度、请求许可、工具许可 | Agent 设置 → 对应 Room → 配额与策略 |

## 设备与执行权

设备 ID 是服务器对同一主人、同一客户端安装所维护的稳定身份，不是一次授权的 token 或 generation。授权续期不能把实例绑定到另一台设备。实例外键同时约束 Agent owner 和 device owner；禁止给其他用户设备分配。初次创建使用 expectedGeneration=0，以后修改使用读取到的 generation，冲突时必须刷新。

实例分配和短期执行租约承担不同职责。即使主人在多个设备登录，只有分配的设备能取得该 Agent 租约；poll、ACK、start、renew、结果和回复操作持续检查有效设备/租约/Agent 与 binding generation。`takeover` 不能代替设备分配。

更换设备使旧租约和执行 generation 失效，已运行但无法确定结果的事件保留 unknown，不重新推理；旧待发回复取消。客户端尽力停止该 Agent 的本机任务，不停止其他 Agent，不清空额度、预留或历史。配置没有跨设备自动复制：目标设备需确认可用的 Codex 登录，或复用本机已登录账号、配置工作目录并恢复完整账本。已有执行历史但缺少可靠费用账本时，不能凭空新建空账本继续消费。

实例保存不代表运行器在线。开始处理请求仍需指定设备、有效 Codex 登录、模型/工作区、三层策略以及显式启动。当前 Codex 不能保证 Strict 硬 token 上限；Estimated 模式需主人明确同意，并使用调用前预留、实际用量结算和未知费用保留。

2026-10-08 补充：有效 Codex 登录可以来自本机已有的 Codex 账号，不要求再次授权。Matrix 账号隔离的是账本、工作目录和使用关联，而非必须复制一份 OpenAI 登录。显式选择“使用本机 Codex 账号”后，由 Codex 自己管理既有认证，Hagency 不复制或返回凭据；断开该使用关联不退出本机 Codex。共享登录仍须逐子进程验证实际提供商、工具与通知配置，不继承本机其他执行权限。实机复审与验证记录见 `2026-10-08-desktop-real-agent-flow-review.zh-CN.md`。

## 默认联系人与主人私聊

Matrix 没有统一的好友关系接口。这里以普通私聊 Room 和主人的 `m.direct` 建立可见联系人。由主人通过自己已有 Matrix OAuth 会话创建私聊并邀请傀儡，服务器 Appservice 接受邀请；不需要第二次 Pasion 登录，不让傀儡抢先替主人创建 Room，也不邀请服务账号进入主人私聊。

私聊创建使用 `is_direct:true`、invite-only、guest_access=forbidden、非 Space、无加密。新 Agent 的端到端加密执行按既有决定延期；不修改其他 Matrix 聊天的加密设置。服务端使用真实成员状态确认主人已加入、傀儡已邀请或加入、没有第三方加入或邀请，私聊只投递主人消息且不要求 mention。增加第三方、开启 guest access/加密等变化会使这条服务关系失效，不能借私聊绕过 Project 接入策略。

创建身份与建立联系人是独立阶段。身份已确认后自动进行联系人流程；网络错误、身份尚在创建或 Matrix 加入未完成时保留身份，显示待恢复，不再次创建 Agent。联系人入口只有 server 确认 active 后才能打开。

客户端按 origin/issuer/subject/owner 隔离私密持久记录，先写创建意图再调用 Matrix createRoom。稳定普通 alias `hagency-agent-{agentId}` 与 owner 写入的 `im.hagency.agent.owner_direct` 标记用于恢复，不属于 Appservice 独占 alias 前缀。创建结果未知时先解析 alias、读取真实状态并核对标记、owner、puppet、私密性；不能盲目重发 POST，不能接管他人的同名 Room。

Room ID 已知时先保存，再读取并合并现有 `m.direct`，保留其他联系人。重试重新读取最新 account data；标准 Matrix account data 没有跨设备 CAS，多个客户端同时修改仍有普通 Matrix 客户端共有的写入竞争。服务器私聊映射唯一且持久，其他设备优先读取既有映射，不能为同一 Agent 换一个主人私聊。

## API 与状态契约

HTTP 路径继续为 `/api/hagency/v1`，产品 discovery 的 protocolVersion 升为 2。客户端要求 `global-agent-identity-v2`、`execution-instance-v1`、`owner-direct-v1` 等必要能力；旧协议和普通 Matrix 服务器明确拒绝。

| 接口 | 请求 / 返回 |
|---|---|
| POST agents | `{displayName,idempotencyKey}` → `{creation:{agent},commandState}`；没有首个 Room binding |
| GET devices | 当前主人设备列表，不含授权 token |
| GET/PUT agents/{id}/execution-instance | GET 可空实例；PUT `{deviceId,name,expectedGeneration}`；owner/CAS 复核 |
| POST agents/{id}/bindings | `{projectId,roomId,idempotencyKey}`；真实资格检查后异步加入 |
| GET agents/{id}/owner-direct | 可空 ownerDirectRoomId、可空真实 binding |
| POST agents/{id}/owner-direct | `{roomId}`；验证当前 owner 的真实私聊并驱动傀儡加入 |
| 本地 POST owned-agents/{id}/owner-direct/ensure | 闭合原生联系人工作流；ownerDirect 包含 roomId/bindingId/state/phase，未知结果可恢复 |
| 本地 agent-policy / agent-model-profile | 无需 binding/requester；与运行器使用同一本地 Agent 账本键 |

身份 creating/active/suspended/retiring/retired、绑定 joining/active/suspended/leaving/left、执行实例分配、租约和 provider 就绪分别展示，不把其中一项成功当成整个 Agent 已经服务。

## 界面实施

移除管理页所有 Back to list 与右侧 Close；通过左侧列表及对应详情导航。页面标题缩小，创建页不重复标题。创建 Agent 页仅显示名称；创建后进入身份详情，再配置执行实例、模型、全局额度及需要服务的 Room。

Agents 详情的身份区显示傀儡 MXID、状态和主人联系人恢复入口。执行实例区选择已授权设备并标注本机；其他设备不能通过运行按钮或旧租约启动。模型工作区等本机配置与跨设备分配区别呈现。Project → Room 的右侧信息栏提供 Invite my Agent，仅列出自己的可用 Agent，并走与 Agent 设置相同的正式 binding 接口。

Web owner console 同步去掉创建时的 Project/Room 条件，增加执行实例、独立 Agent 配额/模型和联系人恢复，避免新服务协议发布后旧表单继续提交错误结构。

## 数据部署、验证与审查

新领域 schema deployment=2；遇旧版本明确拒绝，不自动 ALTER、迁移或复制旧 Agent 身份。当前本地环境使用独立新 Hagency 数据库，原 Hagency 库保留。Palpo/Pasion 库、账号、真实 Space/Room、媒体及稳定 Appservice 密钥保留。旧 Project 登记不会自动导入；新领域可通过显式绑定已有 Space 重新登记。

验收覆盖：无 Project 创建、幂等恢复、永久 owner、一次设备授权续期保持稳定设备、跨设备拒绝/CAS/更换 fencing、无 Project 主人私聊真实 Gateway 验证与 poll/start/reply、第三方成员拒绝、Room 接入资格、暂停/退出/退役、迟到响应隔离、联系人未知创建恢复与 m.direct 合并、真实 UI 入口和布局。测试结果、部署版本及未完成能力在最终验收记录中补充；不能以 fixture 代替实际付费 Codex 推理验收。

### 最终复审与验收记录

复审修复了四个实际遗漏：私聊世界可读历史未被拒绝；身份仍在创建时默认联系人未自动续建；私聊错误显示 Project Room roster 控件；首次查询创建命令错误返回 401，使持久命令流程无法开始。服务器管理页仍提交旧的首次 Room 参数也已同步修正，Project/Room 权限说明改为 Agent 接入资格。

Native 两个源文件在编辑中曾被误覆盖；已恢复原 SDK 身份、回调、scope 拒绝和运行流程，再重建增量改动，并通过原有 runtime 全部测试、scope 选择器测试及新真实端到端流程。最终审查包含新增未跟踪的 direct.rs、commands.rs、owned_runtime/http_tests.rs；不把它们遗漏于审查清单。

| 验证 | 结果与边界 |
|---|---|
| Desktop 库测试 | 325 通过、1 忽略；最终 UI 16/16、Room 信息列专项 5/5，OAuth 4/4 |
| Native Console 库 | 75 通过、1 忽略；包含默认身份等待、账号隔离、实例、恢复、策略与运行器 |
| Native 登录 HTTP | 7 通过、2 忽略；准入 4 通过、1 忽略；真实模式另行显式执行 |
| 本地额度库 | 52/52；持久用量与未知预留不因重新分配设备清空 |
| 真实 PostgreSQL | 45/45；旧设备 ACK/tool/finish/reply/reconcile 拒绝，旧 unknown/sending outbox 不可续送 |
| 真实 Native→Palpo/Pasion | 临时三数据库、一次 DCR/PKCE/登录、Space/Room 登记、全局创建、自动默认私聊、重复恢复、实例 CAS、Project Room 绑定与名单、退出闭环通过（11.72 秒） |
| 真实 server 集成与恢复 | 隔离 PKCE/主人 DM 无 mention 回包、三数据库与独立文件系统备份恢复、重指定设备和晚加入清理通过；没有调用模型 |
| 构建与接口 | Native/server 严格 Clippy、backend 构建、Frontend 创建请求回归、OpenAPI 51 个 owner/4 个 AS 操作及 8 项防漂移通过；Web owner console 144 个导出资源构建通过 |

本地测试服务已切到新库 `hagency_agent_v2_20261007`，HTTPS `https://hagency.local` 的 TLS 校验为 0、readyz=200，discovery protocolVersion=2。运行镜像为 `sha256:9601e7f43e5dc5d984cca2c22fb7d5501c81835d43146917fc6c99271ff841c4`；源码指纹为 `0fec964bde84772fb4df819173c3af01c79d1bc6c35a1aff66c8d93e2605d4e3`。既有 Palpo/Pasion 库、账号、Room、媒体、Caddy 保留，AS 注册恰好一条且密钥字节不变。旧 Hagency DB 与原配置私有备份保留，不自动导入旧 Project/Agent 映射。

Desktop 新二进制已构建并启动（PID 12008，启动日志 `.run/hagency-local-https-20261007/desktop-agent-identity.log`）。macOS TCC 日志确认新调试签名等待可移动宗卷读取许可；已请求用户手动处理。此状态不能算本轮最终原生窗口截图验收通过，许可后的实际检查结果继续追加。

仍明确延期：新 Agent 加密执行、可证明的 Codex Strict 硬 token 上限、实际付费 Codex 模型全链验收。普通 Matrix SDK 加密保留；现有 Estimated、本地工具审批和服务器请求/确定回复链不是虚构上线状态。
