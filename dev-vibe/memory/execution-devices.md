# 执行设备定位

2026-10-20 用户要求确认设备切换入口并显示可定位的设备名。

- 现有入口 Agents → View details → Agent → 勾选切换说明 → Handle on this device。本轮改为更明确的 Switch execution to this device，并显示切换目标名称，保留原确认和 generation 守卫。
- SDK 设备注册 `console/server_login/shared_matrix.rs` 原先 name 固定 Hagency；改用 gethostname 1.1.0 + OS，保留原 installation_id。列表/详情显示名称；旧通用名或同名设备附编号尾八位，详情显示完整名称与完整编号。
- 远端旧设备只有应用通用名时无法推断真实 hostname，需原客户端运行新版重新登记。在此之前使用编号定位。
- 设备注册14与GUI管理44回归通过，bin构建47.66s成功；样式/主题/词条及修改文件格式检查通过。原生PID46732自动登录，列表初次加载超时后Refresh恢复；实窗确认旧设备带编号、完整详情及本机目标chris-pc (windows)，停myagent详情且确认框未勾选。原有远端环境元数据仍提示assignment changed，本轮未扩大范围修复。
- 未切换 myagent 的执行设备、未修改服务配置或账本。全部缓存/日志D:/BuildCache/hagency-client；未提交推送。详见 ../desktop/execution-devices-2026-10-20.zh.md。
- 注意本轮开始时工作区已有语音相关修改及两份 i18n 修改，其他任务仍在维护这些内容；不得覆盖、提交或归入本轮结果。
