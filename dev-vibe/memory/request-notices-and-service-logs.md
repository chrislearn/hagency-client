# 请求提醒与服务诊断

2026-10-20 用户反馈提醒重复引用原消息、固定中文且离线/排队文案不准确，myagent Start 后 provider_authorization_lost，希望查看日志及其他设备在线状态。

- Server request_notices 原为固定中文通用猜测；m.in_reply_to 触发客户端重复引用。Server 改为结构化 org.hagency.request_notice(version/kind/deviceState/deviceName/observedAtMs/requestEventId)，保留线程归属但移除回复引用；独立关联字段保留请求定位，m.notice 仍不进入推理。无 schema.rs 修改。
- Client room/request_notice.rs 在显示时按界面语言渲染，精确兼容全部旧中文请求提醒；隐藏这些提醒的引用块，普通回复不受影响。状态提醒绕开普通 Agent 回复组件，避免其重新覆盖本地化正文；状态行不使用旧内容缓存。通知预览也兼容旧文案。
- 服务端同一授权设备/有效会话和 lease、15 秒 configuration heartbeat 分开判断 ready/not_running/offline/unobserved/unassigned。只描述发送时观察事实，不再声称有其他任务或保证恢复后处理；排队有超时。Client Another device 列表每 10 秒读取已有配置状态，只在 Agent/设备 generation 匹配时显示在线/无近期心跳/未观察到/未知，在线不承诺 Agent 已就绪。
- 启动 spawn/initialize/environment/account 的错误原先全部丢弃成 Provider。改用已有 provider_failure 分类和固定安全日志，不再将配置/进程/协议失败误报为账号过期。运行诊断实际捕获 provider_environment/profile/provider ambient instructions are not disabled。
- 确切根因：Agent-local codex verify_host_environment 对 model_instructions_file 二次检查直接比较字符串，和已规范化校验过的 Windows 路径不一致。提取 verify_host_configuration 并复用 instruction_path_matches（词法约束+canonical同一文件），仍核验 sealed 文件内容/禁用原生工具、MCP、个人指令及各配置守卫；新增实际启动入口的 Windows 前缀/重复分隔符正向回归和篡改拒绝。
- Service 页新增可展开近期日志和复制入口：本服务会话最多100条，UTC时间、状态、Room binding、固定错误码和重试；不存原始 provider 输出/凭据。engine ServiceHealth 记录阶段改变与每次重试，轮询不重复写入；列表摘要同一错误与重试说明去重。
- D:/BuildCache/hagency-client 及 D:/BuildCache/hagency-server 为本轮所有编译/cache/tmp/log/隔离测试PostgreSQL数据目录；metadata 均已核对。Server 75项含全部PostgreSQL回归通过（server-feedback-final-tests.log），启动配置守卫1项通过（provider-startup-guard-tests.log），engine完整219通过/5原忽略。最终客户端/原生验证随后追加。
- Server 改动在 D:/Works/hagency-org/hagency-server；本地代码验证不等于生产发布。没有改用户模型、预算、设备分配，没有发送测试消息。原生服务沿用用户已启用的服务意图。
- 最终完整GUI497通过/0失败/2原忽略（request-feedback-final-app-tests.log，23.48s），bin构建44.06s成功（request-feedback-final-build.log）。style240源文件0违规，theme guard，i18n1960中英文key一致，diff检查通过。新版可见PID20640自动LoginSuccess，myagent私聊中两条旧中文waiting提醒均显示英文中性文案，原hello/are you ok仅各显示一次且无引用块；没有改写Matrix消息。实际服务和日志页验收继续追加。
- PID20640实窗运行恢复（Running），继而处理原队列请求时发现第二处同类校验：provider_context/protocol/thread scope or policy differs。thread/start返回cwd及thread.cwd原来同样直接比较字符串，改用相同的词法+canonical路径核对；保留线程ID、模型、approvalPolicy、approvalsReviewer、readOnly与networkAccess守卫，新增逐项拒绝与Windows路径规范化回归。没有自动重跑已失败/unknown的请求。
- 补充记录运行中的Room lastError变化到服务日志，不仅记录启动失败。修正列表AgentConfigurationStatus的原生响应configuration外层解包（实窗发现Other device状态始终unknown）；代际匹配仍在解包后验证。全Agent-local回归暴露两份既有Unix绝对路径测试夹具在Windows下无效，改为各自TempDir绝对路径，未改生产账本验证规则。
- 后续验证：Agent-local完整69通过/3原忽略（request-feedback-local-verified-tests.log，1.94s），实际安装Codex 0.163.0-alpha.10.2的codex_handshake例子成功打开scoped read-only thread，不执行turn/model/tools（request-feedback-live-handshake.log）。runtime模块51通过、管理GUI48通过，针对无有效设备会话的代际隔离回归1项另通过。最终完整源构建44.39s（request-feedback-current-build.log）。
- 最终可见PID24556同一实例恢复LoginSuccess（初次Pasion元数据发现临时超时，未清凭据或修改登录配置）。实窗My Agents中myagent为Running且无错误，两个Other device显示Device unavailable · no active device session。进入myagent → Service → View Service logs，确认UTC时间正确、Starting service及Service running两条记录和Copy logs入口；当前两Rooms均Enabled。界面保留此日志面板，浅色/English，未发送测试消息、未实际复制日志或修改模型/设备/预算。实际模型生成未用新消息单独验收；原队列在中间版本中因context校验失败已终止，不能自动重跑未知/失败请求。
- 本轮Client及Server修改仍在工作区，未提交/推送或部署Server。旧提醒翻译/去引用、设备列表状态和本机运行修复已在当前Client生效；带设备状态的结构化新提醒需要Server发布后生效。
- 用户随后明确要求Client和Server各自推送远端main。提交前fetch确认两仓库与各自origin/main均无分叉；提交内容包括本轮完整代码、测试和记忆，补齐修改过的引擎文件格式，功能验证沿用上述结果。推送不表示Server生产部署完成。
