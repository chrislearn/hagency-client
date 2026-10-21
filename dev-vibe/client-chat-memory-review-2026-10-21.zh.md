# 聊天窗口、SDK 与客户端/GPU 内存预算复核

复核基线：客户端 `ea54c34f6e5418c3fd6c8bf160e06836baa66440`，修复代码提交 `8bf7504a0c2aefc3b74fda1acee0868ade395986`，另附本报告与记忆提交。本轮不引入 vendor；SDK 修复发布到 `chrislearn/matrix-rust-sdk` 的 `chris/bounded-chat-history`，固定提交 `fc6a65d45244a280b03115688c8e73971a8e6c14`。8 个 SDK workspace patch 与 lockfile 同源同 rev。SDK fork/main 保持 `f06f62654e155be50be091ffef5304025b5928da`。Makepad 仍固定 `77c8039bc0910f7099bfe4c6f6106a74ae783528`，本轮未改。

## 本轮发现并修复

1. **旧操作发送端将所有历史端点串成强引用链。** 长下载跨越多次历史上下文切换后，中间端点无法释放，查找目标也随切换次数增长。改为共享路由直接持有最新端点的弱引用，汇总发送者存活数；锁按地址顺序取得，旧位置快照仍拒收，编辑/上传/下载结果继续转交。新增 20,000 次替换、并发 2,000 次替换/发送、旧发送者存活和接收方已关闭的测试。
2. **图片查看器接管刚解码完成的缩略图时可能漏记预算所有权。** 解码完成到周期回收之间，租约尚未发现纹理；查看器提前接管后，原行回收可能释放正在显示的纹理。接管前先刷新已完成的缓存纹理，再登记查看器的弱所有权。回归覆盖“尚未周期回收即接管”与查看器释放后的回收。
3. **预览请求总寿命没有上限。** 单次媒体 HTTP 超时不覆盖并发排队、版本协商、token 刷新、SDK 重试和缓存锁。现在整条预览链路共用 30 秒截止时间，取消释放 RAII 准入；测试持有全部下载许可，连续 160 次排队超时，验证请求许可不泄漏，并覆盖响应头之后慢正文和后续恢复。
4. **上下文切换只隔离迟到结果，没有取消旧构建。** 新焦点取代旧焦点立即 abort，隐藏/关闭窗口取消构建，整次构建 30 秒超时，时间线销毁取消焦点任务。成功交接前解除自身 abort handle，避免替换旧状态时取消自己。现有账号、revision、目标及原时间线 Arc 核对继续生效。
5. **线程初次创建可能长期持有全局创建锁，且关闭并不取消。** 改为登记带 Drop 取消的任务句柄，待创建线程全局最多 16 个；等待创建锁和网络构建各有 30 秒截止时间。关闭/移除房间取消待创建任务；成功或失败仅清理同一 Tokio task ID 的登记，防止关闭后同名重开被旧任务误清理。主时间线创建的锁等待和构建也加入截止时间。
6. **SDK 被取消的上下文构建留下不在缓存表中的状态。** 在旧 SDK 上加入回归，第二次取消即观察到状态数从 1 增至 2。改为网络初始化前登记缓存 owner；失败清理和淘汰都先清状态，再删除 owner，避免清理等待期间取消后失去登记。房间构造还移除了状态已插入、owner 尚未返回之间的额外 await。64 次真实 HTTP 请求取消后状态保持 1，最后失败清理为 0；释放后 Client 和 EventCache 的强引用均降为 0。
7. **预算化 SDK 的磁盘最新消息预览未被首次恢复。** 先前仅查 `room_if_cached`，冷启动预览可能为空，缺候选消息的自动回填也无法启动。现在首次恢复允许短暂取得缓存，不永久 pin；后续通知不复活已淘汰缓存，避免房间数大于预算时不断互相淘汰。原磁盘恢复/自动回填测试恢复通过；新增 40 个预览、淘汰后多轮更新不复活的回归。
8. **本地上传图片预览绕过预算。** 原上传弹窗整文件读取、在独立路径解码并直接创建纹理，`FilePreview` 克隆还可能复制解码像素。现在最多 4 个本地读取、实际读取上限 8 MiB + 1 字节哨兵，校验像素/动画预算后才登记全局编码预算；预览仅共享编码 Arc，UI 通过统一图片预算异步解码/上传，关闭和切换释放图片 owner。超过限额只省略预览，原附件仍可上传。新增文件增长、超限、像素炸弹、Arc 共享回归；原生夹具新增反复打开/关闭真实上传预览控件。
9. **SDK 损坏磁盘缓存无法自动恢复，空缓存发生不必要写入。** 初步将存储恢复失败归入策略差异不准确；逐项复核发现循环 chunk 链会使 `load_last_chunk` 返回错误，预算加载器直接传播错误，导致房间无法重开。现按上游恢复语义清除损坏的可丢弃工作缓存，再从空窗口恢复，不加载完整元数据。空窗口保持首次真实事件的待写更新，不抢先持久化空 chunk；这也修复了延迟持久化与迟到密钥解密的互等。两项原始房间/线程损坏缓存测试、原始迟到密钥解密测试均恢复通过，未删除或放宽原断言。
10. **关闭窗口可能只通知旧请求通道。** 焦点提交创建新端点，而 UI 还没处理重连动作时，关闭会写旧 watch；新端点可能继续认为窗口打开，无法按关闭窗口淘汰。显示/隐藏现在与后台焦点提交共用房间锁，同时更新原通道与同操作路由的最新通道；关闭递增提交版本并取消构建，防止立即重开后旧 poll 继续提交。路由核对也防止旧账号/旧窗口改变新会话。尚未被 UI 接管的接收端在淘汰时作为轻量转交缓冲保留，重建后继续收到已完成及迟到的传输结果。端点身份改为单调创建序号，延迟淘汰动作能清理更旧 UI 的历史，同时不清理更新的替换窗口。回归覆盖这些交接结果与新会话隔离。

