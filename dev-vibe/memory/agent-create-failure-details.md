# Agent 创建失败的诊断信息

更新：2026-10-22。

## Main 提交验证（2026-10-23）

- 用户授权将当前 Client 改动直接提交并推送远端 main。已将改动接到 origin/main 的 53011a0d 后，合并 en/zh-CN 词典的追加冲突，两边新增翻译均保留（2230 项）。
- 合并后桌面完整库：575 passed、0 failed、2 个原有 ignored。样式扫描 264 文件/0 违规、翻译无缺失、主题检查、native-owner-client 生产调用边界、引擎格式及 diff 检查通过。
- 合并后请求状态单测 4 项、owner HTTP 生命周期回归 1 项通过，覆盖查询不提交、未知结果不可收起、明确拒绝可收起和原幂等键恢复。
- 构建使用 D:/BuildCache/hagency-client/{cargo,target,tmp}，cargo metadata 已核实 target_directory。日志为 D:/BuildCache/hagency-client/client-main-push-*。

## 后续整体流程整理

- 用户指出状态气泡顶部Agent名称冗余。本轮移除名称及其后的空行，仅显示诊断说明与下一步；同步调整已有点击回归，确认不含Agent名称。1项点击回归及diff检查通过；D盘bin构建50.84s成功。正常关闭旧窗口核对保存并等待进程退出后，新版PID71356使用原D:/BuildCache/ui-review-data自动登录，stderr为空。实窗点击abcd Needs attention确认气泡直接显示缺模型说明，没有名称/空白标题行。仅导航/查看说明，无业务写操作；日志D:/BuildCache/hagency-client/agent-bubble-no-name-{tests,build}.log及agent-bubble-no-name-app.{stdout,stderr}.log。未提交/推送。

- 用户要求Needs attention可点击弹气泡说明。本轮将runtime_badge从静态DeviceBadge换成同高度的共享RinxGhostButton；点击通过全应用CalloutTooltip展示Agent名、runtime_service_description及本地控制失败原因，沿用诊断翻译/过滤并使用语义色，显示于标签下方。其他状态也可点击解释；使用已有气泡关闭交互（点击/滚动）。点击前检查行映射与本地执行设备，避免PortalList回收行串Agent。新增点击诊断回归覆盖缺模型/授权/房间worker、只读不跳转/不启动与陈旧行。管理界面53项测试、样式262文件/0违规、主题及diff检查通过；D盘bin构建1m30s成功。正常关闭旧窗口确认保存并等待旧进程结束后，沿用D:/BuildCache/ui-review-data启动新版PID67992，自动LoginSuccess，stderr为空。实窗点击abcd Needs attention显示未配置模型及账号/保存模型建议，点击空白关闭；点击abc Not running显示其自己的说明。随后检测到用户输入，停止后续窗口操作。无服务启动、配置、请求提交或消息发送。用户在本轮开始前已自行创建wewee，重启前已有三张卡与无待确认请求，不能把此状态变更归于本轮操作。日志D:/BuildCache/hagency-client/agent-runtime-bubble-{tests,build}.log及agent-runtime-bubble-app.{stdout,stderr}.log。未提交/推送。

- 用户要求移除My Agents卡片下方Mine标识。本轮删除Cards模板ownership_badge及其渲染，剩余服务范围/设备/运行状态标签顺排；更新已有布局检查的标签集合。管理界面52项测试和diff检查通过；D盘bin构建1m08s成功。正常关闭旧窗口确认保存、结束并等待旧进程退出后，沿用D:/BuildCache/ui-review-data启动新版PID65116。实窗确认两张Agent卡均无Mine，标签左移且无残留空隙。仅导航，无业务写操作。证据日志D:/BuildCache/hagency-client/agent-remove-mine-{tests,build}.log及agent-remove-mine-app.{stdout,stderr}.log。未提交/推送。

