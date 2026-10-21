# SDK 15 项过滤测试逐项闭环

本轮基线：客户端 `3b5a5a19fb56e05ee65c5a163627df26308d019c`；SDK `fc6a65d45244a280b03115688c8e73971a8e6c14`。本轮 SDK 修复固定为 `28a91dc5f519a1d5f0f828df532faf5e88947da7`，根 workspace 的 8 条 SDK patch 与 Cargo.lock 已统一；`cargo metadata --locked` 验证同源同版本，lock 只更换 8 条 Git source，没有顺带升级其他依赖。SDK 改动只推 `chrislearn/matrix-rust-sdk` 的 `chris/bounded-chat-history`；fork/main 保持 `f06f62654e155be50be091ffef5304025b5928da`。不使用 vendor 或本地 path patch。

## 对上轮判断的纠正

这 15 项是上次 SDK library 运行时被命令行 `--skip` 排除的用例，不是 15 个已经确认的产品缺陷。上轮将其中 11 项整体归为旧磁盘策略不适用，判断过于宽泛：旧测试使用的少量历史完全可以保留在新预算内，它们的查找、分页、清理、排序、通知和跨进程一致性断言仍然有效。

本轮在预算特性开启时重跑原 15 项：13 项报告失败，房间 dirty reload 和线程 storage load 两项等待通知超过 60 秒仍未结束，停止了该次测试进程。原始记录保留为 `D:/BuildCache/hagency-client/sdk15-baseline.log`。不是把原断言改成“找不到旧消息也算通过”，也不通过 cfg 隐藏这 11 项。

## 产品代码修复

1. **预算内历史被过早删除。** 新加载器沿前驱逐块检查，最多检查 512 个块、512 条事件和 8 MiB 原始事件内容，单事件上限 256 KiB。不调用全量 metadata API。整条缓存链在预算内时保留原磁盘块、块 ID 和有界排序元数据，初始仍只把最后一块载入内存，旧消息按需从磁盘分页恢复。
2. **房间加载会清掉同房间线程和其他缓存。** 移除 room load 中的 `clear_all_events(Some(room))`。超限只重建当前 LinkedChunkId，保留同房间其他线程及其他房间。新测试覆盖预算内加载和超限裁剪两条路径。
3. **重载路径绕过预算。** 房间和线程的收缩、自动收缩及跨进程 dirty reload 都使用同一个有界 snapshot 加载器，避免重载时重新读取无限长 metadata。原订阅者仍收到 Clear/Append 等必要通知。
4. **只检查已加载事件会漏掉排序元数据增长。** OrderTracker 提供已更新的 item/chunk 计数；传播新事件前同时检查包含未加载磁盘前缀的排序元数据，超过预算才压缩为最多 128 条的后缀。
5. **裁剪不能被误判为真实历史起点。** 超限工作窗口保留本地边界标记，重开后仍存在；房间和线程分页遇到此边界返回 `HistoryWindowFull`，交由客户端重建焦点窗口。标记在发 HTTP 之前拦截，不作为服务端分页 token 发送。SDK 工作缓存可丢弃，权威历史仍由服务端和客户端独立加密搜索索引提供。
6. **移除全房间清理后不能重新引入无限量重解密读取。** 重解密/加密信息刷新只扫描当前房间及已打开线程的有界磁盘窗口，逐块读取、按事件 ID 去重，结果累计原始内容最多 8 MiB；不拉取旧 detached/未打开线程的全房间事件向量。`None` 过滤条件保留 wildcard 语义，原加密信息刷新和迟到密钥测试继续通过。
7. **清空及升级边界。** 空缓存不主动写入空块；清空后的唯一空块会清掉，后续真实事件可正常持久化。超过标准块容量的旧单块同样走后缀恢复，避免 lazy loader 因块过大而打不开房间。

限额针对工作缓存及上述读取/结果路径，不是对整个客户端 RSS、SDK state/crypto、旧数据库磁盘占用作绝对承诺。旧 store API 返回单个旧块时会先物化该块，再按预算检查；本轮没有把这一接口限制写成“任意损坏/超大旧块都能在物化前硬限”。未打开的旧线程及 detached 磁盘数据也没有被全房间删除，它们不会进入本轮的全量重解密查询。

## 15 项逐项判断与验证

下表简称 `room` / `thread` 分别指 `event_cache::caches::room::timed_tests` / `event_cache::caches::thread::timed_tests`。第 1～11 项的原测试函数及断言保留；第 12～15 项修正测试环境与等待方式，保留原功能断言。每项都在预算开启和关闭的未过滤 SDK library 全集里执行。