## 静态检查与分层回归

- 所有本地编译、Cargo、临时文件、数据库和压测输出均在 `D:/BuildCache/hagency-client`，通过 metadata 核对 target。未执行 Docker 构建。
- 最终冻结源码：应用 555 passed / 0 failed / 2 既有 ignored，history 8 / 0 / 2。应用/history all-targets 严格 Clippy、无默认特性 agent_chat 构建检查、Rust 1.99 fast 警告即错误检查全部通过。最终日志 `review3-frozen-tests.log`、`review3-frozen-clippy.log`、`review3-frozen-no-default.log`、`review3-frozen-rust199-check.log`。
- SDK 最新预览模块 93 passed，包括磁盘恢复、候选回填及新增淘汰循环回归。取消和释放引用回归通过；SDK library 严格 Clippy 通过。
- SDK 原磁盘策略回归：627 passed、4 filtered；最终预算策略适用回归：618 passed、15 filtered，使用完整测试名的 `--exact --skip`，避免误过滤同名前缀的恢复测试。**这不是“SDK 原始全集全绿”。** 初次未过滤全集暴露了预览回退，逐项复核又识别并修复了损坏缓存恢复和空缓存首次写入问题；原测试未删除，也未更改 CI 来掩盖失败。
- 15 个最终过滤项中：11 个要求保留完整磁盘 chunk、原 chunk ID、旧磁盘历史查找/排序或旧 chunk 的分页通知。逐项检查实际断言，预算窗口清旧并重编 ID 后这些旧策略假设不成立；两个等待旧历史通知的用例不再适用于此策略。关闭预算特性后这些测试通过。具体完整名单：`review3-sdk-filtered-cases.json`。初始 18 项清单仅为中间调查记录，不能作为最终分类。
- 其余 4 项在关闭预算特性后逐项同样失败：`test_client_no_cycle_with_event_cache`、`test_client_no_cycle_with_send_queue`、`test_no_refcycle_event_cache_tasks`、`test_well_known_caching`。前 3 项依赖 localhost 初始化任务在 1 秒内退出；第 4 项依赖 well-known 刷新在固定等待内完成。独立 loopback 服务的新增取消/弱引用释放测试通过，不将这些失败冒记为通过。
- 样式审计 257 文件 / 0 违规，theme guard 通过，i18n 2050 条 / 1533 站点 / 0 missing，workspace layout 和 git diff 检查通过。

