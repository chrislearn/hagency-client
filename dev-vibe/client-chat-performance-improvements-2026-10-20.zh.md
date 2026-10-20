# Client 大规模聊天历史：实施与复核报告

> 2026-10-21 后续：客户端/GPU预算及正确协议顺序下的30分钟原生RoomScreen长测已完成。最终源码测试541项通过，完整指标、已修复泄漏及覆盖边界见 [客户端内存与原生滚动复核](client-chat-memory-soak-2026-10-21.zh.md)。本轮没有提交推送。

日期：2026-10-20。状态：本轮历史工作集限额、持久加密搜索及分层压测已实施并完成下述复核；不把存储测试结果当作整个进程或 GUI 的性能认证。

## 代码与依赖管理

实际代码仓库为 `D:/Works/hagency-org/hagency-client`。本轮工作从已拉取的 main `bbcc0b39037774400b0bdabbbe8af4072b247780` 继续，客户端工作分支为 `chris/bounded-chat-history`；客户端未提交、未推送、未发布。保留既有自动更新考察文档，未重启或操作用户的客户端会话。

按用户要求，SDK 补丁维护在 [chrislearn/matrix-rust-sdk](https://github.com/chrislearn/matrix-rust-sdk/tree/chris/bounded-chat-history) 的独立分支 `chris/bounded-chat-history`，基于 crates.io SDK 0.19.1 的原始源码提交 `b18166c68bb958a21f0bca8b2d8320cb53583362`。客户端根 Cargo.toml 的 patch 统一固定到 `7e5a2750bb70ae12ecae66c39c98269f17e32856`，SDK、UI、base、crypto、SQLite、store-encryption 等使用同一 Git 来源，避免重复 SDK 类型。fork 分支的提交与推送是本次用户授权的依赖维护操作，两个仓库的 main 均未修改。未创建 PR。

不使用 vendor，也不修改 Cargo 注册表或 Git 缓存。迁移前的临时 SDK 副本已移至 D 盘构建目录作恢复备份，不在客户端工作树或依赖解析路径中。新增 `hagency-history` 的 version、edition、rust-version、license、publish、lint 和依赖版本全部继承根 workspace；版本为统一的 0.6.0。子 crate 仅声明自身需要的功能，例如 SQLite bundled。

## SDK 到 UI 的历史工作集

| 层 | 最终策略与边界 |
| --- | --- |
| SDK room/thread 实时 raw cache | 最多 512 个事件、8 MiB raw JSON；达到任一限额后 Clear 并保留符合预算的最近 128 条；单条 raw JSON 上限 256 KiB |
| SDK 解码 timeline 与关系元数据 | 同一 Clear 更新经过原 SDK UI controller，重置远端项目、聚合和位置元数据，保留本地发送队列的 local echoes；不能仅截断应用 Vec |
| SDK Event focus 上下文 | 初始上下文及分页写入前检查 512 条/8 MiB；超额显式返回 HistoryWindowFull，由应用围绕可见事件重新建窗口 |
| SDK 磁盘 chunk 元数据 | 冷启动只加载末块，不读取全历史 chunk 的排序元数据；旧版本的过大末块收缩为有界后缀；旧的可丢弃缓存重新建立 |
| SDK SQLite event payload | 事务内清理脱离全部 room/thread/pin chunk 的 payload；每房间最多保留 64 个 detached 事件；仍被其他 chunk 引用的事件保留 |
| SDK 房间与子上下文 | 房间 cache 最多 32；每房间 thread/context 各最多 32；回收无使用者对象，全处于使用状态时拒绝新建 |
| SDK 旁路载入与置顶关系 | save_events 收集前检查 count/bytes；置顶关系流逐步检查预算，避免先 collect 全部结果再检查 |
| 应用 timeline 生命周期 | joined room 先登记元数据，首次使用才建立 timeline；最多保留 16 个主/线程 timeline，优先释放已关闭窗口，禁用房间列表的预分页 |
| 应用历史窗口切换 | 第一次从 live 回翻、窗口达到阈值或 SDK 达到预算时，以可见 EventId 重建上下文；账号、Arc、修订号、目标和打开状态守卫防止旧任务覆盖 |
| UI 状态回收 | 清理消息、媒体、链接与绘制缓存，保留草稿和可见事件/滚动偏移；重新打开时通过上下文恢复锚点；mailbox 身份阻止旧回收动作清掉新窗口 |
| UI 更新与事件 ingest | 按本批最终变化项处理，合并连续内容快照；控制事件保序；每轮最多消费 64 条/约 4 ms 后让出 |

Clear 后重新通知 fully-read marker，防止丢失读到位置。跨窗口的编辑、置顶、上传、下载操作结果继续转交，过期位置与分页结果不转交。主 timeline 的 typing 订阅绑定 Room，避免隐藏窗口因无关订阅继续持有旧 Timeline。复核发现 SDK latest-event preview 的 OnceCell 长期持有全部房间 cache，导致容量未满就拒绝新房间；已改为只在计算期间持有，并在预览刷新时只查询现存 cache，避免重新拉起已回收房间。40 房间的真实 SDK 饱和/关闭回收测试已通过。UI 窗口替换和回收同时清掉 thread summary、在途 summary 标记和展开预览，旧 summary 结果不得重新填充已离开的窗口。

上述限额针对历史工作集，不能相加后宣称整个进程 RSS 有绝对硬上限。单次 HTTP/sync 缓冲、成员与房间 state、crypto sessions、durable send queue/local echoes、传输操作、GPU 解码纹理及分配器开销仍各有独立生命周期。现有媒体和头像分别是 32/16 MiB 回收目标，单个可见超大资产可能超过目标。可靠控制/操作结果队列和草稿元数据也没有用丢弃操作的方式强行限容。

## 持久加密搜索

新增 `crates/hagency-history`，应用适配位于 `crates/hagency-app/src/history_store.rs`，UI 集成位于 `home/room_history.rs`。

- SQLite 保存原始事件与派生搜索消息的认证密文；SDK StoreCipher 生成随机索引主密钥，已有受保护会话的随机数据库口令经 homeserver/user/device/room 分域派生 key 后包裹主密钥。不会在 SQLite custom KV 写入口令、正文或明文搜索词。
- 文件按账号、设备、房间隔离；打开时检查实际 SDK Client 与持久会话的用户、设备、homeserver，以及加密索引内的 identity。口令、派生 key 和读取会话的字节使用 Zeroizing。
- Unicode 小写 1/2/3-gram blind index 支持中文子串、显示名和文件名；附件种类另建 keyed token。查询最多抽样 32 个 gram、每个计数最多 1024 个 postings，优先选择稀疏候选，再逐条解密做精确匹配。
- 结果按 timestamp 降序、keyed EventId digest 升序稳定分页，最多取 501 条、显示 500 条。等时间戳与较早时间段分两次 keyset 查询，避免 OFFSET 和 OR 使等时间戳翻页退化。
- 每批最多 1000 条，应用实际分页 100 条；事件序列化预算 256 KiB、不同 substring token 最多 16384；加密记录读取先查大小；SQLite page cache 2 MiB、禁用 mmap、单房间数据库 32 GiB 配额。超额返回错误并回滚，不静默跳过或伪造完整覆盖。
- 每次查询检查取消；候选超过 32768 或耗时达到约 3 秒时返回“查询过宽”错误，避免读事务持续阻塞更新。该检查位于候选之间，不能保证单条解密或调度绝不超时。
- 正常编辑须与原发送者匹配；同时间戳编辑按真实 EventId 决胜；作者 hash 索引避免大量伪造编辑使查找反复扫描全部关系。撤回删除原始与派生记录，并保留 tombstone，重叠历史页不能复活已撤回内容。
- unavailable-to-decrypt 记录保留加密源和 keyed session 索引；每次最多重试 100 条，按 session 和 keyset 继续；全账号共用一个解密工作入口，避免每条历史一个任务。
- 搜索进度、事件变更和 continuation 同事务提交；独立的 `/messages` forward cursor 追赶编辑/撤回，不依赖 SDK 已提交的 sync token。SDK 房间广播只是唤醒，错过广播后仍按自己的游标追赶。
- forward 与 backward 游标在写事务内分别合并并校验旧值；索引重建旋转 generation，旧网络页和旧解密任务不能回写到新一代索引。初始尾部页提交后再次追赶，覆盖抓页与入库间的同步窗口。空页只在 continuation 不前进或消失时结束。
- watcher 更新失败会持久标记 stale；搜索不返回标记失效的旧结果。标记也写不进去时当前账号搜索进入失败状态。重试可重建派生索引；网络错误后的重建可能昂贵，不能声称无成本恢复。打开持久结果前须完成追赶，因此目前不提供网络不可用时的旧结果降级展示。

这是认证加密 payload 加 blind index，不是 SQLCipher、全库认证或防回滚数据库。SQLite 仍可见时间、数量、keyed 标识相等关系、词频与访问模式；有权读取既有会话秘密的攻击者也能解开索引。磁盘配额按房间计算，journal 和多个房间/账号的累计磁盘占用不由该单库配额覆盖。真实长文本的 tokens/磁盘放大可能远高于本次短消息数据集；达到限额后应显示错误，不能承诺任意内容的一千万条都能放入 32 GiB。

## 复核与压测证据

测试机器为 Windows 11 Pro 原生 Intel Core i7-14700KF（20 cores/28 logical processors，约 80 GiB RAM），所有构建、Cargo、临时数据与日志在 `D:/BuildCache/hagency-client`。Cargo 经 `tools/dev.ps1` 设置并用 `cargo metadata --no-deps --format-version 1` 核对 target；无 Docker 构建。

1. **真实落盘加密压力测试：通过。** 真实唯一 EventId、递增时间戳，每 1000 条一批，通过生产 insert/encryption/token/SQLite FULL transaction 路径落盘到一千万条。正文为单个 CJK 字符，每 1000 条一条命中“针”；不是代表长文、图片或所有内容分布。日志 `history-stress-20261020-a.log`，数据库 `history-stress-20261020-a/search.sqlite`。
2. 已完成的 1,000,000 条 checkpoint：387.555 秒，2,477,285,376 bytes；501 个中文命中 warm 11.140 ms、重开连接 10.583 ms、最新页 9.957 ms。测试进程观测 peak working set 14,000,128 bytes；它不是 GUI 或整机峰值。
3. **SDK 真实链路测试：通过。** 本地 HTTP + SDK + SQLite 12,000 条实时事件反复回收，检查 raw count/bytes、decoded items、chunk metadata、detached payload；大正文在 count 未满时触发 byte compaction；fully-read marker 在仍位于新窗口内时保留；上下文反复同锚点、40 次失败后的回收；全活跃房间容量与关闭后的可回收性。
4. **百万/千万逻辑服务端历史随机定位：最终代码通过。** 每档 40 个远距目标，服务端只生成请求窗口，最多 101 条上下文；断言直接 `/context`，没有 `/messages` 线性遍历。此项是规模独立的定位证明，与一千万条真实存储压力测试分开统计。
5. **索引正确性回归：通过。** 密文泄露探测、错误密钥/identity、中文/文件名/附件筛选、编辑授权和决胜、撤回/重叠页、迟解密/session 翻页、同时间戳无丢失分页、取消、损坏/替换密文、序列化/token/磁盘预算与事务回滚、generation 隔离、双向游标合并、schema 升级和 SQL keyset 执行计划。本地真实 SDK forward replay 覆盖 SDK 已经提交 sync 后的索引恢复。
6. 既有 390/1280 宽度离线 UI 内容绘制回归包含在完整应用库测试；不能替代真实账号、网络、显卡和长时间滚动输入验收。

真实长测使用开始运行时的生产版本；最终源码后来增加了索引和游标防护。最终 Git 依赖图已重新打开同一份千万条数据，核对总数、20 页/10000 条中文命中、常见词、无匹配后缀及附件空筛选；并在实际千万条 B-tree 深度下用最终代码追加 1000 条包含中英文本的记录、查询、撤回清理并将 messages 计数恢复为 10000000。复核重复执行仍通过（使用唯一 EventId；留下少量 tombstone/可复用数据库页）。重开连接不等同于清空 OS 文件缓存。

| 数据/阶段 | 实测结果 | 说明 |
| --- | --- | --- |
| 1,000,000 条真实入库 | 387.555 s；2,477,285,376 bytes | 起始生产源码、release、1000 条/事务 |
| 10,000,000 条真实入库 | 5,882.691 s（约 98.0 min）；24,839,065,600 bytes | 同一连续真实数据集；期间同机也在构建/运行回归，不是独占机器基准 |
| 千万 checkpoint 查询 | warm 15.633 ms；重开连接查询 10.578 ms；最新页 10.329 ms | 各返回最多 501 条；原始长测版本 |
| 长测进程内存 | peak working set 14,000,128 bytes | PowerShell 每秒采样并读取进程峰值；最终监控 JSON 已落盘 |
| 最终代码首次打开旧千万库 | 58.563 s | 增加 author 列与 partial indexes 的一次升级扫描；这是 O(N) 升级成本，不能隐藏成常规毫秒打开 |
| 升级后最终代码重开 | 约 1 ms | 同库再次执行，已有 schema；没有清空 OS 缓存 |
| 最终代码中文全结果翻页 | 20 页、10,000 条；290.868 ms | keyset 分页断言顺序和总数，约 14.5 ms/页 |
| 最终代码常见词结果页 | 12.824 ms | 501 条“乙”结果；不是扫描全部千万条正文 |
| 最终代码无匹配长后缀 | 0.020 ms | rare gram 避开常见前缀的候选扫描 |
| 最终代码千万规模追加 1000 条 | 1,496.312 ms | 包含真实加密、tokens、FULL commit 与一次查询；随后撤回恢复总数 |
| 最终首次升级/查询/追加复核进程 | peak working set 14,110,720 bytes | 与原长测进程分开采样；非 GUI 全进程预算 |
| 百万逻辑历史 40 次随机上下文 | 458.317 ms，总计 | 最多 101 条远端上下文；本地 HTTP，不含真实公网延迟 |
| 千万逻辑历史 40 次随机上下文 | 451.937 ms，总计 | 无 `/messages` 线性扫描；不能据此推算公网帧率 |

首次最终复核后库大小 24,867,786,752 bytes，再次追加/清理复核后为 24,873,299,968 bytes；messages 总数均恢复到一千万。旧 SDK 缓存的首次清理也可能有随旧数据量增长的磁盘成本；受限的是常驻历史工作集，并非声称所有升级操作 O(1)。

## 最终检查状态

- 完整应用库：532 passed、0 failed、2 项既有 ignored，实际 534 项；索引库：8 passed、0 failed、2 项显式长测 ignored，实际 10 项。日志 `history-final-complete-tests.log`。索引 release 普通回归另有 `history-final-library-tests.log`，8 passed。
- 两个长测分别显式执行且通过：`history-stress-20261020-a.log`（真实千万写入）和 `history-final-ten-million-verification.log`（最终代码重新打开、搜索、追加与撤回）；常规库测试里的 ignored 不被冒充成普通回归通过。再次重开验证见 `history-final-ten-million-steady-reopen.log`。
- `cargo clippy --locked -p hagency-app -p hagency-history --all-targets --no-deps --features 'agent_chat,octosense-module' -- -D warnings`：最终源码通过，日志 `history-final-exact-source-clippy.log`。
- `cargo check --locked -p hagency-app --no-default-features --features agent_chat`：通过，日志 `history-validated-no-default.log`。
- 最终逻辑历史测量：`history-final-logical-navigation-metrics.log`，明确运行 1 项目标测试而非空过滤。
- 最终原生样式审计：252 个文件/0 violations；i18n：2037 个 catalog entries/1529 处调用/0 missing；主题字面量 guard 与 `git diff --check`：通过。
- TOML/lock 核对：新 crate package 与 dependencies 的版本均继承 workspace；SDK 公共类型相关包各只有一个固定 fork Git 来源；工作树无 vendor；客户端 HEAD 未产生新提交，fork 工作树干净。

上述日志和两个进程的内存监控 JSON 均在 `D:/BuildCache/hagency-client`。没有把耗时很短的 0 项测试结果用作通过证据。

未过滤依赖的严格 Clippy 暴露 article-makepad 既有 27 条 lint；不为本任务改动无关代码。对本次修改 crate 使用 --no-deps 的严格 Clippy，不能把它描述成整个仓库全依赖零 lint。中间失败日志保留，失败或 0 测试均不能计为通过。


## 交付边界

本轮已把消息历史的 raw cache、decoded controller、关系/位置元数据、chunk/payload 工作缓存、上下文、应用 timeline 生命周期和搜索索引串起来做有界处理。仍不承诺整个 SDK/进程/GPU 的绝对字节上限，也没有用真实服务端账号的千万条混合媒体历史做端到端滚动/FPS、断网和显卡验收。索引只覆盖用户实际检查/载入的历史；首次构建旧历史仍需按页从服务器获取，不会凭空建立完整的远端索引。

客户端代码与报告保持未提交、未推送；仅用户授权的 SDK fork 分支已推送供固定依赖使用。自动更新考察文档保持原样。
