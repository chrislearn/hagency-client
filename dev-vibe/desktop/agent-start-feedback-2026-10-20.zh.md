# Agent Start 缺少模型时的反馈

用户把 myagent 分配给本机后点击列表 Start，界面只显示通用“刷新重试”。只读检查本机 model_profiles 为零行，实际缺少此设备上的 Agent 模型配置。

已修正：

- 无模型的启动拒绝使用明确 agent_model_required，独立于已保存配置不可用的 runtime_profile_unavailable。
- 服务状态新增 modelConfigured，仅表示本机已保存此 Agent 的模型配置，不代表账号、模型或服务已经就绪。
- 未保存模型时，列表显示原因和下一步，操作按钮为带设置图标的 Configure；点击进入对应 Agent 的 Model & resources，不发送启动命令。
- 启动或停止失败按 Agent 保留在卡片中；状态轮询保留错误，重试、成功、重新分配设备和换账号时清除。常见模型、账号、连接、lease 和已运行等失败显示固定、可操作的说明。
- 现有已运行服务仍显示 Stop；旧客户端/旧状态中缺失 modelConfigured 字段时不擅自判为缺少配置。

验证使用 D:/BuildCache/hagency-client，metadata 已确认 target。NativeOwner 本地 HTTP 夹具已验证无模型拒绝、不获取 lease/不执行模型、保存模型后状态变为已配置且不生成 runtime-intents.json；授权模块15项、GUI管理模块45项回归均通过。GUI回归包含真实卡片操作进入对应Agent模型页且不提交Start。样式239文件零违规、theme guard、双语词条一致性及改动文件diff/engine格式检查通过。

首轮编译修正Makepad派生宏不支持字段直接使用全限定HashMap泛型（改为类型别名）。正向状态回归还捕获了误用intent路径读取账本的问题，已使用ledger_path解析真实账本并加“状态读取不创建intent”断言；错误版本只运行在隔离临时夹具，未构建/运行用户GUI。

实窗首次检查发现立即切换Model页会在Agent详情异步加载时被旧导航重置到setup；新增按Agent的wanted_model，等Bindings/执行设备确认后进入Model页，跨Agent/返回/账号变化清除，避免旧Run意图覆盖。回归扩展至AgentDetails→Bindings→sync_navigation后仍停模型页，修正后GUI45项再次通过。

最终bin构建47.08s成功，新可见PID28176自动LoginSuccess。实窗确认卡片具体原因、带设置图标Configure及异步加载后自动选中myagent的Model & resources。本地Codex已关联，UI默认模型/effort仅显示，未点Save。最终只读model_profiles仍零行且无runtime-intents.json；保持浅色主题/hello空草稿，停模型配置页，未启动Agent或发送消息。无Makepad [E]/panic。初始列表提示未加载，Refresh后恢复，是本轮前已有启动加载行为。日志agent-start-{authorization-tests,ui-tests,build,final.stdout,final.stderr}.log及agent-start-verification.json。

未替用户选择模型或预算，未启动真实 Agent、发送消息或迁移设备。前一轮设备名称和原语音修改均保留，未提交或推送。
