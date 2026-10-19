# Agent Start 缺少配置与错误反馈

2026-10-20 用户反馈 My Agents 的 Start 仅显示通用失败提示。

- 实际本地 agent-local.sqlite 只读查询确认 model_profiles 为零行；myagent 已由用户分配给本机 chris-pc (windows)，缺少此设备的模型配置。
- 原 Agent-wide service/start 无模型时返回 runtime_profile_unavailable，GUI user_error 未映射该码，统一显示 This action could not be completed. Refresh and retry。
- 本轮将“未保存模型”独立为 agent_model_required；service 状态返回 modelConfigured（仅表示有保存的模型配置，不承诺账号/模型可用或已经运行）。GUI 对未配置的停止服务显示明确说明和 Configure 按钮，直接打开该 Agent 的 Model & resources，不发送 Start。
- 其他启动/停止失败按 Agent 保留在卡片，状态轮询不抹掉，重试/成功/迁移设备/切换账户时清除相应错误；新增模型、资源和账号类错误的可操作说明，仅展示固定错误码对应文本。
- 全部构建/cache/tmp/log 仍 D:/BuildCache/hagency-client，metadata 已核对。新真实 NativeOwner 本地 HTTP 夹具确认缺少模型先返回明确错误，不获取 lease、不执行模型；最终验证结果见本文件后续追加。
- 不为用户自动选择模型、预算，不启动真实 Agent、不发送 Room 消息。工作区原设备名修改及语音修改保留，未提交/推送。
- 最终授权15+GUI45项通过，bin构建47.08s，PID28176自动登录；实窗卡片Configure与具体原因、等待详情/Bindings后进入模型页通过。本地Codex已关联，页面默认选择只是显示，未保存模型（最终只读仍0行）且无service intent。停myagent Model & resources，下一步用户保存模型再从Service启动。无[E]/panic，初次列表Refresh恢复的既有行为保留。见../desktop/agent-start-feedback-2026-10-20.zh.md和D盘agent-start-verification.json。
