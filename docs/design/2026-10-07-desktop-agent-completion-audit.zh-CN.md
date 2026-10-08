# Desktop Agent 创建、资源与运行闭环核对

> 2026-10-08 后续需求更新：未配置预算默认不限额，不再阻止请求；既有 Unset 也按不限额读取。本文中“Room 未设额度阻止调用”仅记录修改前的实测情况，不代表现行规则。有限额度、0 额度、显式拒绝、权限和未知费用检查仍生效。当前设计及新验收见 [可用性复核](2026-10-08-desktop-usability-review.zh-CN.md)。

本轮最新身份/设备/联系人重构见[Agent 身份与执行实例实施细则](2026-10-07-agent-identity-execution-instance.zh-CN.md)。其流程替代本页初次核对时的 Project/Room 首次创建绑定；以下测试记录保留为对应阶段证据，不冒充本轮新协议验收。

核对对象是 `/Volumes/Data/Works/chrislearn` 下的 hagency-desktop、hagency-client 和 hagency-server。本文补充原实施报告，区分已经实现的产品功能、测试覆盖与仍然存在的能力限制。没有修改 Palpo 默认功能，没有引入 Fleet、旧结构迁移或 Agent 转让。

## 核对结果与修补

原先不能称 Desktop 的 Agent 功能齐全。创建与运行后端已经存在，但界面遗漏了部分功能；Agent 全局配置还错误地依赖 Room/requester，创建幂等键没有跨应用重启的恢复入口。本轮对照 hagency-client 的 `OwnedAgentControl`、原生 owner API、本地账本和服务端领域规则补齐了这些缺项。

| 功能 | 原状态 | 本轮结果与入口 |
|---|---|---|
| 创建 Agent / 绑定已有 Agent | 原表单把身份和首次绑定混在一起 | Agents → 创建仅名称；已有身份通过独立 Room 邀请界面选择 Project 与注册 Room，服务器检查接入资格 |
| Agent 总额度和默认请求/工具策略 | 必须指定 binding 和 requester，未绑定或退出全部 Room 后无法独立管理 | Agent 详情 → Token budgets，选择 Agent total；现在不依赖 Room，使用与运行时完全相同的账本键 |
| Room 总额度 | 本地账本已有，界面混在一起 | 选择 Room → Token budgets → This Room total；不与其他 Room 合并 |
| Room 中某用户的额度与请求策略 | 已有 | 选择 requester → 加载策略 → This requester in Room；Allow / Deny / AskOwner |
| 周期与使用量 | 已有 Lifetime / UTC day / UTC month | 显示当前 revision、spent、held；Token count / Unlimited / Unset；修改或恢复默认策略不会清空消费或未知费用 |
| 高风险工具策略 | 账本和拦截已存在 | Deny / AskOwner / 精确工具与绝对目录规则；三层策略共同生效，宽松下层不能绕过严格上层 |
| Agent 本地资源 | 模型/workspace 实际按 Agent 共用，但编辑错误地借 Room scope | Codex resources 独立按 Agent 读取、保存模型及规范工作目录；不依赖 Room；凭据仍在本机私有 provider 存储 |
| Codex 登录管理 | 缺取消和退出入口 | Codex resources → 查看、登录、取消登录、退出；服务器管理员不审批或接触凭据 |
| 推理与执行选择 | effort 固定默认值，运行选项混杂 | Runtime and approvals → Medium/High、每轮预留、显式 Estimated 同意、显式 lease 接管、可选 Room host 文件工具 |
| Room 生命周期 | 缺部分界面入口 | 暂停、恢复、显式确认退出；不删除永久 Agent 身份，不转让 owner |
| Room Agent roster | API 已有，Desktop 未展示 | 选定 Room 的 roster 入口，读取该 Room 的真实 Agent/绑定状态 |
| Agent 退役 | API 已有，Desktop 缺入口 | 明确确认退役；不能将其表示为可逆暂停 |
| Project / Room 创建策略 | server 管理页已有，Desktop 未接 | Project/Room 管理区域；服务端 `canManagePolicy` 为真才显示写控件。Project defaultAllow/allow/deny；Room inherit/disabled/allow-list |
| 管理员服务暂停 | server 已有，Desktop 未接 | 同一 scope 管理区域；与禁止未来创建区分，不替 owner 批准本地资源 |
| 创建与绑定恢复 | 仅内存保留幂等键 | Agents 顶部未完成命令区；持久保存原 operation、Agent、Project、Room、名称和 key，只按原 ID 恢复 |
| owner 请求与工具审批 | 后端已存在 | Runtime and approvals 显示待审精确提案，批准一次或拒绝；切换 Agent 清理旧审批，决策仍由完整身份、device、generation 与 digest 复核 |

## 合理的使用顺序

