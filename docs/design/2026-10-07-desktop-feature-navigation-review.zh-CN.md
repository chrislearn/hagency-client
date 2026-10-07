# Hagency Desktop 功能保留与导航复核

最新 Agent 领域与界面调整见[身份、执行实例与 Room 接入细则](2026-10-07-agent-identity-execution-instance.zh-CN.md)：管理页 Back to list/右侧 Close 已移除、标题缩小；Agent 创建不选 Project/Room；执行实例指定设备，Room 信息页可邀请自己的 Agent，默认联系人使用真实主人私聊。最新 UI 16/16、Room 信息专项 5/5、Desktop 全库 325/1 忽略通过；最终原生窗口检查等待新调试签名的可移动宗卷系统许可，不能把自动回归记为截图验收。

日期：2026-10-07。代码范围是 `/Volumes/Data/Works/chrislearn/hagency-desktop`；文档保存在同目录族的 hagency-client。本文件补充桌面工作区实施文档，记录用户指出的功能遗漏、保留边界、入口设计和实际验收。不是从空项目重建聊天客户端。

## 复核结论与范围

上一次重构把旧桌面导航栏移除，却没有完整迁移它承担的入口。联系人、Mini Apps、动态、文章编辑器、探索等仍有实现，但从桌面难以到达。Project 树替代平铺房间列表后，DM 没有明确分组，房间右键菜单也没有接上。联系人或设置又会切换整个内容页面，嵌在聊天 Dock 里的侧栏随之消失，导致不能稳定返回。

本次改造只移除 Mini Apps 中的 Palpo 管理应用与专用能力；保留通用 Mini Apps 目录、安装、导入、运行、权限与分享。普通 Matrix 聊天、SDK、资料、成员、邀请、通知等不是要删除的 Palpo 管理功能。Hagency 服务端仍通过已有 Palpo/Pasion 提供服务，本次不改其默认功能。

## 常驻导航设计

桌面分为常驻全局导航、当前栏目目录、主内容及 Room 信息四个区域。全局导航在 HomeScreen 层常驻，只显示 Projects、Agents、Chats、Contacts、Mini Apps、More 与底部账号菜单。Projects、Agents、Chats 右侧 ⋯ 分别提供 New Project、New Agent、New chat；点 ⋯ 不切换栏目。More 只收纳低频工具，不在其下混放搜索或 Project。

进入 Projects 后，在导航右侧显示独立 Project 目录；点击 Project 展开其已加入的讨论 Room，并在主内容显示 Project 详情；点击 Room 显示原聊天时间线。Projects 目录只包含服务器登记的 Project 与其已加入、非私聊 Room，不把普通 Space、未加入 Room 或 DM 混入其中。Project 名称优先使用服务器登记名称。

进入 Chats 后，同一目录区域切换为聊天搜索、Direct messages、Other rooms、Invitations，不显示 Project 文件夹。搜索仅在 Chats 内出现，保留其搜索词；进入 Projects 不受该词过滤。Agents、Contacts、Mini Apps 等页面隐藏栏目目录，让主内容使用剩余空间。从 Project 中打开 Room 不会自动切成 Chats。

选中 Room 后，右侧显示独立 280px 信息列，复用成员、资料、历史、附件、通知、邀请与成员资料原流程。它与时间线/输入栏是横向兄弟布局，不覆盖聊天。Room 可用区域不足 600px 时收起右列，允许通过标题按钮显式打开临时信息面板；关闭后不会在每次绘制时自动打开。移动导航继续使用原实现。

正常桌面全局导航 200px、目录 240px；760–959px 窗口使用 160px/180px；强制桌面模式不足760px时隐藏目录，避免内容出现负宽度。账号名称和 MXID 各一行，长值省略。创建与 More 菜单只绘制锚定小卡片，覆盖层本身不绘制全屏背景；Esc 和外点关闭。

