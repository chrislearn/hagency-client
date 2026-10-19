# 执行设备定位与切换入口

现有入口：Agents → View details → Agent，勾选切换说明，再点击切换按钮。
切换操作只将所选 Agent 分配给当前设备，沿用服务端 generation 检查与旧 lease 失效处理。
本轮不实际切换用户的 Agent。

本轮改进：

- 原按钮 Handle on this device 改为 Switch execution to this device（将执行切换到此设备），并显示目标设备名称。
- SDK 客户端注册设备时使用系统 hostname 和 OS，例如 CHRIS-PC (windows)。安装身份、Agent 配置与账本路径不受名称影响。
- 列表和详情的设备标签同时支持本机、其他设备和不可用设备的名称。历史通用名称 Hagency/Hagency Desktop、同名设备补充编号尾部八位；详情另显示完整名称和完整设备编号。
- 名称过长时列表标签截断，详情保留完整信息。注册名称过滤控制字符并限制长度，符合服务端 128 字符上限。
- 旧的离线设备不能凭空补出电脑名：需要原设备运行新版后重新登记；这之前可通过设备编号区分。

验证记录：所有 Cargo 产物、下载缓存、临时文件和日志使用 D:/BuildCache/hagency-client；metadata 已确认 target_directory。设备名称/注册授权回归 14 项通过，GUI Hagency 管理界面 44 项回归通过（涵盖切换确认、设备显示、旧设备编号区分、切换后清空旧资源状态）；样式审计 239 文件零违规、theme guard、词条一致性与修改文件格式检查通过。全包 cargo fmt --check 另发现既有语音/其他源码格式差异，本轮未改写这些文件。

原生 bin 构建成功 47.66s（device-name-build.log），正常关闭原 PID53624 前确认 hello 输入为空。新版 PID46732 自动 LoginSuccess；初次列表加载超时，点击 Refresh 后恢复。实窗列表显示旧设备 Hagency Desktop · f1v3506e，详情显示完整名称、编号及切换按钮，目标明确为 chris-pc (windows)；profile 注册名也已更新。无 Makepad [E]/panic，保留浅色主题，窗口停 myagent 详情，确认框未勾选。原有远端环境元数据仍提示 Execution device assignment changed，本轮没有扩大范围修复该问题。只验证显示，不提交真实切换。证据 device-name-{registration-tests,ui-tests,ready.stdout,ready.stderr}.log 与 device-name-verification.json。

没有修改 Server/schema.rs、发布、迁移设备、启停 Agent 服务或发送 Room 消息。
