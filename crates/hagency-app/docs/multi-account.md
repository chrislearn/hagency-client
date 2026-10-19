# 多账号登录与切换

本文说明当前 Hagency 的账号流程。普通 Matrix 服务器不受支持；登录、保存会话恢复和重新授权都会核验服务器的 Hagency discovery 协议。Matrix 聊天、设备验证和加密数据库仍由 Matrix SDK 管理。

## 入口与登录

- 桌面左下角账号菜单展示当前账号，提供账号设置和退出登录。已保存账号列表与添加账号入口位于设置的账号页。
- 登录页也展示已保存账号；恢复失败或取消添加后，可选择原账号。
- 窄屏使用设置的账号页共享同一个账号列表，不另设密码登录流程。

可添加同一 Hagency 服务器的不同账号，也可添加不同 Hagency 服务器的账号。一次只激活一个账号；未激活账号不会在后台同步，也不汇总其未读数或推送。

添加账号先停止当前账号活动，再输入 Hagency 地址并在浏览器通过该服务器的 Pasion 登录和授权。这一次 OAuth 同时建立 Matrix SDK 会话和 Hagency 主人授权，不要求再输入 Matrix 密码或进行第二次主人登录。授权失败不会删除原账号会话；新授权候选保存失败时只清理该候选。

SDK 唯一负责 OAuth access/refresh token 的刷新与持久化。原生 Agent 管理使用 SDK 提供的当前证明；不会另行刷新、复制或撤销 SDK OAuth token。主人授权还校验服务器、issuer、subject 和完整 Matrix ID，不能把同名用户或不同服务器的身份混用。

## 切换、退出与重新授权

切换到已保存账号时，先停止当前主人 Agent runtime、Codex 管理子进程和授权，再关闭当前 SDK 账号任务作用域，清理账号相关界面、订阅和缓存，等待界面确认清理，最后恢复目标会话及其界面状态。账号 epoch 会拒绝旧账号的异步结果。切换不调用 Matrix logout，不删除原账号的加密数据库，也不转移 Agent 所有权。

退出登录使用确认框和服务器撤销流程。停止本机主人运行后，撤销当前 SDK 登录并移除当前账号的会话文件；其他账号可继续选择。当前账号的加密数据库与不含 token 的恢复索引保留。网络失败会显示错误，不把未完成退出显示为成功。

保存会话失效时需要重新通过 Pasion 授权。当前实现拒绝恢复旧的非 OAuth 登录会话，不提供密码或普通 Matrix SSO 回退。重新授权可能得到新的 Matrix 设备；不能承诺设备 ID 不变。旧数据库保留，历史加密消息的恢复仍取决于密钥备份和设备验证。Hagency 的稳定授权设备与 Matrix 加密设备是不同概念；重新授权不能绕过 Agent 的指定执行设备或账本连续性检查。

账号恢复后，Projects 详情使用 Overview / Rooms / Members / Access，Agent 详情使用 Agent / Codex / Budgets / Run；这些管理入口仍受当前账号授权与本机资源隔离。

“使用本机已登录 Codex 账号”不会把该登录变成 Matrix 身份。账号切换和断开关联不等于日常 Codex logout；主人账本、模型配置及 workspace 保持各自身份隔离。

## 重启与运行恢复

运行恢复已验证普通及快速重启：只有当前完整主人身份、指定设备下曾明确 Start 且没有显式 Stop 的 Room 可以恢复；切换不会在后台恢复未激活账号。Stop Room 持久排除该范围，正常关闭程序可保留合法运行意图，但所有授权、资源、账本和租约都须重新校验。详见 [运行恢复记录](../../hagency-client/docs/design/2026-10-08-agent-runtime-recovery.zh-CN.md)。

## 本地数据边界

| 数据 | 隔离方式 |
| --- | --- |
| Matrix 账号目录 | 完整 MXID 的 BLAKE3 摘要：`account_<digest>`，包含服务器名，避免路径字符与替换碰撞 |
| OAuth 会话、SDK 数据库路径和数据库密码 | 账号目录内 `persistent_state/session`；原子保存，Unix 文件 0600、目录 0700；账号列表只展示用户 ID 和服务器地址 |
| Matrix 消息、缓存和加密密钥 | 登录设备自己的 SDK SQLite 数据库；有效会话恢复使用原数据库；旧数据库保留在恢复索引中 |
| 聊天选择及界面状态 | 账号的 `persistent_state/latest_app_state.json` |
| Hagency 账本、模型配置、运行和 workspace | 服务器、issuer、subject、MXID 的完整主人身份隔离；缺失旧执行费用证明时拒绝新模型执行，不从零余额恢复 |
| 小应用状态、权限、阅读器及账号异步工作 | 保留账号作用域；切换撤销旧权限并拒绝迟到结果 |
| 分享临时文件 | `cache/temp/<account digest>/…` |
| 文件选择器结果 | 捕获账号 epoch；切换后的旧结果丢弃 |
| 语言、窗口位置、主题、应用运行配置 | 应用级设置，多个账号共享 |

目录隔离防止应用内账号串数据，不是同一操作系统用户下的密码学保密边界。私有 SDK 会话文件包含 token 和数据库密码，目前不能声称它们全部存放系统钥匙串。不要公开会话文件，也不要将 token 写入截图、账号索引或报告。

## 目录整理与恢复边界

现有扫描器会核对会话中的真实完整用户 ID；可把旧 Desktop Matrix 目录名整理为摘要目录。已有目标目录时不覆盖，读取错误会报告并停止相关数据库清理。这个目录整理不赋予登录权限：恢复仍要求 Hagency discovery 和合法 OAuth 会话。它不是旧 Fleet 数据兼容或导入流程。

数据库清理检查全部保存账号及不含 token 的 `retained_db.json` 恢复索引。读取失败或损坏时停止清理，不能因为账号当前未激活就删除其数据库。

默认构建不启用实验性 TSP 钱包。启用该 feature 的构建仍拒绝账号切换，避免进程级钱包状态串账号；不要把默认账号流程的验证结果套用于钱包功能。

## 核对依据与验收状态

本页依据 `accounts.rs`、`account_session.rs`、`sliding_sync.rs`、`persistence/matrix_state.rs`、账号设置和退出状态机核对。旧密码 loopback fixture 的历史通过记录不证明当前 Pasion OAuth 多账号流程通过，故不沿用旧统计。

本轮逐页截图、不同服务器账号切换、过期授权及窄屏可用性结果统一记录在 [Desktop 界面复核报告](../../hagency-client/docs/design/2026-10-08-desktop-usability-review.zh-CN.md)。未截图状态不会标为通过。当前使用说明见 [快速开始](hagency-quickstart.zh-CN.md)。