主要日志均在 `D:/BuildCache/hagency-client`：`review2-sdk-cancellation-baseline.log`、`review2-sdk-cancellation-lifetime.log`、`review2-sdk-preview-tests.log`、`review2-sdk-original-policy-suite.log`、`review2-sdk-bounded-policy-suite.log`、`review2-sdk-clippy.log`、`review2-pinned-final-tests.log`、`review2-pinned-final-clippy.log`、`review2-pinned-no-default.log`。

最终 SDK 验证追加 `review3-sdk-corruption-recovery.log`、`review3-sdk-delayed-key.log`、`review3-sdk-expanded-policy-audit.log`（保留策略差异失败断言用于审计）、`review3-sdk-bounded-policy-suite.log`（618 / 0 / 15 filtered）及 `review3-sdk-clippy.log`。

真实千万条消息持久索引已用最终 SDK 固定版本的当前代码重新验证：数据库重开约 0.001 秒；20 页、10,000 个稀疏匹配的顺序/分页验证通过，含广词和不存在后缀的查询总耗时约 0.986 秒；在千万条消息索引深度新增 1,000 条并查询，再逐条撤回，可搜索消息数恢复为 10,000,000。Debug 验证全程 6.09 秒，最终文件 24,883,884,032 字节。日志 `review3-ten-million-reopen.log`；这是已存在数据库的复核，未重新生成整座千万条消息库，未清 OS 文件缓存，也未把 Debug 时延与早前 Release 时延作同配置对比。

## 发布前长测与远端 CI

首轮完成的原生运行 `D:/BuildCache/hagency-client/chat-soak-20261021-review3`，请求 1,800 秒，实际 1,803.235 秒，exit 0；307 个采样点，记录至少 38,375 次计时驱动、307 次 SDK 上下文替换，覆盖百万和千万逻辑历史两阶段。每次上下文同时更换 UI 端点，最旧操作发送者存活整个运行；周期打开/关闭查看器和上传预览，并缩放窗口。源码在构建后全程冻结，终态 16 个源码与二进制 SHA256 全匹配，进程确认退出，stderr 为空。二进制 SHA256：`26FF32994A16447D650D44AEF77A34FD72A5225631ACD4255DE6CD1EDBA5340D`。

本机使用 Rust 1.98.0 release、`chat-stress` 与 stats_alloc，实际 Windows D3D11；硬件清单含 GTX 1060 6GB，详细环境见 `environment.json`。1.99 编译兼容性另有严格检查，未将其当成另一轮原生长测。指标按原预设门槛验收，没有完成后放宽门槛：

| 指标 | 最终采样结果 | 验收门槛 |
| --- | ---: | ---: |
| 预热后 Rust 保留分配增长 | 0.354 MiB/分钟 | ≤ 1 MiB/分钟 |
| 预热后进程私有字节增长 | 0.668 MiB/分钟 | ≤ 5 MiB/分钟 |
| 预热后 GPU 专用/共享增长 | 0.013 / 0.002 MiB/分钟 | 各 ≤ 1 MiB/分钟 |
| 私有字节峰值 | 272.32 MiB | 观察项，非 RSS 硬限 |
| Windows 工作集峰值计数 | 278.19 MiB | 观察项 |
| GPU 专用/共享峰值 | 44.91 / 5.67 MiB | 驱动观测值 |
| 纹理 / 渲染实例最大采样 | 21.38 / 0.374 MiB | 256 / 32 MiB |
| VM / recordings 最大采样 | 24.14 / 0.955 MiB | 128 / 32 MiB |
| 时间线项 / 行控件最大采样 | 103 / 8 | 205 / 64 |
| widget tree 节点 / path 最大采样 | 1,648 / 585 | 20,000 / 50,000 |
| 渲染实例准入拒绝累计 | 0 | 0 |
| 40ms 驱动计时器间隔 P99 最大值 | 51.46 ms | 观察项，并非 GPU 帧时延 |