| # | 原测试 | 原因与本轮判断 | 修复后验证内容 | 结果 |
|---|---|---|---|---|
| 1 | `room::test_auto_shrink_after_all_subscribers_are_gone` | 加载即删除旧块，分页拿不到应在预算内的早期事件；属于过度裁剪 | 两订阅者共享分页结果；最后订阅者释放后只保留末块，无多余通知 | 两策略通过 |
| 2 | `room::test_clear` | 旧事件已提前删除，使前置查找断言失败；空块处理还影响清空结果 | 磁盘旧消息可查；分页通知正常；clear 发送通知并清空查找、内存与磁盘 | 两策略通过 |
| 3 | `room::test_load_from_storage` | 小缓存被整体重编/丢前缀，并非预算要求 | 初始只加载末块，旧消息磁盘可查；后续分页、generic update 和去重均正常 | 两策略通过 |
| 4 | `room::test_room_ordering` | 任意重编/移除预算内块使旧位置失去排序信息 | 未加载事件的顺序、加载后顺序及 gappy sync 去重后的顺序仍正确 | 两策略通过 |
| 5 | `room::test_shrink_to_last_chunk` | 旧磁盘前缀已丢失，前置分页失败 | 先分页到旧事件，再 reload 末块；Clear/Append 通知与最终事件集合正确 | 两策略通过 |
| 6 | `room::test_reload_when_dirty` | 旧块被丢弃，测试等待应有的分页通知而挂起；重载还绕过预算 | 两客户端共享存储，反复交替持锁变脏后重载，事件、分页和通知一致 | 两策略通过 |
| 7 | `thread::test_auto_shrink_after_all_subscribers_are_gone` | 线程旧块提前丢失，无法完成前置分页 | 线程根与回复分页、多个订阅者共享、最后订阅者释放后的末块收缩 | 两策略通过 |
| 8 | `thread::test_clear` | 预算内历史提前删除；清空后不应残留空持久块 | 旧线程消息可查；分页、Clear/generic 通知及内存/磁盘清空均正确 | 两策略通过 |
| 9 | `thread::test_reload_when_dirty` | 旧块已删除，前置分页结果为空；重载走全量 metadata | 两客户端交替变脏后的线程重载、分页事件及通知正确 | 两策略通过 |
| 10 | `thread::test_load_from_storage` | 房间初始化清掉同房间线程磁盘数据，线程初始化通知不再出现 | 房间和线程各自恢复并通知；旧线程事件可查；分页及重复事件处理正确 | 两策略通过 |
| 11 | `event_cache::tests::test_generic_update_when_paginating_room` | 可保留的磁盘前缀被删除，分页无新事件 | 有新事件分页发送 generic update，空块分页不发伪更新，真实结束状态正确 | 两策略通过 |
| 12 | `client::tests::test_client_no_cycle_with_event_cache` | 关闭预算特性也失败；固定 localhost/代理发现及固定 1 秒等待使后台初始化未退出 | 独立 loopback HTTP 与本地用户域，禁用系统代理；5 秒截止条件等待后仍断言 Client 强引用从 1 到 0 | 两策略通过 |
| 13 | `send_queue::tests::test_client_no_cycle_with_send_queue` | 同样受外部初始化与固定 500ms 等待影响，并不能直接判为无泄漏 | 独立本地服务，队列开启/关闭两种状态都等待引用归零，仍断言 WeakClient 无法升级 | 两策略通过 |
| 14 | `event_cache::tests::test_no_refcycle_event_cache_tasks` | 同样的本机基线失败，不能靠跳过代替释放验证 | 独立本地服务；保留 EventCache 初始强引用为 1、释放后为 0 的断言 | 两策略通过 |
| 15 | `client::tests::test_well_known_caching` | 已在预算关闭时复现；用户域发现/系统代理及固定 5 秒等待影响刷新 | 本地 discovery、用户域、HTTP transport；条件等待刷新，仍检查内存/磁盘缓存、reset、过期刷新及请求次数 | 两策略通过 |

## 新增策略回归

SDK `event_cache::retention::tests` 新增 10 项：

