# 模型保存按钮反馈

2026-10-20 用户反馈 Save local model profile 点击后只有顶部Tabs变色，保存按钮没有进度反馈。

- 根因：全局set_busy对Tab调用set_disabled改变配色，save_model只set_enabled没有disabled绘制/文案/进度。保存结果与读取Policy共用Target::Policy，成功后还会重新读ProviderModels，形成额外加载循环。
- 新Target::ModelSaved复用确认返回的modelProfile，避免保存后重查模型目录。专用model_saving状态在请求真正发出时设置，按钮显示Saving…、旁边LoadingSpinner、按钮禁用并有disabled样式；成功按钮恢复并旁显绿色Model profile saved，失败/前置校验旁显警示。用户修改模型/effort/workspace/web search后清除旧反馈；换Agent/账号清除状态。
- Tabs保持当前选中/未选中配色，不再用全局busy置灰。事件层已有busy守卫，保存期间不切换Tabs；模型输入及web search禁用防止在途修改。
- 管理GUI46项回归通过，覆盖Saving文案/spinner/重复提交禁用、Tabs enabled/disabled状态不变、成功/失败/反馈清除、保存不启动额外目录读取。style239/0、theme guard、双语词条一致性、diff检查通过。
- metadata已确认target=D:/BuildCache/hagency-client/target，所有构建/cache/tmp/log仍D盘。实际当前用户已保存模型gpt-6.1-sol/low/web_search=true；本轮只用原配置重复保存验收，不启动服务或发送Room消息。最终原生验证见后续追加。
- 未提交推送，原设备切换/启动反馈与语音未提交修改保留。
- 最终bin构建47.20s成功（model-save-build.log），新版PID9720自动LoginSuccess。通过实际已保存的GPT-6.1-Sol/low/web_search=true配置重复保存：实窗观察Saving…+disabled按钮，Tabs选中/未选中配色保持；确认成功后按钮恢复，旁显绿色Model profile saved，未触发额外模型列表Loading。转圈visible状态由注册GUI回归检查（实际保存很快，首帧截图主要显示Saving文字）。profile全字段摘要前后一致，runtime-intents.json仍不存在，无[E]/panic，停原Model页/浅色主题。model-save-verification.json为profileUnchanged=true、serviceIntentExists=false；日志model-save-ready.{stdout,stderr}.log。未改模型、effort、workspace或web search选项，未启动服务或发送Room消息。
