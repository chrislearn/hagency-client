# Hagency 新 Agent 架构决策记录

日期：2026-10-07。状态：已实施；实际验证及尚未通过的门槛以[实施报告](2026-10-07-server-appservice-client-refactor.zh-CN.md)为准。本文固定正式产品决策，历史草稿不构成兼容合同。

## ADR-001 用户永久拥有 Agent，取消 Fleet

用户需要创建自己的 Agent 并用本机资源处理 Room 请求，服务器不提供待批准的模型资源。因此 Agent 与创建者是永久关系，与 Room 是可独立管理的 binding；Agent 不属于 Fleet，也没有唯一 Project 外键。

决定：owner 只从经过验证的会话取得。数据库禁止更改、删除永久 Agent 身份，退休不可逆。没有转让、认领、继承、重分配及旧数据导入入口。跨 Project 复用同一傀儡 MXID；暂停某个 binding 不暂停其他合法 binding。

依据：用户明确要求终身创建者、不兼容旧结构及允许跨 Project；服务端 `domain_schema.sql` 的 identity/binding 约束与 `domain.rs` 领域操作。恢复必须保留原 owner，而不能借重建账号改变主人。

## ADR-002 一个部署自带必装 Appservice

决定：`hagency-server` 保持真实 backend workspace 与内嵌 Palpo/Pasion 编排，自动安装一个服务级 Appservice。稳定注册、namespace、AS/HS 密钥由服务端私有文件持久化。用户不安装 Fleet、不持有 Appservice 密钥。

启动后使用真实 Matrix 探测 Room、send 与持久 inbox 验证接入；`/readyz` 为本次启动证明，不是持续健康承诺。Appservice 注册冲突不能自动覆盖；安装不得改 Palpo 源码、默认聊天或权限规则。

依据：`hagency-server/crates/backend/src/agent_appservice.rs`、`main.rs`、`agent-service/src/appservice.rs` 及实际原生集成测试。

## ADR-003 Project 对应 Space，Room 成员独立

决定：一个 Project 登记一个真实 Matrix Space，一个 Room 当前登记到一个 Project。一个 Agent 可有多个 Room binding。Space 成员资格不复制为子 Room 成员资格；普通 restricted Room 的加入资格仍由 Matrix 原有机制决定。

创建须同时满足 Project/Room 策略、真实双方成员及合法傀儡邀请/加入能力。deny 优先。创建禁止与服务暂停分开；新增创建禁令不隐式取消已建立服务。

发现 Room 必须当前同时属于 Space 和该 Room；Room Agent 名单只要求当前属于该 Room且真实父子关联存在，不向未加入私密 Room 的 Space 成员泄漏名单。创建 Space/Room 使用用户自己的 Matrix 授权及普通 Matrix API，然后 adopt；不使用服务级密钥替人创建群聊。

依据：`gateway.rs`、`domain.rs`、`domain_discovery.rs`、原生 Matrix 创建 journal 与真实 OwnerHost 创建测试。

## ADR-004 Pasion 身份、Hagency 会话和设备执行权分离

决定：客户端作为 native public OAuth client，以 DCR、授权码、PKCE S256、state 和 loopback 回调登录用户自己的 Matrix 身份。令牌保留在 Rust host。服务端联合 Pasion introspection 与 Matrix whoami，固定 issuer/subject/MXID，不以昵称认领原身份。

用户会话用于 Agent/Project 管理；独立 device token、generation 与 Agent lease 用于投递和执行。两种 bearer 不互换。授权复核有界；退出、撤销和续期不能恢复旧设备。断网退出先停本机，远端撤销需补交或等待实际授权到期。

本机 IPC 只签发 60 秒单次控制台票据，锁内兑换后立即消费；生成的本地 cookie 授权最长 900 秒，进程重启不持久恢复。它与 Pasion/Hagency 用户会话、设备执行权及提供方登录分别核验，不能作为任一远端或模型凭证。

依据：客户端 `console/server_login.rs` 与 `device_execution.rs`，服务端 `identity.rs`、`store.rs`、`api_transport.rs`；真实 Pasion native 登录、续期、撤销及创建闭环。

## ADR-005 服务端管理资格，创建者管理资源和工具

决定：模型凭据、预算、发言者政策与工具决策都在 owner 客户端。服务端的 `authorize-tool` 只验证正在运行的执行身份及当前 Matrix 作用域，不授予模型额度，也不是管理员批准工具。

额度分为 Agent、binding 与 requester 三层，先并发预留再按实际使用结算；未知费用保留。每个明确启动的 Room 独立 worker，一个 Agent supervisor 持有一个设备租约。未启动的 Room 不执行，停止一个 Room 不释放其他 Room 使用的租约。

补充决定（已落地并通过专项及新镜像验收）：换执行设备不允许重置预算。取得租约前核对服务端 Agent 全部历史已启动执行与本地完整费用凭证，租约原子校验历史快照，取得后再次检查。服务器仅返回执行元数据，不保存模型金额；缺失账本或不能证明费用完整时必须阻止新模型调用，提示恢复同一 owner 的完整新 schema 账本。执行 ID 相同、用户勾选接管或管理员许可都不能代替费用证据。已有完整未知费用预留继续保留，已知回复恢复不等于允许新推理。

