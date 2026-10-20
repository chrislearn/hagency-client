# 客户端聊天内存预算与原生滚动长测（2026-10-21）

状态：实现、复核及最终原生长测完成；用户随后授权将全部客户端改动提交并推送远端 main，正在完成发布前集成检查。

最终生产源码在 Windows 原生 D3D11 / Release 下完成 **1800 秒**连续滚动：**38,250 次定时滚动输入、306 次 SDK 上下文替换**，覆盖百万与千万逻辑历史，每次验证消息时间顺序。预算和预设增长率检查全部通过。修复了 WidgetTree 脱离节点泄漏、图片解码/编码预算缺口及查看器迟到结果保留；未将全进程 RSS 或驱动 VRAM 声称为绝对硬限。

| 最终运行指标 | 结果 |
|---|---:|
| 进程完整存活/退出 | 1803.741 秒，exit 0，已退出 |
| Private Bytes 峰值 | 267.66 MiB |
| 纹理池峰值 | 21.38 MiB |
| 驱动专用 / 共享 GPU 内存峰值 | 43.79 / 5.58 MiB |
| 图片工作额度峰值 | 12.00 MiB |
| GPU instance buffer 峰值 / 拒绝 | 0.36 MiB / 0 |
| SDK UI 项 / 渲染行控件峰值 | 103 / 8 |
| WidgetTree 节点 / 路径缓存峰值 | 1617 / 572 |
| Script heap / recordings 峰值 | 24.04 / 0.95 MiB |
| 字体排版 cache 峰值 | 3.48 MiB |
| 预热后 Rust live / Private Bytes 增长率 | 0.34 / 0.91 MiB/分钟 |

缓存预热与分配器保留仍存在，不把这些斜率描述为“所有内存完全不再增长”。最终检查限值为：Rust 保留分配 ≤1 MiB/分钟、private ≤5 MiB/分钟、驱动专用/共享 GPU 各 ≤1 MiB/分钟；控件树数量保持同一量级，未再出现旧版按滚动行数持续保留的趋势。

## 变更范围

客户端 `D:/Works/hagency-org/hagency-client`，分支 `chris/bounded-chat-history`，基线 `bbcc0b39037774400b0bdabbbe8af4072b247780`。保留先前的 SDK 窗口和持久加密搜索改动。本次不修改 SDK fork。

渲染器在真实独立 Git checkout `D:/Works/chrislearn/makepad` 修改，origin 为 `chrislearn/makepad`，分支 `chris/bounded-chat-memory`，基于 `1f3b1dedfbb81424eb8dbf69e5e2c634fa73dc54`。没有 vendor，也没有改 Cargo Git 缓存。用户随后授权发布，渲染器修复已提交并推送到 fork 独立分支，固定 rev 为 `77c8039bc0910f7099bfe4c6f6106a74ae783528`；根 workspace 的全部 82 条同源 patch 已切换为该 Git rev。fork/main 与原仓库主分支未改。压测脚本从 Cargo metadata 定位实际渲染器源码，不再要求 sibling checkout。

新增依赖 stats_alloc / dhat 的版本放在 workspace，应用使用 workspace 继承；仅可选测试 feature 启用。所有 target、Cargo 缓存、临时数据、原生抓图和压力日志在 `D:/BuildCache/hagency-client`，dev.ps1 在执行 Cargo 前用 metadata 校验实际 target_directory。未使用 Docker。

## 实施的预算

| 范围 | 限额/方式 | 达到限制的行为 |
|---|---|---|
| 聊天编码图片的活动 Arc 分配 | 全局 64 MiB、最多 512 个不同分配 | 拒绝新图片，保留文本/占位 |
| 单张自动图片/头像/查看器输入 | 8 MiB | 流式接收超过阈值即停止；缓存旧条目读取后复核 |
| 单张解码像素/动画 atlas | 16 MiB | 分配前拒绝；实际解码任务按该图片预留的像素量再收紧 |
| 图片在途、解码、CPU 像素、上传余量 | 全局 128 MiB 预留额度 | RAII 预留失败拒绝；离屏在途任务保留额度直至结束 |
| 图片 GPU 准入 | 实际纹理池 + 所有待上传预留估计不超过 256 MiB | 停止新图片准入；释放无读者的纹理及 CPU pixels |
| GPU instance buffers | 独占 Cx 下 32 MiB | 后端保留预算约束；按已完成提交回收 |
| draw publication / recordings | 16 / 32 MiB | 使用渲染器已有有界设施 |
| Script VM heap | 独占 Cx 下 128 MiB | 32 MiB 起周期性安全点 GC，再由堆限额守卫 |
| 自动预览网络 | 共 128 个活动/等待请求，4 个并发 | 超额拒绝，30 秒超时，有限重试 |
| 头像完成队列 | 256 条 | 溢出唤醒 UI 并释放等待占位，允许之后重试 |
| 文字排版 cache（渲染器既有策略） | 16 MiB 估计目标，当前帧可见集受保护 | 超出部分在后续帧回收；不是不可超过的硬限 |
| 后台 Tokio runtime | worker ≤4，blocking ≤8 | 排队执行，不随 CPU 核数无限扩线程 |
| 查看器解码 | 共享的有限任务池 | 满载返回可见错误，不为每张图新建线程 |