全局导航各项左侧分别使用项目层级、机器人、聊天、联系人、应用网格和更多图标。折叠按钮位于左栏顶部 Hagency 标题右侧，避免混在内容页面工具栏或底部账号区。折叠后全局栏宽64px，只显示图标和账号头像；悬停提示栏目或账号身份。展开后恢复文字和创建菜单按钮。Project/Chats 目录不随全局栏折叠，继续可选；折叠状态按当前账号的既有 AppPreferences 持久化流程保存（退出、暂停或切换账号），不是每次点击单独写盘。

Project 目录条目右侧使用 ⋯，打开 Overview、Discussion Rooms、Project members、Invite members、Open Space 五项菜单。前三项直接打开该 Project 对应详情页签；邀请只针对绑定 Space，显示和点击均检查当前账号的 Space 邀请权限，实际发送沿用原 InviteModal/Matrix 鉴权。Space 邀请不隐式邀请或加入讨论 Room。菜单异步权限结果与邀请分发同时校验 Project ID、Space ID、服务器、MXID、SDK epoch、Service identity 和 request nonce；切换账号或目标后的回执不能打开旧邀请表单。无转让或删除入口。

| 功能 | 上版问题 | 本次入口 / 保留方式 |
| --- | --- | --- |
| Project 列表与创建 | 列表与创建杂糅、树缺资料入口 | Projects 显示列表；行右侧 ⋯ → New Project 打开独立表单；默认新建 Space，也可选择已有 Space |
| Project 讨论聊天 | 树仅展开、资料不明确 | Projects 目录点击 Project 展开讨论组并显示详情；右侧 ⋯ 提供概览、讨论组、成员、邀请、打开Space；点击 Room 打开原时间线及右侧信息列 |
| Project 成员 | 详情缺成员信息 | Project 详情的 Project members；名单来自绑定 Space，姓名、MXID、加入/邀请状态及角色 |
| Project 资料 | 仅技术字段 | Overview 展示项目及绑定关系；Open Space 打开原 Space Lobby；不以 Matrix 角色冒充 Agent 创建权 |
| Room 成员与邀请 | 没有独立入口 | Discussion Rooms 选择 Room → Room members；邀请依据该 Room 当前 Matrix 权限；不由 Space 成员推导 |
| Room 资料、成员、时间线 | 新 UI 容易替代旧能力 | Open Room / 点击树中的 Room 使用原 RoomScreen，资料、成员、个人资料等原流程继续可用 |
| 私聊 | 混在 Other chats，缺新建入口 | Chats 目录中的 Direct messages 明确独立，Chats ⋯ → New chat 直接打开既有找人搜索；Contacts 打开联系人列表/个人资料/发消息流程；不会因 Project 映射吞掉 DM |
| 联系人、群组、屏蔽 | 旧导航栏被移除 | Contacts；沿用联系人页面与个人资料消息/屏蔽/动态入口 |
| 房间操作 | 新树未转发右键动作 | 房间 ⋯ 和右键/长按复用原 RoomContextMenu：已读、收藏、低优先级、通知、搜索、附件、资料、邀请、复制链接、离开等 |
| 收藏与排序 | 树没有状态提示 | 收藏显示 ★；私聊和其他聊天保留现有排序规则 / SDK 顺序；Project 子 Room 按名称排列 |
| 邀请接受/拒绝 | 新树可能遗漏 | Invitations 单独列出，仍通过原 InviteScreen；未接受的 Room 不变成已加入讨论组 |
| Mini Apps | 原实现保留却失去入口 | 左栏 Mini Apps；保留内置 Article Editor、Hub、开发者导入、签名校验、权限确认、冻结快照与离线运行 |
| Mini Apps 分享/通用能力 | 容易误删 Matrix/网络功能 | 保留 Matrix、Octos、storage、net、clipboard、images 等通用能力与分享选择器 |
| Palpo 管理 Mini App | 与新的管理边界重叠 | 从内置 registry、Hub/本机列表移除；旧 app ID 和 palpo.* 能力在导入、安装、运行入口拒绝；不触碰其他 App |
| 动态、我的发布、文章编辑 | 旧 rail 被移除 | More → Moments / Article editor；个人资料和账号相关页保留原发布入口 |
| 文件传输 | 桌面入口遗漏 | More → File Transfer，使用原实现 |
| 探索与加入 Room/Space | 旧 AddRoom/Space rail 入口消失 | More → Discover / Explore rooms；普通 Space 保留原浏览/加入流程，不自动登记成 Project |
| 内置浏览器 | 应用工具入口不足 | More → Browser，沿用原窗口 |
| 设置、设备、账号、退出 | 曾把登录放在已登录导航 | 底部账号菜单及 More → Settings；切换/增加账号沿用统一 Pasion 身份及数据隔离 |
| Agents、本地 Codex 与策略 | 保留本轮新能力 | Agents 列表及详情；身份永久归创建者，Room/请求者配额与工具策略在本机管理 |
| 旧 Agent Operations | 不应与新的 Agent 管理并存 | 默认 feature 已关闭；新的入口统一为 Hagency Agents，不恢复旧运营菜单 |