AskOwner 绑定原请求或完整工具参数、owner/device、nonce/digest、政策 revision、有效期及一次消费。模型不能读取管理 bearer；跨 Room 的上下文和文件能力不可自动复用。首期只开放可选受限 Room 文件 list/read/create，其他工具保持不可用。

依据：`hagency-agent-local` 的 ledger/inbox/Codex/file host；`console/owned_runtime.rs`、`owned_agents.rs` 与多 Room/精确审批测试。

## ADR-006 Codex 首期与预算能力边界

决定：首个 adapter 使用实际已验证的 Codex app-server 协议及 owner 专属 home/keyring，不复制日常 Codex 凭据。实测版本 0.160.1 没有可证明的单次硬 Token 上限；Strict 必须拒绝，Estimated 必须由 owner 明确选择。估算可能超支，实际结算阻止后续请求；不能将估算冒充硬配额。

所有模型 turn 启动前和工具实际调用前核验远端授权。长纯文本 turn 每 5 秒再次核验，单次核验上限 5 秒；失权后尽力停止子进程并保留未知费用。无法承诺远端提供方停止或计费瞬时撤回。

真实协议握手、动态工具注册和冷恢复属于无推理证据；真实用户登录及模型驱动任务是独立验收门槛，不能用 fake peer 或 API 200 替代。

## ADR-007 新数据、可靠投递与恢复

决定：独立 PostgreSQL `hagency_agent_v1` 与 owner 私有新 SQLite，不导入旧 Fleet/Engagement/账本。Palpo/Pasion 保留各自数据库；Hagency 新数据要求不授权删除它们。

Appservice 先持久收件再 ACK，客户端先持久收件再 ACK、先持久 execution ID 再 start。未知执行不自动重跑。回复固定原结果、transaction ID 与执行身份；只有 Matrix sent/event ID 证明送达。旧 generation 永久失效，恢复身份或重新加入不能复活旧任务。已观测 Room 成员/发言权撤销或实际发送 HTTP 403 的旧回复永久禁止投递。设备、会话或租约失效撤销当前发送权；主人可在新 epoch 显式恢复未被永久 blocked 的已知回复。可信原事务事件的历史确认只收敛送达事实，不重新发送或解除 blocked。

TTL 从可信 AS 入站时间计算，积压路由不重置期限。去重摘要及永久归属必须保留；正文压缩不代表磁盘安全擦除或无限存储。三数据库与部署身份、AS 凭据、Matrix/Pasion 密钥及媒体恢复共同构成恢复边界，不能把三库 SQL 恢复等同于整机灾难恢复。

## ADR-008 复用 crypto，明确延期缺口

决定：保留现有 Matrix SDK Olm/Megolm 与 crypto store。受限傀儡设备授权、完整 to-device/密钥同步及新边界往返验证尚需工程改造，按用户允许的困难加密延期执行。当前新领域拒绝加密 Room，不将其降级为明文，也不交付 AS 密钥作为替代。

依据及后续工程面见[加密缺口报告](../../../hagency-server/docs/CRYPTO_CAPABILITY_GAPS.zh-CN.md)。新凭据与密钥仍必须属于原 owner；旧密钥不能通过远程撤销被抹除。

## ADR-009 客户端支持多个隔离账号，登录为入口

用户明确要求同一个客户端可以选择历史服务器，也可以切换不同 Matrix 账号和服务器。客户端不再永久限制为一个账号；每个 Agent 的永久创建者规则保持有效。

决定：页面在显示管理控制前核验当前浏览器的 Matrix 授权，未登录或过期先进入 Pasion 登录。设备名称使用默认值。服务器地址历史只存本机的有限 origin 列表；账号配置按 `(origin, issuer, subject, MXID)` 的规范 JSON SHA-256 识别，选择配置不能恢复 token。OAuth 回调回到新的 Owner 首页。

切换串行封闭所有旧浏览器权限、待兑换链接、设备执行与审批，停止原账号运行器和提供方进程，再建立有限本机启动权限并重新 Pasion 认证。登录页只选择服务器，不显示或预选用户名；进入 Pasion 前解除旧 profile 的登录限定，实际完成认证的 subject/MXID 决定打开哪个隔离账号。协议显式选择 profile 时仍固定完整身份，不能通过昵称或相同 MXID 获得另一主体的数据。配额、账本、恢复见证、Codex home/keyring 引用与 runtime key 全部按完整身份隔离。

数据库永久固定 profile digest，并与目录身份标记一致。原 origin/MXID 原型目录保留但不自动采用；已有远端执行历史而缺完整同身份账本时仍要求恢复，不能借切换或新空目录重置可执行预算。旧账号失效 cookie 不得退出或停止新账号的任务。

依据：用户在本轮登录改造中明确确认多账号切换；`console/server_login/profiles.rs`、`owned_agents.rs`、`owner_provider.rs`、`owned_runtime.rs` 和本地 `profile_identity` 约束；验证见实施报告与数据协议。
