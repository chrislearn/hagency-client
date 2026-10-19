# 外部机器人设置与导航

更新：2026-10-20。

- 用户要求将 Registered agents 明确为“已关联的外部机器人”，区分主菜单 Hagency Agents，补齐中英文，并修复该页遮住主菜单和设置菜单的问题。
- 原因是 app.rs 的全窗口 agent_access_modal。AgentAccessPanel 现放入 SettingsScreen 的 PageFlip，作为“智能体服务”的子页；全局弹层和对应返回处理已移除。返回、Escape 和系统返回先回到智能体服务，设置关闭按钮关闭设置；切换设置分类或主菜单会关闭外部机器人子页。
- 注册顺序改为 settings 的公共样式之后、SettingsScreen 之前注册 AgentAccessPanel，避免引用未注册的控件。账户归属和异步请求 nonce 守卫、关联及 AppService 管理行为保持原有机制。
- 表单使用“已关联的外部机器人 / Linked external bots”，补充 Matrix ID、类型、外部 AppService 用途与主菜单 Agents 的模型、执行设备、启停说明；关联、解除关联、状态和确认文案都有中英文。
- 增加 SettingsScreen 回归：宽窄窗口保留设置导航并限制面板边界、返回留在设置、脚本重应用保留输入、离开设置关闭子页。设置布局 5 项通过；完整 app lib 498 通过、2 项原有忽略、0 失败（external-bots-app-tests.log，23.40s），bin 构建成功（external-bots-build.log，53.10s）。style 240 文件 0 违规、theme guard、1981 个中英文 key 一致、git diff --check 均通过。
- 编译、缓存、临时文件和日志均使用 D:/BuildCache/hagency-client，cargo metadata 已核对实际 target；没有 Docker 构建、schema.rs 修改、服务器变更或发送聊天/机器人管理指令。
- 第一轮实窗确认主菜单与设置菜单同时可见、返回停留在智能体服务，中文的外部机器人标题、说明、标签及操作显示正确；发现类型下拉列表在语言切换后仍保留英文。补充在打开和脚本重应用时更新固定类型标签，动态机器人/房间选项由页面控制器翻译，避免通用 i18n 刷新覆盖实际选项。修复后设置布局 5 项再次通过（external-bots-localization-tests.log），最终构建成功（external-bots-final-build.log，52.80s）。
- 最终实窗 PID 60460：首次启动遇到服务器连接失败，正常关闭重启后自动恢复原会话，未操作登录认证或修改凭据。已确认中文“已关联的外部机器人”“通用 Matrix 机器人”和全部主体说明显示正确，两级菜单始终可见；恢复原 English 偏好并保留外部机器人设置页供用户查看。用户随后授权将本轮与管理页整体修复一并提交并推送至 origin/main，见 workspace-pages.md。
