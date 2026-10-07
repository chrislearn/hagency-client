# Desktop 实机 Agent 流程与原 hagency-rs 对照复审

本文核验对象为 `chrislearn/hagency-desktop`、`chrislearn/hagency-client` 与 `chrislearn/hagency-server`。`hagency-org/hagency-rs` 只作为旧版交互参考，不在旧仓库实施修改。测试使用现有主人 `@chris:hagency.local` 的 `testgeny`，不向其他人的 Room 发送测试消息。

## 实机发现与处理原则

1. Agent 身份已经 active、默认主人私聊已建立，执行实例也指向本机；这些条件不等于 Codex 已经能处理消息。资源页面仍是空模型和空工作目录，独立 Codex 目录显示未登录。服务器已收到两条主人私聊请求，并持久化为 pending，没有 started 执行、租约或回复 outbox。失败发生在本机执行配置阶段。
2. 用户本机 Codex 已登录。此前每个 Matrix 主人使用另一个 CODEX_HOME 的设计导致看不到已有登录，强迫额外 OAuth。app-server 没有规定必须重新登录。改为支持明确选择已有本机 Codex 账号，凭据仍由 Codex 管理，Hagency 不读取、复制或返回 auth.json/token。Matrix 主人的账本、模型配置和工作目录继续独立。
3. 共享 Codex 登录不能意味着继承不受控制的 MCP、hooks、skills、通知脚本或自定义提供商配置。本机安装版本与网站文档的命令选项存在差异；实际探针发现空表 CLI 覆盖不会清空原 MCP 表。必须验证真实生效配置，不能靠 `mcp_servers={}` 声称禁用工具。不能退出用户日常 Codex 账号来实现 Hagency 的断开关联。
4. 原 Agent 页面把身份、执行实例、Room 管理、退役和全部资源配置堆在同一长页面。改成互斥的 Setup、Codex resources、Budgets、Run and approvals；资源选择由真实模型目录提供，工作目录提供可信的私有目录准备按钮，运行页说明缺失的前置条件。
5. 已完成的服务器身份可能还对应本地 pending 创建意图。恢复必须查询原请求，保留原幂等键；不能按显示名称删除日志或再次创建。恢复区仅用于确有 unresolved 请求的列表页，创建详情不重复展示大块恢复提示。
6. 从联系人打开主人私聊，左侧错误地保持 Agents 选中；私聊右侧错误地显示“邀请 Agent 加入 Project Room”，导致出现 Project 登记错误。联系人进入消息页应切换 Chats，普通直聊隐藏 Project Room 的 Agent 邀请操作。

7. 完成 Project 创建之后新建 Room，错误复用已完成的 Space 意图，造成提交按钮被禁用。已完成意图仅作历史确认，新的表单使用独立请求；未知意图仍保留原键，禁止重复创建。
8. 已创建私有 `Desktop flow check` Project 和 `Agent flow check` Room；SDK 创建、注册、树展开与从 Room 邀请现有 testgeny 已实机成功。服务器确认 Project/Room/binding active，主人、Agent 和 service 账号三人 joined。右侧成员栏初次读取的单人结果未更新；成员事件现在触发有账号与 Room 校验的独立信息刷新，保留正在提交的邀请意图。
9. 非 Palpo miniapp 仍保留应用详情、授权、安装/导入入口；打开独立 miniapp 窗口不得改掉底层 Project/Chat 导航，关闭后仍回到原工作区。实机只核对 Article Studio 入口与授权说明，未授权发布或上传。
10. 仅终端登录识别成功不足以确认打包 Desktop 完成。首次 UseLocal 实机失败，静态错误码为 `local_codex_configuration_unsupported`；原因是实际私有目录在 HOME 下，祖先配置校验误拒 `~/.codex/config.toml`，而此前 `/tmp` 探针没有这一祖先。仅放行 canonical 路径精确等于用户明确选定 Codex home 的配置，其他 Project/workspace 配置仍拒绝；真实 HOME 布局探针通过，再以 Desktop 重新点击验证。

## 与旧版的比较与取舍