## 成员数据与权限

成员读取只使用当前已登录 Matrix SDK，不另建 OAuth 或修改服务器 API。Project 读取其 Space，Room 读取自己；只在 SDK 当前状态为 Joined 时读取 JOIN/INVITE 名单，不隐式加入或把父 Space 关系作为权限。显示 Matrix 角色与 power level，不能据此断言管理员配置的 Agent 创建许可。

请求在开始时固定 homeserver、MXID、账号 epoch，并校验捕获的 SDK client 本身；结束后复核身份和房间状态。UI 还验证 request nonce、选定 roomId 和 Space/Room 类型。切换账号或选择其他 Project 后，旧成员响应不会填入新页面。

邀请按钮在名单未加载、权限不允许或目标改变时禁用；允许时打开原 InviteModal。真正发送邀请仍由 Matrix 服务器鉴权。Space 成员并不自动成为 Room 成员，这一规则在两个成员页面分别说明。

## 保留原实现的边界

恢复的是既有功能与入口，不把 Room 时间线、联系人、加密、媒体、搜索、资料、邀请再实现一份。聊天 Dock 的 UI 状态整理只去除被上移的固定侧栏项，保留打开的 Room、最近顺序、选中聊天及用户分栏，不迁移旧 Fleet/Agent 领域数据。

Mini Apps 的旧 Palpo adapter/仪器资源可以保留为历史源码，但默认目录和合法导入/安装/运行均没有可达路径；没有通过删除 Matrix/Octos 能力来达到移除 Palpo 的目的。独立 client 的 quota/Codex 引擎保持原边界，不增加 Agent 转让。

## 复审与验证记录

- Mini Apps：目录 18 项、内置应用 8 项、包导入 9 项通过；目录严格 Clippy 通过。1 项生产网络测试按原配置忽略，没有把网络未测写成通过。
- Project/Room UI：10 项通过，覆盖真实脚本注册、Project/Agent 详情、成员目标隔离、Space 与 Room 的独立邀请权限及互斥 section。
- Project 树：私聊与 Project 映射、搜索、邀请、登记撤销等回归；菜单真实注册和独立动作回归通过，Project 树共 7 项包含在统一 Home 测试中。
- 独立只读复审发现成员读取捕获 SDK client 的账号切换竞态，已补捕获对象的 origin/MXID 校验及发送前 epoch 检查，不仅在结果返回后丢弃。
- 实际旧窗口已显示用户创建的 testproject。测试不删除或改写这个 Project，不触发付费 Codex 执行。
- 最终统一 Home 39 项、Hagency 管理 10 项、翻译 3 项通过。真实绘制回归明确断言左侧 active View 为 280px、1280 窗口右侧 PageFlip 为 1000px、正常桌面全部 7 个主入口可见、500 高度展开 More 时账号栏在窗口内、聊天树仍有可用高度。额外断言长短账号的姓名与 MXID 子元素没有越界。
- 最终 Clippy 通过，保留 19 条既有 warning；统一 binary 构建与实际窗口检查记录见下方。
- 用户允许 macOS 外置磁盘读取后，实机检查发现绘制回归漏掉了侧栏实际宽度：AdaptiveView 的 active Desktop walk 覆盖外部 280，窗口左栏占约半宽；主导航又需要滚动，账号 MXID 第二行被裁剪。此构建不能计为布局验收通过。已增加实际 active View 和右侧 PageFlip 宽度断言，并修正外层固定宽度容器；账号子元素和菜单可见性继续复核。
- 真实窗口已确认 Project members 返回 chris 已加入及服务账号待邀请两条记录；Mini Apps 保留 Article Editor、App Hub、Import an app；联系人原页面可达。不会将这些只读检查当作完整私聊创建、邀请发送或付费 Agent 执行通过。