图片工作额度按像素/动画 atlas 的 6 倍估计，覆盖常见解码 scratch、合成、像素输出及 mip/上传余量；该估计不等于所有第三方分配器或驱动的逐字节计量。共享宿主 Cx 不改宿主 VM、instance、publication、recording 配额，有单独测试验证。

图片加载保留 SDK 的 HTTP/TLS client、协商 endpoint、access token 刷新、SDK media store，以及附件完整性校验。明文/加密内容、超大 Content-Length、分块超量、404、旧版 r0 endpoint 和缓存命中均通过真实 loopback HTTP 测试。加密内容校验到 EOF 后才进入缓存。

动画额外检查 GIF 子帧画布边界、PNG acTL/fcTL 一致性、WebP 动画容器和帧数、ICO 内嵌尺寸。最多 4096 帧/容器块，GIF subblocks 另有限额。聊天 SVG 预览不自动展开，使用原附件保存路径；这是防止任意矢量展开占用的明确产品取舍。大图无法预览时提示保存后打开，新增中英文翻译。

## 发现并修复的问题

1. `Cx::set_global` 只在未注册时插入，不能用于恢复已取出的 ledger。早期 smoke 因此丢失图片账本，已改直接赋值恢复；修复后纹理池稳定在约 20–22 MiB。
2. 隐藏控件仍保留纹理和成员头像子控件。TextOrImage、Avatar、图片查看器切换/清空时主动解除纹理及读者；弱引用不会延长离屏行生命周期。
3. 头像完成队列原先无界，且旧账号回调可能写回新缓存。现在队列有限、回调携带 generation，旧 generation 被丢弃。
4. 图片错误转换顺序会把 HTTP 404 错当连接错误。先判断 HTTP status 后再映射网络错误。
5. 图片头预检与实际解码默认策略脱节。独立 Makepad fork 新增 request-scoped decode limit，异步 worker 继承并在退出/展开时恢复；嵌套只能收紧，其他模块默认策略不变。WebP 内部 memory limit 也继承，避免只凭外部容器头估算。
6. 查看器解码失败/通道断开后未清理 receiver，阻止再次加载。现清理并显示对应错误。复核还补齐其全尺寸回调在进入 UI action 队列前的全局编码额度准入、查看器入口复核；reset 清除当前资源标识，按当前 MXC 过滤迟到/其他图片的完成与错误，避免关闭后重新保留数据。超量与 reset/资源替换均有回归。
7. 最主要的长滚动泄漏在 Makepad WidgetTree：lookup-only 刷新覆盖旧 children，却只在 mark_structure_dirty=true 时移除旧子树。旧 graph nodes、path cache 和 Weak Rc 从此失去可达的清理路径。改为所有刷新都清理脱离的子树，保留 lazy dense rebuild 行为。新增连续 2,000 轮行替换回归，直接检查保留节点及路径缓存数量。

## 根因证据与压力方法

修复前真实原生窗口运行约 26 分钟：GPU 图片池约 20 MiB 已稳定，进程 private bytes 仍超过 1 GiB。另一轮 Rust 分配计量中，153 秒约 218 MB、359 秒约 312 MB、753 秒约 507 MB，排除单纯驱动显存增长。60 秒 dhat 采样定位到 WidgetTree GraphNode map、PathCacheEntry 和 WidgetWeakRef 分配长期保留。

这些早期定位运行使用 Debug 构建；最终验收使用 Release，不能据两者给出同配置 A/B 的内存降幅或速度提升百分比。

证据位于 `chat-soak-20261021-a`（提前关闭用于修复，不能算完整 1800 秒通过）、`chat-soak-20261021-c`、`chat-heap-20261021/heap.json`，均在 D 盘压力目录。

最终 fixture 使用生产 RoomScreen、PortalList、文本/富文本/CJK/emoji、每五条一张真实 PNG；运行真正 Matrix SDK /context，但历史服务是隔离 loopback，按需生成逻辑 1M/10M 的上下文。每 40ms 请求一次滚动、交替方向、约六秒换一个随机上下文，周期性打开关闭图片查看器、调整原生窗口大小。图片来自预置媒体缓存，Matrix worker 未启用；真实 HTTP 传输/附件解密的边界由独立 loopback 网络回归覆盖。没有登录或向用户真实账号发消息。使用 Windows 原生 D3D11 和 Release 二进制；stats_alloc 仅在测试 executable 计量。

