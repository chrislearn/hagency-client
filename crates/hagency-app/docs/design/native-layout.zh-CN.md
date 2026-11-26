# 原生工作区布局规范

[English](native-layout.md) | 简体中文

客户端继续使用 Makepad 原生界面，参考 OctoSense 的表面层次、排版和组件规范。
聊天、项目、Agent 的导航方式及授权流程仍由现有业务实现负责。

## 文字与颜色

默认字号使用逻辑像素。`theme.rs` 将字号转换为 Makepad 点数并应用一次用户的文字缩放。常规字重用于阅读和操作，标题、未读名称和关键数值使用强调字重。多行正文的行距系数为 1.4。

Apple 平台的中英混排使用苹方简体常规和半粗体，Windows 优先使用微软雅黑 UI，Linux 通过 Fontconfig 选择中文无衬线字体。缺字时继续查找随应用提供的字体，代码和 Emoji 使用各自的字体。

| 用途 | 主题角色 | 默认字号 |
| --- | --- | --- |
| 时间、计数、简短注释 | `RINX_META_SIZE` | 13 px |
| 导航、按钮、输入框、表单标签 | `RINX_UI_SIZE` | 15 px |
| 消息、段落、列表正文 | `RINX_BODY_SIZE` | 16 px |
| 面板、卡片标题 | `RINX_HEADING_SIZE` | 17 px |
| 分区标题 | `RINX_SECTION_SIZE` | 19 px |
| 对话框、视图标题 | `RINX_TITLE_SIZE` | 22 px |
| 工作区页面标题 | `RINX_PAGE_TITLE_SIZE` | 26 px |

- 页面、卡片和输入区分别使用 `PAGE`、`SURFACE`、`FIELD`。正文用 `INK`，辅助文字与占位符用 `MUTED`，不可操作的控件用 `DISABLED`。
- 主操作与焦点用 `ACCENT`，实心主按钮搭配 `ON_ACCENT`。选中行用 `SELECTED`，悬停与按下分别用 `HOVER`、`PRESSED`。
- 成功、待处理、错误和信息分别使用 `SUCCESS`、`WARNING`、`DANGER`、`INFO` 的前景和背景色对。状态文字或图标同时说明含义。
- 输入框和可见控件轮廓用 `CONTROL_BORDER`，面板边沿用 `BORDER`，内容间的细分隔线用 `DIVIDER`。内置主题的普通文字色对至少达到 4.5∶1，控件轮廓与相邻背景至少达到 3∶1。

以上颜色名均带有 `RINX_` 前缀，随浅色、深色和主题包一起更新。

[430 px 窄窗口的简体中文设置页，文字放大至 130%](native-layout/appearance-zh-narrow.png) · [深色 Agent 列表](native-layout/agents-zh-dark.png)

## 布局与控件

- `RinxLabel` 不自带内边距，由容器负责对齐和留白。
- `RINX_SPACE_1` 至 `RINX_SPACE_8` 提供 4、8、12、16、20、24、32 的间距阶梯，
  随当前主题的密度设置缩放。
- `RinxSection` 统一卡片背景、描边、圆角、内边距和内容间距；`RinxToolbar`
  统一操作区的换行与按钮间距。具体页面在这些组件基础上扩展。
- `RinxListRow` 使用水平 20、垂直 12 的内边距，没有描述的项目保持紧凑行高。
- `theme/layout.rs` 根据页面实际分配的宽度计算布局，而不是读取整个窗口宽度。
  宽页面基础边距为 24，小于 640 时为 16，随后应用主题边距和密度。
  小于 960 时标题与操作区上下排列；文字放大时提前换行。
- 滚动区域延伸到面板右边缘，内容、卡片与标题单独保留一致的边距，避免重复
  扣除滚动条宽度。
- 表单和概览遵循主题的阅读宽度；看板使用整个工作区，同时保留 280 的列宽及
  滚动条留白，维持拖拽几何关系。
- 狭窄区域的统计卡片纵向排列。标题和统计卡片随内容增高，避免放大文字后裁切。
- 主题重新应用时清除布局缓存。独立运行的默认配色不覆盖外部宿主主题，导入的
  主题包仍可覆盖默认角色。

侧栏、聊天颜色、登录卡片和设置页面使用相同的表面层次。登录卡片最大宽度为
480，短窗口下内容可滚动，继续使用原有登录控件。

## 原生检查

在工作区根目录执行：

```sh
cargo build --locked -p hagency-app --example workspace_preview --example project_board_preview
python3 crates/hagency-app/tools/check-native-layout.py --output /absolute/path/to/new-evidence
HAGENCY_NATIVE_CREDENTIAL_IO=deny cargo test --locked -p hagency-app --lib
python3 crates/hagency-app/tools/check_theme_literals.py
git diff --check
```

开发预览使用真实产品组件、虚构内容和独立临时数据目录，不登录、不连接后台、
不启动 Agent。检查程序保存原始截图、控件矩形、二进制哈希和原生日志，覆盖
宽／窄窗口、浅／深色、中文和文字放大，检查标题与操作区边界、内容边距、列表
位置、统计卡片、登录布局和看板列宽。仍需人工检查 PNG；几何检查不能单独证明
视觉质量、登录成功或真机体验。

## 实际截图

以下是使用虚构内容的原始 Makepad 截图，非设计稿。

![项目概览](native-layout/overview-light.png)

[深色工作区](native-layout/projects-dark.png) ·
[正式客户端登录页](native-layout/login.png) ·
[看板修改前](native-layout/before-board.png) ·
[看板修改后](native-layout/after-board.png)

macOS 检查结果：886 项库测试通过，2 项原有忽略项；18 组原生布局检查全部通过，
主题颜色检查和空白检查通过。正式 `hagency` 程序也使用独立数据目录成功显示
未登录页面并正常退出。[验证记录](native-layout/validation.json) 包含源码与
二进制哈希、实际窗口尺寸和验证范围限制。

## Projects / My Agents 工作区

两页参考当前 Concraft 客户端的项目与 Agent 工作区：默认 240 px 的上下文目录、
28 px 标题行、相邻的新建／刷新按钮，以及相同的居中选择引导。Agent 目录提供
搜索，并在桌面详情旁保留；窄屏在目录与详情间切换。搜索后的点击仍绑定原记录，
启动／停止保留设备和服务检查。待处理操作从目录底部进入独立滚动页，保留原请求。

聊天提示只在真实请求期间显示加载，失败时明确说明群聊不可用并提供重试。
45 秒 UI 等待期限不会取消仍持有原生资源的后台读取；超时后停止自动轮询。
已知私聊保持可见，未确认归属的项目房间不会混入群聊。

`legacy-*` 原生预览通过真实 Dock 恢复旧模板，再绘制页面；`chats-loading`、
`chats-error` 分别检查加载和失败。独立预览的项目目录使用示例行；正式客户端
仍使用经过账号校验的 Project tree 数据，并单独检查实际账号的导航。

最新原生截图（示例数据）：[Projects](native-layout/projects-workspace.png) ·
[My Agents](native-layout/agents-workspace.png) ·
[群聊失败与重试](native-layout/chats-group-error.png)。
本次布局通过 892 项库测试（2 项既有忽略）、61 项管理界面专项测试与全部
34 组原生检查，包含原有看板、登录和设置页面。
