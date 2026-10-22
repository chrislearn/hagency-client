# 联系人中的 Agent 身份信息

更新：2026-10-22。

- 用户实窗确认联系人分栏，并要求先提交推送 main，再丰富联系人详情：显示是否为 Agent、所属用户、自己的 Agent 明确标识以及 private/public。已将分栏源码与其记忆单独提交为 `7330d9fa Show contact details beside the contacts list` 并推送 origin/main；未将其他任务已有未提交改动纳入该提交。
- 新增 `src/profile/agent_details.rs::AgentProfileDetails`，在联系人卡片与操作区之间显示账号类型（My Agent / Agent）、所属用户的精确 Matrix ID、可见性（Private / Public；旧服务未返回则 Unknown），包含中英文文案。以现有 `Command::Agents` 当前所有者列表优先匹配目标 `puppetMxid`；若不属于自己，再通过 `Command::DiscoverAgents` 查询公开 Agent。
- 不根据用户 ID 前缀/显示名认定 Agent，不将其他用户的私有 Agent 暴露给当前用户；公共目录最多返回 256 项，满页且未匹配时显示查询失败，不能据此断言对方不是 Agent。没有新增服务端接口或修改 schema.rs。
- 返回数据严格核对列表所有者、目标 Matrix ID、public 可见性、isMine 与实际归属一致性及重复匹配；跨账号/服务地址/会话 epoch/联系人/请求代次的旧响应不会被应用。新联系人先清理上次详情，退出账号/取消选择清空状态。超时仅结束 UI 等待，后台只读任务继续完成既有授权记账，不主动启动 Agent 或发送消息。
- 检查中有独立加载提示；失败显示通用错误和 Retry，不显示原始服务错误。正常用户未在可见列表匹配到时不添加 Agent 标识，也不声称其为人类账号。
- 初步验证：新增身份解析/精确匹配/恶意和缺失字段/重复身份/满目录/320与790px布局/清理选择共 3 项测试通过；同一编译测试程序 `home::` 共 93 项通过。主题字面量守卫、翻译 2226 项/0缺失、原生样式 263 文件/0违规及 diff 检查通过。编译由 tools/dev.ps1 的 metadata 核对 D 盘 target，Cargo home/tmp 同在 D 盘。
- 最终验证：应用库严格 Clippy（`--no-deps --lib -- -D warnings`）通过；正式 bin 构建 1m01s 成功。使用 computer-use 的正常 Alt+F4 路径关闭旧 PID73060，日志确认窗口与应用状态保存后结束其托盘驻留，再沿用原 `HAGENCY_APP_DATA_DIR=D:/BuildCache/ui-review-data` 启动新版 PID33980，自动登录成功、stderr 为空。实际窗口进入 Contacts 点击 abcd，确认完整显示 Account type=My Agent、Agent owner=@chris:localhost:8088、Visibility=Private；停在该详情供用户查看。没有启动服务、创建 Agent、改变可见性或发送消息；其他用户公开 Agent 的分支由自动化测试覆盖，未用真实第二账号验收。
- 日志使用 `D:/BuildCache/hagency-client/contact-agent-details-*`（tests、home-tests、clippy、build、app.stdout/app.stderr）。本次新增身份信息暂未提交推送；origin/main 已核对为前述分栏提交 `7330d9fa`。

用户随后要求 Agent 详情去除 Moments：`AgentProfileDetails` 提供 `allows_moments`，仅正常完成查询且未匹配 Agent 时允许显示；已确认 Agent、查询中及失败时隐藏。联系人页同步控制行的可见性和点击处理，切换联系人或退出账号时清空该状态，保留复制链接/消息/屏蔽行。既有 3 项身份与布局回归补充上述 Agent/普通用户/清理/失败状态断言并通过，日志为 `D:/BuildCache/hagency-client/agent-hide-moments-tests.log`。

该增量 bin 构建52.34s通过；旧 PID33980 经正常 Alt+F4 关闭并确认保存状态后结束托盘驻留，原资料目录启动新版 PID584，自动登录成功、stderr为空。computer-use 实窗确认 abcd 的 My Agent/所属账号/Private 信息完整保留，Moments 行消失，复制链接行紧接身份区，无残留占位。日志为 `agent-hide-moments-build.log` 与 `agent-hide-moments-app.{stdout,stderr}.log`。未提交/推送。

用户继续要求联系人列表明确区分 Agent，并修复 Contacts 标题和 + 按钮错位。本轮在名称右侧加 20px 高的 Agent 文字标签，使用共享主题的 accent soft/foreground，不遮挡头像。联系人加载时批量读取当前所有者和公开 Agent 列表，复用详情的服务端字段验证，普通账号不显示标签；按 Matrix ID 设置每次绘制的可见性，避免 PortalList 行回收串标识。结果按账号/服务地址/epoch/请求代次围栏处理，账号清理和新好友模式清空标识。标题改为占满标题栏的居中容器加文字自适应高度；+ 图标与返回按钮都显式居中，避免 Fill 高度文字和按钮的不同居中行为。日志前缀 `D:/BuildCache/hagency-client/contact-badges-*`。

该增量自动化验证：home:: 93 项与身份/标签字段验证 3 项，共 96 项通过；其中实际绘制断言检查标题文字高度、与 + 按钮垂直中心差 <1px、Agent/普通联系人标签和搜索后行复用清理。应用库严格 Clippy 通过；主题守卫、翻译2226项/0缺失、样式263文件/0违规、diff检查通过。共享 MobileTitle 保留原 title 直接子项接口，窄屏 Chats 使用方兼容。

该增量正式 bin 构建2m30s通过。旧 PID584 经正常关闭并确认保存后结束托盘驻留，原 D 盘资料目录启动新版 PID37432、自动登录成功，stderr为空。实窗进入 Contacts，abc/abcd/wewee 三行均显示 Agent 标签，Contacts 与 + 图标处于同一水平中心线；点击 abcd 仍在右侧打开详情，My Agent/所属用户/Private 保留且无 Moments。仅导航查看，无消息发送、可见性修改或服务启动。未提交/推送。

用户确认上述几次联系人和 Agent 信息改动效果可以，并要求提交推送远端 main。本次提交包含联系人页、Agent 身份组件及注册、本功能新增的7条中英文文案和本记忆；其他 Agent 创建流程/错误反馈任务的源码和翻译改动保留在工作区，不纳入此次提交。提交前 fetch 核对 main 与 origin/main 无分叉，沿用上述已通过验证。