1. 使用自己的 Matrix 账号通过 Pasion 登录 Hagency server。Desktop 与 owner 管理服务共用同一 SDK 身份，不再次要求 Pasion 登录。
2. 在 Agents 独立创建服务器范围的 Agent，设置指定设备的执行实例，再从 Agent 设置或 Project Room 邀请已有 Agent。一个 Agent 可以跨 Project 工作，永久 owner 不变。
3. 在 Agent total 设置全部 Room 共同使用的额度、默认请求和工具策略，在 Codex resources 配置本机模型与工作目录。即使当前没有 Room，也能管理这些全局配置。
4. 为各 Room 设置额度。默认 Room 额度是 Unset，未配置时不会开始模型推理。根据需要再为 Room 内用户设置更严格的额度、拒绝或逐次确认。
5. 在目标 Room 显式启动运行。owner 本地额度与工具选择不向服务器管理员申请批准；服务器只检查身份、接入资格、成员权限及执行设备。
6. 查看运行状态和审批。停止一个 Room 不应停止其他 Room；暂停整个 Agent、服务器 scope 暂停或账号切换则按既有生命周期终止相应运行。

## 创建恢复协议

命令在第一次远端 mutation 前写入当前完整身份对应的私密目录。目录由 origin、issuer、subject 和 owner 共同隔离；条目没有 token、provider 凭据或消息内容。文件有界读取，采用私密文件检查、原子替换和持久化同步。

创建和绑定都先调用服务端 `/api/hagency/v1/commands/{operation}/{key}` 查询原命令。已存在时读取既有 Agent/Binding；明确 404 时才发送保存的原 payload 和原 key。网络错误、身份错误和其他未知结果不被当作 404，也不更换 key。客户端重启后可恢复原条目。

同一未完成 operation/Agent/payload 改用新 key 会得到 `agent_command_recovery_required`，防止重复创建。同账号下另一个明确不同的请求仍可提交；一个 Room 被撤权不能卡死整个账号。界面未成功读取历史时禁用创建，未知条目保留原输入供恢复，恢复后再次读取真实状态；不能把 pending 当成完成。

接口：`Command::AgentCommands` 与 `Command::ResumeAgentCommand{id}`。返回创建结果沿用原 `{creation, commandState, pendingReason}`。该持久化保证在 owner 原生管理服务中执行；Desktop 提供对应恢复页面，不能把旧浏览器页面的按钮数量当作 Desktop 的覆盖证据。

## 请求、执行、记账与回复

实际生产链路为 Desktop `Service` → `NativeOwner` → `OwnerHost` → `OwnedRuntime` → typed execution HTTP API → server 的持久化队列/lease/outbox。UI 的创建成功或 provider 登录成功均不表示 runtime 已启动。

server Appservice 先持久接收事件，再向 homeserver ACK。owner 设备取得 Agent lease 后，各 Room worker 分别 poll、ACK、Start；ACK 不等于已经执行。执行前检查 Room/发言者真实资格、本地策略和账本。每次 provider 调用先持久预留，实际 usage 结算；缺失 usage 保留 unknown hold，后续不能靠重启或修改策略绕过。

租约在运行中续期；请求及工具授权重新核验 device、Agent、binding generation、lease epoch 和实际远端 scope。回复保存为确定输出后进入 server outbox，Matrix 使用稳定 transaction ID。发送重试或重启恢复不能重新调用模型生成同一个答案；旧账号响应和过期 scope 不能覆盖新账号界面。

RuntimeStart 可建立在线连接；没有配置 Room 额度时，每个请求会在推理前拒绝。本文不声称所有配额问题都在点击 Start 时一次性检查，因为启动还承担确定回复的恢复工作。

## 原需求逐项对应

| 原需求 | 当前对应 |
|---|---|
| R01、R02 | owner 创建/绑定 API、永久傀儡身份、持久原命令恢复；Desktop 创建入口 |
| R03、R04 | Project 默认策略与 deny 优先、Room 策略、真实 Matrix 成员和 scope 检查；Desktop 管理入口补齐 |
| R05、R07—R10、R12 | 本地三层账本、独立全局配置、发言者请求/高风险工具策略；无管理员资源审批 |
| R06 | 持久队列 → owner 设备 lease → 本地 Codex → 确定回复/outbox |
| R11、R18 | Project 绑定 Space，Room 成员、上下文、额度及审批独立；Agent 跨 Project 复用 |
| R13—R17 | 新产品入口取消 Fleet；AS 必装；不迁移旧库；不改 Palpo；不可转让 owner |
| R19 | Codex 首个执行器，Desktop 本机 provider 与运行管理 |
| R20 | 普通 Matrix SDK 加密保留；新 Agent 傀儡的加密执行链按用户允许继续延期 |
| R21—R23 | Pasion 自有 Matrix 身份、Hagency-only admission、近期服务器、默认设备名、多账号数据与运行隔离 |

## 验证与审查记录