`summary.json` 的 `acceptance_errors` 为空；stats_alloc 的重分配增减已包含于 allocated/deallocated，保留分配计算未重复加 reallocated。已检查启动、查看器、上传预览、千万阶段、959 万位置及结束前的 drawable PNG；消息顺序正确，未出现局部空白。完整证据为本目录 `manifest.json`、`hashes-final.json`、`completion.json`、`summary.json`、`samples.jsonl`、`process.jsonl` 与 PNG。

客户端 main 发布包含修复代码及协作文档。远端核验按最终推送 HEAD 筛选 `CI`、`Hagency App CI`、`Hagency App Build CI`、`Spelling`，记录各 run ID、URL、状态和结论；最终实际执行记录统一保存于 `D:/BuildCache/hagency-client/review3-remote-ci-final.json`，不以基线 main 的成功记录替代。

`chat-soak-20261021-review2` 在 548.535 秒由代理主动正常关闭，以纳入 SDK 缓存恢复修复；完成记录仍保留请求 1,800 秒和实际时长，标记 superseded，不计为最终 30 分钟通过。该中间版本已检查聊天、查看器和上传预览的 drawable 截图，且二进制及 16 源码哈希一致。最终运行必须采用新 SDK 固定版本和新的输出目录。

## 结论适用范围

原生长测采用实际 RoomScreen、真实 SDK 时间线与 Windows D3D11，loopback 服务提供百万/千万**逻辑历史位置**，不是在公网账号灌入千万条消息。压测夹具这次增加了每个上下文更换 UI 端点、整个运行保留最旧操作发送者，覆盖本轮修复的生命周期路径。持久数据库千万条实存验证另行记录，二者不混为一项端到端证明。

限额覆盖已声明的时间线工作窗口、媒体准入、独占渲染上下文纹理/instances/recordings/VM 等组件；不是全进程 RSS 或显卡驱动内存的绝对硬上限。SDK state/crypto、完整文件下载、旧媒体缓存先读出再校验大小，以及按房间保存的草稿/操作状态等，仍有各自的存活规则。没有把有限场景长测等同于所有账号规模和所有网络故障的穷尽证明。
## 远端 macOS 复核修复与最终重测

首次发布后的真实 macOS ARM Clippy 暴露 `TimelineUpdate::SendFailedBeforeBeingQueued` 比其他枚举成员大超过 200 字节；Windows 的布局差异使本地未触发同一 lint。上传预览共享编码数据后，另一个成员变小，显露了这一差异。将发送失败正文改为 `Box<RoomMessageEventContent>`，减少通知队列中每个枚举项的内联大小；四个发送失败入口均更新。新增跨上下文替换转交失败消息的回归，验证中文/emoji 正文保留。未关闭检查或修改 CI 策略。

该补丁本地应用测试 556 passed / 0 failed / 2 既有 ignored，history 8 / 0 / 2；严格 all-targets Clippy、无默认 agent_chat、Rust 1.99 fast `-D warnings` 均通过，日志为 `review4-{tests,clippy,no-default,rust199-check}.log`。最终补丁的独立 1,800 秒原生复测输出目录为 `D:/BuildCache/hagency-client/chat-soak-20261021-review4`；实际完成时长、预设门槛判定及源码/二进制校验，以该目录 `completion.json`、`summary.json`、`hashes-final.json` 为准。上文 review3 的数值与 SHA 属于此枚举布局补丁之前的完成记录，不冒作最终补丁的哈希。最终 main 的四条真实 CI 结果仍以 `review3-remote-ci-final.json` 中精确 head_sha 为准，失败的旧发布记录保留在 `review3-ci-history.jsonl` 和 `review3-ci-macos-native-failed.log`。