- 用户明确My Agents只管理自己创建的Agent，不需要发现其他用户公开Agent。本轮删除Browse public Agents入口、公开目录/选择/详情及其状态、事件、回调处理，清理7条专属中英文翻译；继续使用OwnerOperation::Agents返回当前身份所属Agent，自己创建的public/private均保留，服务范围设置不变。移除旧目录测试分支并收紧列表填充布局断言。管理界面52项通过、翻译2219项/0缺失、样式262文件/0违规、主题及diff检查通过。D盘bin构建58.84s成功；正常关闭旧窗口核对状态已保存、结束并等待托盘进程退出后启动新版PID74932，沿用D:/BuildCache/ui-review-data并自动LoginSuccess，stderr为空。实窗1280×800确认入口/公开目录均消失，首张自己的Agent卡紧接提示区（y193），没有残留留白。仅导航，没有提交请求/启动服务/发送消息。日志D:/BuildCache/hagency-client/agents-owned-only-{tests,build}.log及agents-owned-only-app.{stdout,stderr}.log。未提交/推送。

- 用户继续要求把Check status/Retry request放到Create标题后。本轮为共享InlineNotice的标题行命名header，仅请求卡覆盖为可换行横排、标题Fit，把操作按钮移入标题行；宽屏同行紧随标题，窄屏自动换行。管理界面52项和共享提示1项测试通过，布局回归补充按钮位于卡内、宽屏标题同行和单请求高度<110px检查；原生样式扫描262文件/0违规，diff检查通过。D盘bin构建52.68s成功；正常关闭旧窗口确认状态保存后结束托盘进程，单实例退出竞态结束后成功启动PID18624。实窗1280×800确认Check status/Retry紧跟Create标题、说明在下方、首张Agent卡上移至y237。仅导航，没有提交请求或启动服务。证据日志D:/BuildCache/hagency-client/agent-notice-header-{tests,feedback-tests,build}.log及agent-notice-header-app.{stdout,stderr}.log。未提交/推送。

- 后续用户反馈 Requests 区域留白和滚动条内缩。本轮去掉固定 240px 请求 PortalList、外层 Section/重复标题说明，改为按内容 Fit 的 FlatList（无内部滚动条），紧凑提示 padding10/spacing4。列表容器的右侧20px留白移至卡片自身；Project/Agent 的 PortalList 视口铺到面板最右边，内容仍保留20px gutter。保留原底部16px留白。新增420/1080/1440宽度的真实布局回归，检查单请求无预留空白、宽屏高度<150、视口右边缘与内容gutter。
- 本轮布局验证：管理界面52项测试通过；样式扫描262文件/0违规、主题与diff检查通过；D盘bin构建54.50s成功。正常关闭旧窗口并确认状态保存后重启，新版PID73052仍使用D:/BuildCache/ui-review-data，自动LoginSuccess，stderr为空。1280×800实窗进入My Agents，确认单请求紧凑显示、第一张Agent卡上移到y271、两张Agent卡完整可见，不溢出时无滚动条；视口贴右由420/1080/1440布局测试覆盖。仅切换页面，没有查询/重试请求、创建Agent、启动服务或发送消息。日志为D:/BuildCache/hagency-client/agent-layout-compact-{tests,build}.log与agent-layout-compact-app.{stdout,stderr}.log。未提交、推送或发布。