## Chrome 扩展安装问题

实际 Chrome AX 窗口标题含 Guest，页面提示项目不可用。Google 明确禁止在 Guest/Incognito 中安装扩展；需在普通 Chrome profile 窗口安装。当前本机 Browser 插件的 extension ID 与 OpenAI 商店条目一致，并非 Hagency 页面造成的安装错误。

官方资料：[Chrome 商店安装故障](https://support.google.com/chrome_webstore/answer/1698338?hl=en)、[OpenAI Browser extension](https://learn.chatgpt.com/docs/chrome-extension)。安装入口按 OpenAI 文档使用桌面应用 Settings → Computer Use → Install。当前任务不修改 Chrome profile、企业策略或扩展权限。

### 本轮布局增量复审

第二次只读复审发现没有聊天历史时关闭管理页仍留在失效内容上，已使 Close 与 Chats 共用返回路径；没有历史则创建或选中 Hagency 聊天空页。Project 概览、讨论 Room、成员与创建表单分别使用主题卡片，主内容与侧栏背景分开；页签默认/悬停/按下都使用对应主题前景色，支持深色主题。

新建聊天仅打开原联系人搜索，不自行发送消息或自动新建 DM。先切到 Contacts，等 TabSelected 后再进入搜索；已经在 Contacts 时直接打开。其他导航或退出清除待处理动作，进入新搜索使旧加载 nonce 失效。原聊天加密、媒体等仍使用 SDK/原页面。

最终统一 binary 构建通过（21.02 秒），源代码比 binary 更早；128 项源码指纹全部一致，diff 空白检查通过。服务器 readyz 200，正常 HTTPS 验证结果为 0。最终开发构建已启动，但其新调试签名又触发了可移动宗卷系统提示；已请求用户手动允许，当前仍不计最终窗口验收通过。

### 2026-10-07 导航精简追加

根据用户再次审阅，删除左侧 New Project / New chat 两条独立导航。创建统一收入栏目右侧 ⋯，More 也改浮层，避免原来 7 个工具只显示 3 个并挤压聊天树。

真实窗口又确认主内容颜色与侧栏重合、卡片 border_size 默认 0，前一版所谓独立 surface 并没有形成可见层次；已改主内容使用主题 field，卡片使用主题 surface 和 1px 边界。详情与表单隐藏重复的 New/Refresh 控件，成员页使用自己的刷新，隐藏 power 数字只显示角色。聊天找人搜索输入框增加边界，进入搜索后隐藏重复的 ＋。该追加的构建/菜单实机记录后续补齐。

该追加代码 Home 41 项（含新增菜单真实动作、外点/Esc关闭、More全部工具在500高内可见）、管理UI10项、翻译3项通过。最终Clippy保留19条既有warning，新增菜单的6条redundant guard已修。源码指纹清单更新为130项；最终binary构建通过（19.34秒），全部130项指纹一致，diffcheck通过。该菜单构建已启动；新调试签名再次等待系统外置磁盘许可，已请求用户手动允许，菜单构建的真实窗口尚未计为验收通过。此前已实机确认280px侧栏、账号姓名/MXID两行、Project列表与成员页、NewChat真实搜索页可达。

### 2026-10-07 栏目目录与 Room 信息列续修

用户截图中的菜单覆盖错误已在旧原生窗口真实重现：点 Projects ⋯ 后整窗变灰，只剩菜单项。原因是覆盖层使用 SolidView，透明色写法没有消除实际背景绘制。现在覆盖层使用普通 View 且明确 show_bg=false；卡片独立绘制，235px 宽、Fit 高、锚定栏目右侧。增加检查实际 draw-command 实例的回归，确认菜单卡片外没有全屏覆盖绘制，底层绘制指令仍存在。单纯检查组件几何不能替代这一检查。

此轮改成全局导航与栏目目录分列，Search chats 仅在 Chats，Project 树仅在 Projects。树和 Matrix RoomsList 元数据消费者各只有一个实例，切换栏目不另造 SDK 同步消费者。Project Room focus 保持 Projects workspace；各账号切换清理搜索与目录数据。Chats 搜索词独立保存，Projects 不受搜索过滤。

Room 信息从覆盖聊天的面板改为真实横向兄弟列，timeline/input 的实际尺寸断言覆盖 840px、600px、500px 的 Room 区域。窄窗首次选 Room 不自动显示临时面板，只在明确点击信息按钮后打开；切换 Room 重置目标，关闭后正常绘制不会又打开。只为当前显示的信息实例读取数据，隐藏实例不重复请求。成员异步响应增加 origin、SDK epoch、精确 Room ID、request 和 payload Room ID 的匹配，防止切换后旧名单污染新页。

统一 Home 48 项、管理 UI 10 项、翻译 3 项通过；包括实际脚本注册、绘制指令、导航动作、响应式分栏与首次窄窗行为。普通 Clippy 通过，保留19条既有警告；严格 -D clippy::all 仍因既有 build.rs 的 collapsible_if 失败，不宣称零警告。binary 构建通过（45.52秒，包含等待 Cargo 锁）；所有生产源码早于 binary，开发 launcher 记录的构建输入与最终 Cargo binary 一致。签名后 bundle 不以字节一致性与 Cargo binary 比较。源码指纹130项已更新，Desktop diffcheck通过。第二位代理完成六个增量文件只读复审，未发现新增可执行缺陷。

新版开发应用已启动，当前停在外置磁盘资源文件读取并等待系统许可；已请求用户手动处理。上述自动检查不计作新版菜单、分栏和 Room 成员的最终原生窗口验收，实机结果待许可完成补齐。服务器 readyz 200，正常 HTTPS 证书验证为0。测试没有删除/改写用户的 testproject，没有发送消息或执行付费 Codex 模型。

### 2026-10-07 图标、折叠与 Project 操作菜单

按最新截图补齐全局图标、顶部折叠按钮、64px图标栏和紧凑账号菜单，Project 目录中的 ⓘ 改为 ⋯ 操作菜单。全局与 Project 的更多按钮改为无浮雕的平面按钮；菜单只绘制自己的卡片。

Home 51项、管理UI11项、翻译3项通过。新增真实绘制/动作检查覆盖多次展开折叠、图标与头像边界、目录位置、单一RoomsList消费者与ProjectTree实例、偏好序列化；Project菜单实际注册5项，陈旧权限/邀请结果不能导航或打开邀请；详情页签等待精确Project的Room结果后跳转。另一个代理完成偏好、Home、左栏、账号与ProjectSection增量只读复审，无新增实质缺陷。Clippy通过并保留19条既有警告，最终binary构建通过（22.42秒），源码指纹更新为131项。

用户新截图已确认前次构建的全局导航与Project目录实际分列。图标/折叠最终构建启动初期桌面工具读取超时，已提示可能的系统外置磁盘许可；随后进程完成启动且原生窗口可读，最终验收已继续，不再处于等待状态。

真实窗口已确认展开栏的六个图标、顶部折叠/展开按钮、64px折叠栏、独立240px项目目录、账号头像和账号菜单。展开恢复后文字与创建按钮正常出现。Project ⋯ 菜单实际显示五项，锚定在条目下方，底层导航/内容完整保留；Project members直接跳到testproject成员页，看到chris已加入和服务账号待邀请两条记录；Invite members经过权限复核打开“Invite to testproject”原表单，未输入收件人，已取消。没有实际发送邀请或消息，没有修改用户的testproject。菜单及折叠截图保存在Desktop的`.run/hagency-local-https-20261007/desktop-project-menu-icons.png`与`desktop-icons-collapsed.png`。
