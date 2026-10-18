# 图文工作室 v2

工作分支：`wechat-ui`。本次没有合并 `main`。应用使用原生 Makepad 控件、Markdown 文档交换和 Html 阅读器，没有 WebView。

## 已交付的六项功能

| 功能 | 当前实现 |
| --- | --- |
| 所见即所得 | 原位编辑段落与标题，选择文字加粗、斜体、HTTPS 链接，标题/引用/列表，块移动与删除，撤销/重做；Markdown 为独立源码模式 |
| 插图 | 系统 JPEG/PNG 文件选择、本地素材库、正文图片、图注、替代文字、三档宽度；导入时校验、缩小、移除元数据 |
| 主题 | 经典绿、暖纸色、海盐蓝、极简墨色；真实样例、字号和间距选择；主题随文章传递 |
| 封面 | 独立选图、横向/纵向焦点、宽图和方图预览、摘要、正文头图开关 |
| 全文审阅 | 完整可滚动文章、图片和封面、字数/阅读时间；发布前内容和素材检查、按名称选择会话、最终确认 |
| 发布管理 | 持久草稿/已发布/已撤回列表；Matrix 发布、引用原事件的修订、撤回原文及修订、本地草稿保留；待确认任务可恢复重试 |

桌面宽度至少 960 时显示文章目录、有限宽度纸面和样式/封面侧栏；窄屏采用单画布与属性页面。界面支持中文和英文，沿用 Apple 平台的 PingFang 字体配置。

## 如何打开

- 移动布局：**发现 → 文章编辑器 → 查看应用信息 → 授权使用 → 新建文章**。
- 桌面布局：打开会话，点击输入区域的 **＋ → 文章编辑器**。
- 分享小应用：文章库的“分享小应用”→ 选择会话 → 确认。接收者重新授权，只能使用自己的资料与草稿。
- 分享文章：编辑 → 全文预览 → 发布检查 → 选择会话 → 确认发布。接收者点击文章卡片进入原生阅读器。
- 修改/撤回：文章库的“已发布”→ 对应文章 → 修改或撤回，再次确认。

开发运行：

```sh
cargo run --offline --features agent_chat --bin robrix
```

发布目标是当前账号加入且允许发言的 Matrix 会话。此小应用不接入微信公众平台的发布接口，也不把登录令牌交给小应用或其他用户。

## UX 来源与原生映射

- [完整产品规划](PLAN.zh-CN.md)、[编辑器研究与来源](RESEARCH.md)。规划保留了后续增强，不能将整张验收矩阵视为已通过。
- [12 场景移动 UX 原图](../../output/imagegen/article-editor-v2/mobile-ux-atlas-v2.png)、[桌面 UX 原图](../../output/imagegen/article-editor-v2/desktop-editor-v2.png)。两次请求显式使用 **gpt-image-2**。
- [生图命令](GENERATE.md)、[参数与原始哈希](generation-request.json)、[移动提示词](atlas-prompt.txt)、[桌面提示词](desktop-prompt.txt)。CLI 未返回模型解析后的快照 ID，记录的是实际请求别名。
- 12 个场景均从实际生成像素测量裁切，上游 `image-to-appcard-flow` 的 `intake,prepare` 通过；[运行回执](pipeline-output/runs/20260921T091705Z-fed7d5cc/run.json)。
- [原生映射说明](NATIVE-MAPPING.md)及各场景 `robrix-mapping.json` 记录原生控件、状态和动作。通用流水线后续编译/WASM 阶段未运行；本文档不声称其自动生成了富文本编辑器或 Matrix 后端。

## 数据和权限

结构化文档是唯一正文数据，格式范围使用 UTF-8 边界，原生字形、光标和选区共用排版。源码模式仅接受可无损表达的基础 Markdown；HTML、代码与嵌套列表被拒绝且保留用户源码。

草稿、素材、发表记录和待确认操作按当前 Matrix 账号隔离，原子保存。每次打开重新授予限时权限，异步操作后重验账号和授权。只有原生宿主可上传、下载和使用 Matrix 凭据。共享的小应用描述不含草稿、访问令牌或授权。

加密会话同时使用加密事件与加密媒体。图片按字节、像素、尺寸和内容哈希验证，阅读器不会加载文章任意指定的外网图片。其他 Matrix 客户端可读标准文本/HTML 回退；加密插图的扩展显示需支持本文章格式。

更新使用 `m.replace` 保留原文章身份。Palpo 测试实例的关系索引没有返回已存在的修订，因此宿主额外通过 SDK 解密后的历史回溯核验。回溯上限 10,000 条事件；无法完整核验时显示错误，不报告撤回成功。撤回不能收回别人已保存或转发的副本，也不删除服务器上全部媒体副本。

## 验证与边界

**实测结果：201 项单元测试通过、2 项忽略；14 项原生流程检查通过；937 条双语文案、986 个翻译调用点检查无缺项。**

实际结果及截图见 [validation.json](validation.json) 和 [evidence](evidence/)。可直接查看[桌面编辑器](evidence/18-desktop-editor-en.png)、[主题选择](evidence/06-theme.png)、[封面设置](evidence/07-cover.png)和[全文预览](evidence/08-full-preview.png)。测试使用隔离的临时账号资料与真实 Palpo 服务，没有操作个人 Matrix 账号。原生自动化通过 Makepad 输入桥接驱动真实控件，截图来自运行中的应用。

```sh
cargo test --offline --features agent_chat --lib
python3 tools/wechat-ux/check_i18n.py
# 需要本机已配置、被 Git 忽略的 disposable Palpo fixture：
python3 tools/wechat-ux/live/native_article_v2.py
python3 tools/wechat-ux/live/native_article_v2_encrypted.py
# 可选的主题/原生链接/桌面定点检查，参数必须是上述测试生成的 sender 目录：
python3 tools/wechat-ux/live/native_article_v2_layout.py <disposable-sender-root>
```

仍未完成的规划增强：封面拖动/缩放手势、目录点击跳转、预览宽度切换及阅读进度、细分上传进度、文章库保存时间/删除入口、聊天文章卡片封面缩略图。封面当前用焦点滑块，正文图片和阅读器封面已实现。

尚未验收：真实 iOS 设备与屏幕键盘、真实中文输入法组合过程、系统文件选择器自动化、掉线/进程中断故障注入、多设备同时修改、独立逐页视觉评分。插图测试使用生成的示例素材并通过真实原生素材选择器，图像预处理另有单元测试。**功能测试通过不等于 9/10 相似度；目前没有给出该评分。**

本次是可运行的原生实现与开发分支验证，未宣称上述未测场景已达发布验收。