| 方面 | 旧 hagency-rs | 当前 Desktop 的取舍 |
| --- | --- | --- |
| 本地资源配置 | NativeResources 展示配置、来源和余量；新建页按模型、推理强度、预算组织 | 沿用清晰字段、真实选择与配置前置条件；加入本机 Codex 登录复用 |
| 模型选择 | SearchSelect 与真实资源观测 | 使用 app-server 模型目录，保留已有选项并采用实际支持的推理强度，目录不是推理成功证明 |
| 预算展示 | 明确展示余量和未知值，不能把 null 当作 0 | 保留 spent/held 的真实账本数据；Agent、Room、requester 三层同时执行，不重置已消费或未知预留 |
| Agent 身份 | 旧 Fleet、Engagement、资源发布与席位机制 | 遵从新需求：服务器全局傀儡、永久主人、无需 Project/Room 创建条件，不恢复旧审批或转让机制 |
| 执行设备 | 旧版的资源与发布流程较重 | 一个 Agent 暂绑定一个执行实例，指定稳定设备身份；租约不能绕过实例分配 |
| 导航与聊天 | 旧控制台管理页与聊天分离 | Projects 下展开 Rooms；Chats 收纳直聊，Room 信息在右列，Contacts 与非 Palpo miniapps 保留 |
| 错误与恢复 | 独立数据读取失败不应清空其他部分；未知结果有恢复操作 | 未登录、配置缺失、未知费用、权限拒绝分别解释；不将 raw JSON 或通用“Verified”作为就绪结论 |

## 实机验收范围

需要分别验证：本地 Codex 账号识别、真实模型选择、现有目录/私有目录准备、配置保存、三层额度、主人 DM 启动、实际短消息推理与 Matrix 回复、停止后不再处理新请求，以及重新启动不重放已完成请求。Project Room 邀请与成员管理不得借测试突破主人/Project/Room 权限。涉及永久退役、删除或更换实际账号的数据，不为截图验收额外执行。

验收区分真实模型调用、数据库/协议测试和仅 UI 入口核对，不以编译或 fixture 冒充实机成功。

## 参考依据