- `test_small_window_preserves_ids_order_and_disk_lookup`：预算内块 ID、懒加载、旧位置顺序和磁盘查找。
- `test_room_load_and_trim_do_not_clear_sibling_thread_or_room`：预算内/超限两种加载都不误清兄弟线程及其他房间。
- `test_exact_event_limit_is_kept_and_one_more_is_bounded`：512 条保留、513 条裁剪；房间与线程都验证磁盘、内存、重开和边界标记。
- `test_empty_chunk_metadata_is_bounded_too`：512 个空块边界及 10,000 空块旧库，防止只限事件、不限元数据。
- `test_bytes_and_single_event_limits_are_enforced_on_reopen`：累计字节、单事件、超过标准容量的旧单块。
- `test_compaction_bounds_unloaded_ordering_metadata`：已加载向量未超限，但包含磁盘前缀的排序元数据超限时必须裁剪。
- `test_oversized_event_is_a_boundary_not_a_hidden_hole`：丢弃超大中间事件时只保留连续后缀，不把前后消息拼成没有缺口的假历史；修复前新断言确实失败，日志 `sdk15-oversized-hole-before.log`。
- `test_clear_leaves_no_empty_persisted_chunk_and_accepts_new_events`：清空后无空块残留，新增事件仍可持久化和重开。
- `test_encrypted_sqlite_reopen_and_thread_isolation`：真实加密 SQLite 的 512 条保留、513 条裁剪、进程内句柄销毁后重新打开及兄弟线程持久数据保留；边界在磁盘中核对，初始只加载末块的懒加载规则保持。
- `test_stored_retry_scan_is_bounded_without_erasing_other_windows`：10,000 块旧缓存的有界扫描、不扫未打开线程、跨活动窗口结果及 wildcard 查询。

另在 room/thread 原 timed_tests 中各新增 `test_bounded_reload_external_growth_and_pagination_boundary`：模拟另一进程写入 1,024 块，现有订阅者 reload 后看到正确末事件和更新，磁盘收敛到小窗口，分页返回窗口边界错误而非伪造历史结束。原损坏链恢复、迟到密钥、加密信息刷新、取消/弱引用及最新预览回归继续纳入全集。

## 验收与可复核记录

所有本地 target、Cargo、临时文件及日志位于 D 盘，并用 cargo metadata 验证 target。SDK 的两个完整 library 验收命令均不带 `--skip`、测试名过滤或 `--ignored`：

```text
cargo test --locked -p matrix-sdk --lib --features testing,bundled-sqlite
cargo test --locked -p matrix-sdk --lib --features testing,bundled-sqlite,hagency-bounded-history
cargo clippy --locked --no-deps -p matrix-sdk --lib --features testing,bundled-sqlite,hagency-bounded-history -- -D warnings
```

本机预算关闭全集 631 项、预算开启全集 645 项；12 项为本轮新增预算回归，另 2 项是此前已有的预算特性回归。共同基础库另有 162 项未过滤测试通过；无默认特性的预算库检查通过。11 个原函数正文与基线逐字比对相同，记录 `sdk15-preserved-original-assertions.json`；15 项在两策略的逐项结果为 `sdk15-case-results.json`。最终原始结果见 `D:/BuildCache/hagency-client/sdk15-original-final.log`、`sdk15-bounded-final.log`、`sdk15-clippy.log`。这里的全集指上述配置的 **matrix-sdk library**，不冒称整个 SDK 仓库所有 crates、集成测试、平台和特性组合都已经穷尽。

SDK fork 新增独立 workflow `Hagency SDK history regression`，只在独立功能分支 push 或手动触发，在 Ubuntu、Windows 上分别执行两个策略的未过滤 library 全集及严格库 Clippy；保留原上游 workflow。客户端继续检查最终固定版本的应用/history 测试、严格 lint 和构建配置。最终提交、各 workflow 的真实 run URL/结论、fork/main 未改核对保存在 `D:/BuildCache/hagency-client/sdk15-final-verification.json`；以准确提交号核验，不拿旧 main 的绿灯代替。

上轮 30 分钟原生 UI 和千万条实存索引报告保留为当时版本的历史证据，不作为本次新 SDK 版本已经重跑长测的证明。本轮针对改变的持久缓存和重载路径增加超限及多进程回归，客户端集成结果另见最终验证记录。


## 最终客户端集成复核

固定最终 SDK `28a91dc5f519a1d5f0f828df532faf5e88947da7` 后，本地应用 556 passed / 0 failed / 2 既有 ignored，history 8 / 0 / 2；应用/history all-targets 严格 Clippy、无默认 agent_chat、Rust 1.99 fast `-D warnings` 均通过。日志 `sdk15-client-{tests,clippy,no-default,rust199}.log`。样式、主题、i18n 和 workspace layout 审计通过。

新固定版本构建的历史验证程序另行重开既有 10,000,000 条加密索引库，原分页/顺序/搜索以及新增后撤回断言全部通过，Debug 全程 10.15 秒。日志 `sdk15-ten-million-reopen.log`，可执行程序及 SHA256 见 `sdk15-history-binary.json`。这是显式调用独立的忽略压测用例，不属于 SDK library 全集；未重新生成整座数据库，未清空 OS 文件缓存，不能当作冷读性能对比。
