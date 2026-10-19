# 管理页统一导航边界与审批房间

更新：2026-10-20。

- 用户要求整体检查页面样式、菜单消失与审批房间的用途，不能按截图逐个打补丁。检查记录：../workspace-pages-audit-2026-10-20.md。
- 新增 shared/workspace_pages.rs，把 10 个原本位于 App 全窗口层的管理/内容页面集中放入 WorkspacePages。按 HomeScreen 实际 PageFlip 或设置 detail 区域设置 ModalBounds，保留主菜单、上下文侧栏和设置分类；与嵌入宿主边界相交并在绘制后还原。确认/验证/授权/上传等真正对话框继续使用全应用范围。
- 主菜单、设置分类和其他管理入口统一调用各页面原有 Close，保留取消扫描、请求代际清理及主题预览回滚；不关闭桌面独立浏览器/小程序窗口。App 注销时关闭页面并清空布局边界。先前外部机器人子页修复仍在同一工作区。
- 宽窄切换回归发现 AdaptiveView 可在绘制中替换子树，新的 PageFlip 会暂时回到默认页。HomeScreen 在绘制后同步新实例的所选页并触发下一帧；管理页在边界尚未完成时不绘制，禁止退回全窗口。回归包含真实 HomeScreen 的宽→窄→宽及 Home/Settings 切换。
- 审批房间是真实的私有审批房间发现入口，列房间而非待审批请求数。新增用途说明、刷新禁用/检查中文字、无房间解释卡片，区分全量无结果、项目无结果和部分检查失败；新增状态均有中英文，重应用时更新。认证标记、账号、joined-room、namespace、超时、扫描上限、代际/取消守卫保持既有机制。
- 全部产物、cache、tmp、日志使用 D:/BuildCache/hagency-client，metadata 核实实际 target。新增共享容器/10页宽窄边界和设置切换回归、真实 Home 布局回归、审批空/部分失败/加载及取消回归。最终全量与实窗结果随后追加。
- 最终两个布局回归通过（workspace-pages-responsive-tests.log）。运行该次编译得到的完整测试程序（工作目录 crates/hagency-app，D 盘环境）501 通过、2 项原有忽略、0 失败（workspace-pages-verified-tests.log，25.57s）。仅新 shared 文件做 rustfmt，最终 bin 从保存源码构建成功（workspace-pages-build.log，1m08s）；style 241 文件 0 违规、theme guard、1990 个中英文 key 一致、diff 检查通过。
- 第一轮实窗确认审批页已保留两级菜单，扫描为 0/5/0 并显示解释卡片，主菜单 Agents 可退出该页；同时发现长说明单行裁切及设置分类输入被遮罩拦截。补上正文/说明换行、容器外新按下/滚动输入交还原页面、管理容器允许应用内导航滚动（真实对话框后绘制仍设置自己的范围）。增加窄页长文案多行、各页面滚动例外区域和菜单新按下不被抢占回归。501 项完整再次通过（workspace-pages-complete-tests.log），补充输入守卫后两个通用布局回归再次通过（workspace-pages-input-tests.log）。最终源码全量/构建及实窗结果随后追加。
- 最终完整测试程序 501 通过、2 项原有忽略、0 失败（workspace-pages-last-tests.log，24.62s），最终 bin 构建成功（workspace-pages-final-build.log，1m04s）。实窗 PID 68540 自动恢复会话：审批页的主/设置菜单保留，点击设置 Interface 可直接退出审批页；外观编辑保持两级菜单，点击 General 正常退出；More → Explore rooms → Create a room or space 的管理表单保留主菜单，点击 Settings 正常退出。没有实际创建房间、修改主题或发送审批/聊天指令。
- 中英文实窗均确认审批标题、用途、扫描中按钮/状态、0/5/0 结果及空状态说明；英文长说明已完整换行。语言验收后恢复原 English 偏好，保留新版审批页。布局与输入回归覆盖 10 个共享宿主页面的宽窄边界、导航按下不被抢占、滚动允许范围与菜单关闭清理；房间历史无认证 owner 时保持不绘制守卫。没有服务器或 schema.rs 变更。

- 用户随后明确要求将全部修改推送远端 main。本轮客户端代码、测试、双语资源及检查记录一并提交推送至 origin/main；提交前 fetch 确认本地与 origin/main 均为 50fa85d6，无分叉。推送结果以 Git 回执及远端引用核对为准。