本轮审查重点是实际调用链、账号/Agent/Room 晚响应隔离、revision 并发、权限控件与服务器授权的一致性、费用持久化、创建未知结果和回复恢复。最终执行结果、构建及实机检查记录在本节末尾追加。

已完成的专项证据包括：无 Room 的全局额度及资源编辑、跨 Agent/Room 禁止旧配置写入、同原 payload 不能换 key、真实 owner HTTP 接口隐藏额外字段、非 owner 不能读写其他 Agent，以及实际 PostgreSQL Project/Room 管理权限复核。

最终回归与日志：

| 检查 | 结果 | 日志 |
|---|---|---|
| Desktop 原生管理 UI 真实注册/组件回归 | 14 passed | policy 复审代理最终执行记录 session 99794 |
| owner Console 单元及 HTTP/runtime 夹具 | 66 passed、1 ignored | `/tmp/hagency-agent-console-final.log` |
| client 全 Console HTTP 回归 | 133 passed、7 ignored | `/tmp/hagency-agent-http-suite-final.log` |
| 本地账本、策略、文件工具与 Codex 适配器 | 52 passed | `/tmp/hagency-agent-local-final.log` |
| 原命令持久化与 suspended 完成状态回归 | 1 passed | `/tmp/hagency-agent-journal-final.log` |
| server PostgreSQL 与真实权限 HTTP | 42 passed | `/tmp/hagency-project-admin-pg-tests.log` |
| client/native 严格 Clippy（lib/tests） | 通过 | `/tmp/hagency-agent-native-clippy-final.log` |
| server 严格 Clippy、OpenAPI | 通过 | `/tmp/hagency-project-admin-server-clippy.log`、`/tmp/hagency-project-admin-openapi.log` |
| Desktop 最终 binary 构建 | 通过，41.45 秒 | `/tmp/hagency-agent-desktop-build-final.log` |

client 全 HTTP 套件第一轮并行执行暴露两项测试夹具时序问题：假 server 返回的 lease 没有像真实 server 一样限制在设备授权内，以及撤销测试固定等待100毫秒。夹具改用短 lease 与有期限地等待持久撤销完成；没有放宽生产权限检查。第二轮完整套件133通过、7忽略。新增 runtime 夹具也修正了严格响应字段和 server 已收到回复但客户端尚未落盘的测试观察竞态；最终两项独立验收通过。

Desktop 普通 Clippy 检查完成，但工程保留现有风格/冗余引用/未使用代码警告，不能标作 `-D warnings` 全工程通过。源代码复核清单更新为141个文件，均有实际文件；包括本轮 client/server 新增源文件，见 `2026-10-07-desktop-code-review.sha256`。

测试 server 只替换 server 容器，保留 DB、Appservice、Caddy 和原配置。最终镜像 `sha256:48d3b21b1e149fe51d5c9fcb0306ab4109b0c2bd0c75946b7e2a16dfc6c76799`；server source fingerprint 为 `a12329681cf7defc144d71892323858b5b438c3c9c697ebbc16af514ac7adb47`。HTTPS readyz200、TLS验证成功，Pasion issuer保持 `https://hagency.local/_pasion/`。部署证据在 server `.run/hagency-local-https-20261007/server-admin-capability-evidence.json`。

最新 Desktop 调试构建已启动。macOS 因临时调试签名变化重新要求可移动宗卷访问；系统日志明确记录 `kTCCServiceSystemPolicyRemovableVolumes` 等待确认。本轮新增管理页面的实际窗口检查须等用户点允许，不能将原生组件测试或此前导航截图写成新增页面实机验收。启动日志位于 Desktop `.run/hagency-local-https-20261007/desktop-agent-completion.log`。没有修改系统 TCC 数据库或绕过权限提示。

## 明确的能力边界

- Codex 当前适配器不能证明 provider 侧的严格单轮 token 上限，Strict 模式拒绝启动；Estimated 模式须 owner 明确选择。预留和总账本限制新调用，但单轮实际费用可能超过预留，不能把它宣传为绝对费用硬上限。
- 未实现 CPU、内存或并发执行的 OS 级硬限制。这些不属于现有 hagency-client 资源模型；没有添加只改变文字、不限制进程的假开关。
- Agent 的模型/workspace 按 Agent 共用；Room 上下文、额度、审批和可选 host 文件根独立。运行中的 worker 使用启动时配置；修改模型资源后应停止并重新启动相应 Room。
- 原生 shell、外部 MCP、网络工具与文件覆盖仍不作为已开放能力。现有可选 host 文件能力只提供 Room 范围内 list/read/create，并继续执行 owner 策略。
- 新 Agent 的 E2EE 执行按用户已批准延期；没有降级普通 Matrix SDK 加密。
- 测试不调用付费 Codex，不发送真实用户 Room 消息或邀请。假 provider 的完整 HTTP 验收证明协议与持久化流程，不能代替用户真实模型账号的付费运行验收。
