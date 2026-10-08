# Agent 运行恢复与显式停止（2026-10-08）

> 2026-10-08 后续需求更新：未配置预算默认不限额，不再阻止请求；既有 Unset 也按不限额读取。本文中“Room 未设额度阻止调用”仅记录修改前的实测情况，不代表现行规则。有限额度、0 额度、显式拒绝、权限和未知费用检查仍生效。当前设计及新验收见 [可用性复核](2026-10-08-desktop-usability-review.zh-CN.md)。

## 当前状态

**最终 Desktop 构建通过，库测试 362 通过、0 失败、1 忽略；Native 库测试 185 通过、0 失败、5 忽略，严格 Clippy 通过。普通重启、快速重启和显式 Stop 排除均已实机确认。** testgeny 已回复原消息 `Hello!`；agent3 能接收请求并返回额度暂停，私聊 Room 仍未设置额度，不能报告为正常推理成功。本文区分身份、已保存配置、实际运行和请求授权。最终版窗口已启动；最近服务器列表、切语言后的保留选择、保存账号无二次授权恢复、中文创建标题和 Chats 选中态均已截图确认。

## 运行意图与恢复规则

主人在 Run 页明确启动一个 Agent 的一个 Room 后，持久化该主人、指定设备、Agent、binding 对应的运行意图，包括显式选择的 estimated reservation、effort 和 host files 开关。重启只恢复这些曾明确启动且没有显式停止的范围，不能枚举全部 active Agent/Room 并自动启动。

**Stop Room 是持久停止该范围的意图。** 该 binding 不应在重启后自动复活；同 Agent 的其他已启动 Room 可以继续或各自恢复。程序退出/升级时停止进程与用户显式 Stop 是不同操作；旧 generation 的租约可能要等截止时间自然失效：正常关闭可以保留已启动范围的恢复意图，但不能因此保存已失效权限或承诺旧进程副作用已回滚。账号切换时先停止旧主人；任何恢复只发生在当前完整身份下，不能在后台恢复未激活账号。

恢复必须重新取得当前 Pasion/SDK 主人证明、验证同一服务器/issuer/subject/MXID、指定设备、Agent 与 binding 状态及实际 Room 权限，重新核对租约和完整本地费用证明。恢复意图不是授权，不能接管其他设备、不自动解除管理员暂停/移除成员、不重置 Agent generation，也不绕过用户配额或工具审批。

模型、workspace、Codex 登录和真实能力目录须已保存且仍有效；不猜默认模型/目录/effort，不为恢复创建第二次 Codex 登录，不把缺资源显示为在线。原已选择参数仍受当前能力检查，失败应显示明确恢复原因，不能永久 Loading。

本轮源码涉及 `native/hagency/src/console/owned_runtime.rs` 中运行意图持久化、设备筛选与 Stop 删除，以及 Native facade 和 Desktop backend 的当前身份恢复调用。生产已冻结；最终 build/test 日志见下文。专项测试不与全量结果重复计数。

## 队列、费用与未知结果

| 服务器 / 本机状态 | 合法恢复方式 |
| --- | --- |
| pending，executionId 为空 | 恢复有效运行范围后处理原请求；不另发替代消息，不把 pending 算模型消耗。 |
| 已 durable 接收，尚未调用模型 | 使用原 inbox / dispatch 身份按现有协议继续；不能新建重复请求。 |
| 已开始且模型结果/副作用未知 | 保留 unknown 与预算 hold，禁止自动重新推理；重启不是清账。 |
| 已知结果、回复待发或网络结果未知 | 仅按原已知结果、稳定 Matrix transaction ID 和 exact content 进行合法 reconcile；不重跑模型。 |
| sent / completed | 保留历史费用、上下文和回执，不重新解释或重复发送。 |
| Room/requester 未设额度、拒绝或待审批 | 显示正确策略阻断；主人显式配置/审批后再按当前请求合同处理，不能默认 Unlimited。 |
| 用户显式 Stop | 排除自动恢复；新请求可继续在服务器排队，但本机不自动开始。 |

三层额度分别是 Agent 总额度、Room 额度和 Room 中请求者额度。Agent Unlimited 不等于另外两层 Unlimited。未设置额度与真实零额度也不是同一含义。策略暂停回复说明请求未获正常模型处理授权，不能将其 sent 状态报告为推理成功。

## 本轮实际证据（手动恢复与重启恢复分阶段）

以下记录来自根代理实机操作与只读状态核对，未为文档新增测试消息：

1. 服务器已有两条用户 hello 事件 pending。testgeny 的 lease 为 epoch 11，截止 `2026-10-08 02:03:50 UTC` 已过期。Agent 身份/绑定仍在不代表运行租约有效；此前仅启动过进程而没有重启恢复不能处理这些排队请求。
2. 根代理手动以 reservation 15000、low effort、host files false 恢复 testgeny。原 hello 请求得到 `Hello!`，答案为 sent，GUI 已再次确认回复、👀 和用量图标。该结果证明手动恢复能处理原 pending；这一步仅证明手动恢复，自动恢复另有下述重启证据。testgeny 的正常回答与 agent3 的额度暂停分别记录，不将两者混为推理成功。
3. agent3 为 active，默认指定当前设备，但没有 lease，并且保存的模型与 workspace 缺失。不能将默认绑定设备解释为资源自动可用。
4. 根代理补保存 agent3 的模型及私有 workspace 后 Start。用户 Agent total 为 Unlimited，但 Room / requester 仍为 Unset；两条 hello 实际得到额度暂停 `paused_reply`，且已 sent。**这是策略暂停反馈，不是正常模型推理成功。** 用户随后在 GUI 将 requester 显式设置为 Unlimited，但 Room 仍为 Unset；用户尚未回答 Room 额度选择，根代理没有放行 Room。不得替用户选择、默认放行或重跑已完成请求。