- 用户明确要求整体流程顺畅、错误明显、全应用较统一，指出上一轮通用“未确认”前缀和请求下拉框让人无法判断下一步。以前的前缀方案已被本轮替换，不再当作当前产品行为。
- 新建不依赖全局 journal 加载成功，也不被其他 pending/rejected 请求或未绑定房间的 Agent 阻断。引擎只阻止相同 payload 以新 key 复制未知请求，独立创建允许；创建后引导模型配置和服务，房间可选。
- 请求逐条显示卡片及查询/重试按钮，明确拒绝支持收起（保留审计），查询不触发 mutation。引擎保存 rejected/errorCode，重试发送前先落盘 unknown，网络不确定状态不可收起。
- 共享 InlineNotice 提供语义色、标题、说明、可展开详情和可选下一步/关闭按钮；已接入管理、模型保存、登录服务器错误、房间加载，全局通知共用色板。新表单/切页/编辑清除上次反馈；移除模型保存的重复全局报错。
- 发现 Server origin/main 已有 f48717f（完整服务范围支持），本地原 b0782a0 落后。已 fast-forward，1.99.0 D 盘构建成功（9m33s），以原配置重启为 PID58968；/readyz HTTP200，startupRoundtripConfirmed=true。不删除 visibility、不把私有 Agent 降级成公共，不修改 schema.rs 或生产部署。
- 完整库首轮 568 passed/2 原 ignored；HTTP 1 项、状态单测 4 项通过；最终增量及实窗验收待后续追加。详见 ../agent-creation-flow-2026-10-22.zh.md。
- 最终增量：管理界面 51 passed、状态单测 4 passed、真实 owner HTTP 1 passed；引擎与桌面 lib 严格 Clippy（-D warnings）通过，样式扫描 262 文件/0 违规、翻译 2226 项/0 缺失、主题与 diff 检查通过。最终 bin 构建 54.48s。
- 已正常关闭旧窗口并核对持久化成功，再结束其托盘驻留。最终新版 PID50524 使用原数据目录，自动 LoginSuccess，stderr 为空。实际窗口验证黄色逐条请求卡与查询主按钮、查询不提交、进入 New Agent 清除旧提示、空名称本地校验显示红色提示卡、详情展开显示错误码、关闭提示恢复干净表单。停止在空白 Create Agent 页面。
- abc/wewee 两条旧 unknown 记录的数量、状态和原写入时间保持不变；本轮没有创建/恢复真实 Agent、选择模型、启动服务或发送 Room 消息。查询为只读，空名称校验没有发送请求。证据保存在 D:/BuildCache/hagency-client/agent-flow-verification.json 和 agent-flow-* 日志。未提交、推送或发布。

## 早先仅补充错误码（历史）

- 用户从 Cloud 聊天提供桌面 Create Agent 截图：名称 abc，只有通用失败提示。实际代码位于合仓后的 Client `crates/hagency-app/src/hagency/ui.rs`。
- 原 `visible_error` 只有 Provider 类错误显示错误码，其余全部经 `user_error` 丢失码；未知错误统一变成 Refresh and retry。现在所有符合 owner API 合约（1–80 字节、小写 ASCII/数字/下划线）的机器错误码都会显示，原始响应、路径、凭据文本不显示。
- 增加名称/参数、幂等键、本地请求记录、响应协议、appservice 未就绪、服务端数据库及配置类错误的中英文可操作说明。创建/绑定/恢复失败提示检查原请求后恢复，保留原幂等键；公共 Agent 查询也显示相同诊断。
- 只读确认当前运行使用 `D:/BuildCache/ui-review-data`。其中 abc 的 agent.create 本地记录是 unknown，creation 为 null，没有记录原失败码。没有自动提交/恢复创建请求，不能仅凭截图断言服务端已拒绝或已创建。
- 发现本地 Server 源码 `crates/agent-service/src/domain.rs::CreateAgent` 使用 deny_unknown_fields 且仅接受 displayName/idempotencyKey；客户端会发送 visibility，存在接口版本不兼容，可导致 invalid_arguments。运行中服务端位于 `D:/BuildCache/hagency-server/target/debug/hagency-server.exe`；本轮没有修改/重启服务端，尚未用原请求复现确认实际错误码。
- 验证：管理 GUI 50 项测试通过（含新增诊断测试），翻译检查 2192 项/0 缺失、主题字面量及 diff 检查通过。Cargo/target/tmp 由 tools/dev.ps1 固定到 `D:/BuildCache/hagency-client` 并通过 metadata 核对。
- 桌面 bin 构建成功（57.03s），旧可执行文件改名保留为 D 盘 target/debug/hagency-before-agent-create-feedback.exe；关闭旧窗口后核对状态持久化成功，再结束托盘驻留进程。最终新版 PID78612 使用原 `D:/BuildCache/ui-review-data`，自动 LoginSuccess，stderr 为空，startup recovery 为 0 eligible bindings。
- 实窗只读打开 My Agents，显示 1 unfinished request。abc 记录最后写入时间仍为原 16:26:52、state=unknown，没有提交或恢复请求。未实际触发新失败来验证这次错误码；测试已覆盖静态错误映射及非法原文过滤。用户开始操作窗口后停止进一步输入。日志为 D:/BuildCache/hagency-client/agent-create-feedback-{tests,build}.log 及 agent-create-feedback-final.{stdout,stderr}.log。未提交、推送或发布。