- [Codex 登录缓存与凭据存储](https://learn.chatgpt.com/docs/auth)：已有登录缓存可以复用；凭据由 Codex 的文件或系统凭据存储维护。
- [Codex app-server 协议](https://learn.chatgpt.com/docs/app-server)：通过 account/read 识别登录，model/list 获取模型选项，实际 turn/completed 状态确认推理结果。
- 旧版参考：`hagency-org/hagency-rs/mockup/components/NativeResources.jsx`、`NativeAgentDetail.jsx`、`mockup/app/resources/new/page.jsx`。

## 最终验证记录

2026-10-08 已完成第一阶段真实闭环：

- Desktop 在同一 Matrix 账号下明确选择已有本机 Codex 登录，实际页面显示已关联，没有再次 OAuth；真实 catalog 默认模型为 GPT-6.1-Sol，推理强度 low。
- 为现有 testgeny 准备并保存主人与 Agent 隔离的私有 workspace。执行实例仍在原稳定设备，不新建 Agent、不改永久主人。
- UI 保存 Agent 总额度 100000、主人 DM Room 额度 60000、主人在 DM 的额度 60000，均 lifetime；allow requests、deny high-risk tools。运行选择 estimated，单请求 reservation 20000，host files 与 lease takeover 未开启。
- 两条原 pending hello 和一条 `Reply only: HAGENCY-DESKTOP-OK` 实际执行并返回；Desktop 可见真实 Agent 回复。服务端 completed=3、outbox sent=3（均有 Matrix event ID）；本地 replied=3、calls settled=3。三层 spent 均 13109，held 均 0。
- 输入/输出结算分别为 4355/11、4355/11、4365/12；缓存输入没有再重复累计。配置保存未清零账本。
- 随后通过 UI 停止 DM runtime。没有以手工清除 unknown hold 或回复 outbox 的方式通过验收。

真实测试进一步发现此前 DM 顶层消息每句独立 Codex thread，并且全部回复为 Matrix thread 卡片。第二阶段已修复并重新验收：新 top-level owner_direct 以真实 Room ID 作为稳定内部上下文键、普通消息回复；显式线程和讨论 Room 保持事件线程隔离。已发送/unknown 历史 intent 的 root/body/transaction ID 不重新解释。无需 schema 或 API 字段变更，不重置现有数据库。连续记忆、停止/重启与不重复发送的真实结果见下文。

第一阶段验证：Desktop lib 331 passed/1 ignored；UI 22/22、i18n 3/3；agent-local 最终 55 passed/1 ignored、runtime 28、provider 5 passed/3 ignored、原生闭合路由 1、Pasion/device wire integration 1；实际 HOME 布局账号/模型 probe 1/1 和恶意 MCP/notify 零推理 probe 1/1。服务端连续 DM PG 46、OpenAPI 51+4、严格 Clippy 通过。第一阶段 paid 推理证据为上面三次；第二阶段新增的一次实际回复见下文，账号/模型探针没有调用推理。


## 第二阶段实机发现：连续私聊与授权期限

该阶段首先实际收到第一句普通消息回复 `SAVED`；对应 input/output 为 4367/6，累计四次结算为 17482 tokens，三层 held=0。当时第二句询问测试词的原请求仍 pending、未 started，没有重复调用或费用；当时不能将连续记忆验收视为通过。修复后的恢复结果见后续验收。

UTC 22:08:01.665 的 Matrix whoami 401 后，01.694 的 SDK/Pasion token 刷新、01.702 whoami、01.712 Hagency session renew 均为 200。首个 401 是正常自动刷新的一环，并非用户应再次登录的证据。02.261 的 lease release 为 401；epoch 2 随后失效。未发现明确 429/403 或 renew 401。

相关代码问题是：server 严格把 lease 截止时间限制在当前授权 proof 的期限以内，客户端 heartbeat 原本固定每五秒更新，且使用旧授权快照与旧 lease deadline。授权临近到期时即使 SDK 稍后成功刷新，旧租约也可能先结束。修复采用真实 SDK 提前刷新、有限续期缓存减轻 session 锁竞争、按实际 lease deadline 调度心跳；不得延长未经服务器确认的权限，也不得以忽略到期或自动接管其他设备来规避。跨授权期限的真实运行已通过，见后续记录。

管理页面另增加只读等待上限：仅界面停止无期限 Loading，后台原请求继续完成；过期结果通过账号和 request fence 丢弃，不取消可能正在做 grant/device 登记的 bootstrap，不自动重发变更或 unknown 请求。

### 第二阶段连续记忆已通过

修复版 UI 启动原 DM 后，原先排队的“测试词是什么”请求实际回复 `orchid47`。服务端 completed/started/sent 均为 5，前三个历史事件线程与两个新普通私聊请求保持各自的原事务；本地 settled/replied=5，三层 spent=21879、held=0，原稳定 Codex 上下文 ID 保持不变。没有再发一条替代请求来冒充恢复成功。

Epoch 3 跨启动时的 Hagency 短授权期限后，后续 renew 仍为 200，设备 generation 不变。随后 UI 明确 Stop，lease release=200。该短证明期限测试不等同于 OAuth 五分钟 token 刷新测试；后者另行实测，见下一节五分二十五秒运行记录。

续期最终实现：每五秒以内复用尚余十秒以上的已验证 proof；正常续期后返回的新证明若不足二十秒，才让 SDK 主动刷新一次并重新取得服务器 proof。正常三十秒 freshness window 不触发无谓 OAuth rotation。心跳取五秒与真实剩余期限一半的较小值，新 Lease 到手后使用新期限检查设备；身份、generation、账号 epoch 校验仍严格执行。慢网络超过当前 lease 时仍安全停止，错误可见，不自动接管或复活过期租约。

续期修复阶段库测试（含新增 SDK 与界面读等待回归）334 passed/1 ignored；共享授权 9、完整 runtime 29、管理/UI 25、i18n 3、真实 SDK OAuth HTTP roundtrip 1 通过，native 严格 lib/tests Clippy 通过。

### 停止、排队、重新启动已通过

UI Stop 后有成功的 lease release，运行器显示不在本机运行。主人随后在同一 DM 发送一句短消息；服务器确认 pending=1、executionId 为空，旧 completed/sent=5 保持不变，本地费用未增加。正常 Start（未选择 takeover）后仅执行这条原请求一次，实际回复仍为 `orchid47`。总 completed/started/sent=6，settled=6，三层 spent=26299、held=0，稳定上下文仍唯一。

最后两项界面校正：启动恢复已选私聊时使用 exact Room 的 direct 属性切换 Chats，不能留在 Projects 树；元数据尚未加载时仅保留有账号 epoch 的短暂恢复意图，用户主动选择导航后不得抢回。直接进入 Run 页自动只读获取保存的模型、Codex 账号状态和真实模型能力，加载期间显示检查中；不能把尚未读取的 false 当未登录，不能制造 medium/high 的假目录，也不再次要求 UseLocal 或重写配置。

### 真实 OAuth 到期与 Project 服务已通过

UTC 22:31:36 至 22:37:01 连续运行约五分二十五秒，epoch 4、device generation 8 保持不变。22:32:37.048 实际 SDK/Pasion token refresh 为 200，随后 session 与 lease 继续成功续期，没有依赖重新登录或设备重新登记。长时阶段结束时，原六条 completed/sent 保持不变，账本没有重复消费。

同一 Agent/设备/instance generation 1 下另启动已绑定的私有 Project Room。UI 提及选择器生成 canonical Matrix mention 后，实际收到线程回复 `HAGENCY-ROOM-OK`。Project 仅一条 started/completed/sent；其上下文以 `$event` 为根，与 DM 稳定 Room 上下文独立。总 settled=7；Agent spent=30738（DM 26299 + Project 4439），两个 Room 与 requester 分别保持各自消耗，所有 held=0。Project Room/requester 都为 60000 lifetime，Agent 总额度仍 100000，没有新增执行实例或修改永久主人。

随后分别通过 UI 停止 Project Room 和 DM。UTC 22:41:16.869 lease release=200；七条 completed/sent/settled 保留，无 pending/unknown，全部 held=0。Project 私有 Space/Room、成员与绑定保留，不以删除测试数据或重置账本完成检查。

Budgets 最终采用独立紧凑 Room 选择器，避免选 Room 层后把 Pause/Leave 管理卡混入配额页。Requester 仅在请求者层显示，切 Room 仅读取政策并清除旧 revision 展示，不自动保存；Agent 总层始终全局。由 Agent 设置打开讨论 Room 时，同时切到 Projects 工作区并展开对应 Project，标题用精确 SDK Room 名称缓存；直聊仍切 Chats，导航动作附带当前账号 identity。

首次从 Agent 打开 Project Room 时，项目树可能尚未初始化账号身份。最终修复只在当前账号导航校验通过后初始化精确 Project 展开状态；异步列表加载不重新施加展开意图，用户随后折叠不被抢回，换账号清除旧树。首次展开、手动折叠及换账号回归通过，项目树组 9/9。

最终冻结源码完整 Desktop lib：338 passed、0 failed、1 ignored；最终打包二进制已生成。原生共享授权 9、runtime 29 和服务端 PG/OpenAPI 验证结果保持有效。复核覆盖账号隔离、单设备租约、原始幂等意图、三层账本、未知费用保留、私聊与 Project 独立上下文及 UI 导航，未以源码哈希替代语义审核。三个仓库 `git diff --check` 通过。

### 最终窗口检查与用户原消息恢复

2026-10-08 06:53（Asia/Shanghai）用户在 Project Room 提及 testgeny 并发送“自我介绍一下”。检查确认该原事件为 pending、未 started，lease epoch 4 已停止；原因是开发版更新前明确停止运行，最终权限放行后尚未恢复，并非消息丢失或 Codex 再次登录。

通过最终版 UI 恢复原 Project Room，取得 epoch 5 后仅执行该原请求一次，06:56 实际送达中文自我介绍。Desktop 的线程卡片与完整线程内容都已打开验证。累计 server completed/sent/真实 Matrix event ID 均 8，本地 settled/replied 均 8，无 pending/unknown。新增 input/output=4411/74，Agent spent=35223，Project Room/requester spent=8924，DM 仍26299，held=0，未修改限额或原意图。

最终版直接进入 Run 已自动读取保存的资源，真实目录为 low；Budgets 的 Room 层只显示紧凑 Room 选择器，未混入暂停/退出操作。打开 Project Room 自动展开 Projects 并显示正确 Room 名和3名成员；打开主人 DM 自动切 Chats、显示 testgeny 和2名成员。

随后也通过 UI 恢复主人 DM，Refresh 显示“Ready to process requests in this Room”。Project Room 与 DM 均保持运行，未发额外测试消息；桌面最后停留在用户原消息的中文回复线程。此时不再重编译、重新签名或停止运行。

### 后续表单样式与返回导航复核

按用户后续要求，Project、Agent 管理表单的 20 个选择框统一复用 Rinx 选择控件；选择框与文本框同为 40px 高、4px 圆角，统一字号、左右内边距和主题颜色。菜单在选择框下方展开，菜单项为 36px 高；实际模型菜单、模型选择和工作区输入框已在 Desktop 核对，未修改模型或保存的资源配置。

用户随后要求恢复内页顶部的返回列表入口，因此覆盖此前移除 Back 按钮的界面决定。标题左侧增加带左箭头的“返回列表”按钮，只在详情或表单页显示；列表页隐藏。Agent 详情和创建页、Project 详情和创建页已逐一实测返回到对应列表，创建测试只打开表单，没有提交新实体。新增 Room 和 Agent 接入 Room 的表单也共用这套内页导航。返回保留已有实体、待确认创建意图、配置和运行任务，清除未执行的详情导航及延迟私聊检查；忙碌请求完成前按钮禁用，未取消或重放后台写操作。

修改后的管理/UI 测试 26/26 通过（含真实已注册列表卡片和返回按钮的点击动作回归），Makepad 注册无脚本错误，开发版编译通过；新版窗口已启动，未新增登录授权。

本次更新后，原 testgeny 主人私聊与 Project Room 均恢复至 Ready，原设备取得 lease epoch 7；随后从运行页点击返回列表，服务端租约仍有效，没有停止 Agent，也没有发额外测试消息。