### 重启与 Stop 排除的实际进展

- 持久运行意图共 3 条：保存的 reservation 分别涉及 600 / 15000，effort 为 low，host files false。普通重启记录 `3 eligible / 3 start accepted`，testgeny / agent3 lease 分别为 epoch 14 / 3 且有效，两 Agent 的 UI 均显示 running。这里仅证明已保存运行范围恢复，不代表 agent3 的额度暂停变成正常推理。
- 根代理显式 Stop testgeny 的 Project Room binding `bnd_e94173cb255f88d2e670fde13c66dcf47951540ac525b39ce8e8ef4ac2b5565a` 后，持久意图只剩 2 个 DM。快速新 build 5 重启记录 `2 eligible / 2 start accepted`，两个 Agent lease 为 epoch 15 / 4 并持续有效；没有把已停止项目包含在恢复 eligible 列表中。
- 首轮快速重启曾遇到同一稳定设备的旧 generation lease busy。修复只对恢复流程使用最多 35 秒、每 1 秒重试的等待，并在重试时重验 fresh 授权与执行设备等 gate；不是强制 takeover 或延长过期 lease。fake positive-generation busy 回归两次通过。
- Native 库 **185 passed、0 failed、5 ignored**，严格 Clippy 通过（`/tmp/hagency-runtime-recovery-native-final.log`）。Desktop 最终 `cargo build --locked --bin rinx` 成功；`cargo test --locked --lib` 为 **362 passed、0 failed、1 ignored**，日志位于 Desktop `.run/hagency-local-https-20261007/desktop-runtime-recovery-{build,tests}-final2-20261008.log`。
- 最终版启动日志再次记录 `3 eligible / 3 start accepted`，GUI 两个 Agent 为 Running、agent1 为 Not running，testgeny 私聊 banner 与 Chats 高亮一致。
- 快速重启及保存账号恢复后，该 Project Room 的 GUI 明确显示 “This Room is not running. Other selected Rooms may still be active.”；两个 DM 继续运行。随后以 reservation 15000、low、host files false 手动恢复项目 Room，刷新确认 “Running in this Room”。未知结果、撤权或不同设备的实机分支未为本次测试制造。

## 最终验收清单（根代理补证据）

| 验收项 | 所需证据 | 当前结论 |
| --- | --- | --- |
| 明确 Start 的范围在进程重启后恢复 | 原主人/设备/Agent/binding；新有效 lease、相同保存参数、UI 非永久 Loading | 普通 3/3、快速 2/2 accepted、有效 lease 与 GUI running 已确认；最终构建通过 |
| 显式 Stop 的范围不恢复 | 重启前 Stop 记录、重启后无该范围 polling/start；其他范围不受误伤 | 持久意图 3→2，重启 eligible 为两个 DM；GUI 精确未运行状态及随后手动恢复已确认 |
| 同 Agent 多 Room 独立恢复 | 只恢复原运行意图、上下文/三层费用隔离 | Project Stop 保留 DM 恢复已见；未额外调用模型验证费用 |
| 授权/资源/指定设备不合法时安全停住 | 明确错误；无模型/新提示/副作用；不自动 takeover | Native 自动化覆盖，实机未制造撤权/改派 |
| pending 原请求 exact-once 处理 | 原事件/dispatch；新增唯一 execution、sent event，非替代测试消息 | 原 hello 已见正常答案；未额外重放旧请求 |
| unknown 不自动重推理 | 原 hold 与 started history 不变；无新增模型 call | 自动化覆盖；未制造实机 unknown |
| completed/sent 保持 | 重启前后原 txn/body/event/费用与上下文不变 | 原正常/暂停回复重启后仍显示；完整账本对照未另制造 |
| 三层额度阻断与恢复 | Unset 产生策略暂停；用户显式设置后仅合法请求运行 | 两条 paused_reply sent 已确认；requester Unlimited / Room Unset，正常处理待用户明确 Room 额度 |
| 账号切换不启动旧账号 | epoch / profile / device fence，后台无未激活主人运行 | 添加账号页返回保存账号无需二次授权已确认；不同账号/服务器实机未覆盖 |

最终保存账号恢复后的只读快照：testgeny / agent3 lease epoch 为 18 / 7，均有效；testgeny Agent spent 53160、held 0，DM spent 35192、Project Room spent 17968（均 held 0），未清账或额外重跑旧请求。当前三个运行意图均为 reservation 15000、low、host files false。

源码复核快照见 `2026-10-08-desktop-code-review.sha256`。构建 binary 和关键 UI 源码摘要见 Desktop `.run/hagency-local-https-20261007/desktop-usability-build-manifest.json`。上述 lease epoch 为当时只读快照，后续恢复可继续递增；不公布 token 或私有配置。尚未覆盖的实机分支保留为限制，不能用测试通过替代。

关联说明：[执行设备](2026-10-08-agent-execution-device.zh-CN.md)、[实机 Agent 流程复审](2026-10-08-desktop-real-agent-flow-review.zh-CN.md)、[界面可用性复核](2026-10-08-desktop-usability-review.zh-CN.md)。