本次数据支持渲染窗口工作集不随逻辑历史总数线性增长，不代表把一千万条事件全部送入 UI。先前真实物理 10M 加密 SQLite 建库、重开、查询压力见前一份性能报告；两类证据须分开解释。

工具：`tools/chat-memory-soak.ps1` 创建全新隔离 profile，保留二进制 SHA256、关键源码 SHA256 和约每六秒进程指标（含驱动计数查询）；`tools/analyze-chat-memory-soak.py` 汇总进程、Rust live、图片、纹理池、instance buffers、VM、recordings、WidgetTree。采样的 timer p99 是事件调度间隔，不能称为屏幕帧时间/FPS。

## 验证结果

- 应用：541 通过、0 失败、2 项既有 ignored；history：8 通过、0 失败、2 项压力 ignored。日志 `viewer-budget-validated-suite.log`；前轮真实物理千万条索引验证不重复算为本轮原生测试。
- 严格 Clippy：app/history all-targets，agent_chat + octosense-module + chat-stress，no-deps，-D warnings，通过。日志 `viewer-budget-validated-clippy.log`。
- 无默认功能 + agent_chat 构建通过：`viewer-budget-validated-no-default.log`。
- Makepad 解码器 `image_cache::tests`：27 通过，`image-decoder-tests2.log`。
- Makepad `widget_tree::tests`：50 通过、2 项基线失败；未修改基线为49通过、同样2失败。失败名为 `test_observe_and_find_single_node`、`test_property_patch_no_structural_rebuild`；不删除或弱化测试。日志 `widget-tree-final-tests.log` / `widget-tree-baseline-tests.log`。
- Makepad Windows 生成绑定仍有2项既有函数指针比较警告；应用 no-deps Clippy 通过不等于整个上游零警告。
- 样式审计255文件/0问题；i18n 2038条目、1530处调用/0缺失；两个仓库 diff whitespace 检查通过。

最终 Release 构建：`chat-soak-ordered-final-release-build.log`，6m08s。最终压力输出为 `D:/BuildCache/hagency-client/chat-soak-20261021-ordered`，其中：

- `manifest.json`：二进制 SHA256 与13个关键源码 SHA256；结束后重新核对全部一致。
- `samples.jsonl` / `process.jsonl` / `summary.json` / `completion.json`：306组原生采样、进程与驱动指标、预算验收、完整存活时长。
- `capture-start.png` / `capture-viewer.png` / `capture-ten-million.png` / `capture-end-room.png`：原生 drawable `/g` 抓图；已查看开头、查看器、959万位置及结束后的正常聊天画面。最后进程 pid3636 已自然退出。

最终 binary SHA256：`830A9440359AD0FA2BDB5E5CC967BCF91A12E7B50C51C15FE3FB7465403C008A`。

复核测试工具时发现首轮 `/context events_before` 返回顺序写反；已修正为 newest-first，并对SDK最终消息索引加严格递增断言。首轮 `chat-soak-20261021-final` 虽完整运行30分钟，仍仅保留为早期定位/压力证据，其原始fixture按启动SHA完整保存为 `fixture-source-at-launch.rs`；它还不包含最后补齐的查看器准入守卫。最终验收采用修正后重新完整运行的 `ordered` 轮，没有把旧数据冒记为最终源码。

## 覆盖边界

原生压力覆盖长期换页和资源回收；没有把每个8/64/128/256 MiB准入阈值都在原生GPU上逼满。超量、并发预留及畸形图片的拒绝边界由单测/loopback HTTP回归分层验证。

这些限制约束聊天自动加载及应用拥有的相关工作集。它们不是操作系统 RSS/Private Bytes 或整个 GPU 驱动 VRAM 的绝对硬上限；字体 atlas、swapchain、驱动、SDK state/crypto、其他业务模块均独立占用内存。

旧 SDK media cache 条目在读出 Vec 后才校验大小；普通 SDK retention 有自身限制，历史 ignore-retention 的本地上传可能更大。显式“保存文件/下载”仍使用既有附件传输，不在自动预览的 8 MiB 限额内，尚未认证任意大小文件下载全程恒定内存。公网真实账号、多个客户端/宿主、音视频及各种恶意压缩数据的完整跨平台压力没有在本次运行中覆盖。不能据此次测试声称全客户端所有子系统都已具备单一 RSS 硬限。
