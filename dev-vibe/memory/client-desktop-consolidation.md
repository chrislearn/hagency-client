# Client 和 Desktop 统一工作区

更新：2026-10-19。

- 最新仓库地址为 https://github.com/hagency-org/hagency-client.git，用户已明确指定。origin 已更新；旧 chrislearn/hagency-client 由 GitHub 重定向到同一仓库，历史 PR/提交链接仍可解析。当前 canonical metadata、About 源码链接、OAuth 公开 client_uri 及匹配断言已更新，未改业务控制逻辑。
- GitHub fork parent 是 hagency-org/hagency-rs。比较 Client b6da949b 与上游1212833a：ahead3814/behind101，共同祖先e51a0b13；上游101=79非merge+22merge。合仓前39e990c8也behind101，故不是Desktop导入漏提交。这里只做差异分析，没有merge/cherry-pick/rebase上游。
- 79个非merge按主题：旧Coordinator/Fleet37、Claude18、Octos17、工作区安全与ADR194共2、末尾跨运行时修复5。当前已重整Claude/Octos桥接，4个Octos底层文件与上游最新逐字一致，不能把behind数当成功能缺失数。分析见../upstream-divergence-analysis-2026-10-18.zh.md。

- 用户明确要求实际合入 Desktop、重新组织目录、所有 crates 放到 crates/，并为原 Desktop 仓库写迁移原因后归档。此前“只考察”范围已被本次指令覆盖。
- 实施分支 chris/consolidate-desktop-crates 已合并，本地当前为 main。Desktop 52d88b20 历史完整导入，导入 merge e9f5b4a4。
- 28 个 package 全部在根 crates/；桌面 crate 为 crates/hagency-desktop，package/bin 暂保留 rinx，默认 workspace 成员为 Desktop，hagency path 指向 ../hagency。
- 引擎原 native/ 路径改为 crates/；工具 tools/native，公共测试素材 tests/fixtures，Desktop 协作记录 dev-vibe/desktop。统一根 workspace、锁文件、patches/profiles/.cargo 与 CI；移除子工作区和子锁。
- Windows 构建必须使用 D:/BuildCache/hagency-client 的 target/cargo/tmp/npm，tools/dev.ps1 设置并核对；WSL 先核对 /mnt/d 挂载。Docker Desktop 当前磁盘在 C，不能据 Cargo target 在 D 就运行本机 Docker 构建。本次未使用 Docker。
- 本次保留 Web 控制台，不发行安装包；历史下载继续指向旧 Desktop Releases。原 0.5.0 安全审计和随包 Octos 独立锁问题保留。
- Desktop 原仓库当前 gh viewerPermission=WRITE，是否允许 archive 需执行并核实，不能提前称已归档。新目标 Client 为 ADMIN。
- 详细实现与验证：../client-desktop-consolidation-2026-10-18.zh.md；工作区指南 docs/workspace.md。
- 本地 Desktop check 与 456 单测通过/2忽略；12 个共享/引擎包共288通过/41忽略，32 Node 与8打包回归通过。独立crate选择发现rusqlite原先隐赖MatrixSDK默认feature，现显式启用fallible_uint。
- 原引擎Windows全量并非通过：ops::stop_clean仅Unix、模型测试写死/tmp/task及其他平台执行探测失败；未因合仓削弱检查或记为通过。原文件已比对一致；支持平台回归在GitHub运行39127796689和39127799243，PR https://github.com/chrislearn/hagency-client/pull/2。
- PR #2 已合并至 Client main，merge 0622ce8aad384fc21efd98f1661552c5b96f979d，本地已同步。为让实际 main 自动CI获取runner，旧两次手动运行已取消；其fmt/clippy/测试清单/调用图及Owner浏览器已通过，不是全矩阵完成。
- 原 Desktop README 双语与 MIGRATION.md 在 cd616d95749af8560a90743c316da679de578077 推送，明确合仓原因、目标和历史Release保留。实际 archiveRepository 调用被GitHub拒绝，API verified archived=false / admin=false / maintain=false / push=true。必须由该仓库管理员完成归档，不需要再询问用户是否授权（用户已经明确授权）。
- 主分支自动检查39128551810/39128551856/39128551885；未正式发布安装包。Desktop便携article-core检查已通过，完整mainCI状态后续以运行结果为准。
- 0.6.0发布工作流已实现，详情../release-0.6.0.zh.md。用户随后明确“先不发布上线”；保持暂停标签、Release及Cloud部署。用户反馈CI失败不代表恢复正式发布。
- CI修复7fb76731已推送main：无checkout的Desktop汇总用runner.temp，升级fixture复制crates/和根.cargo/config.toml、遵守metadata实际target；Octos审批fixture处理写回执之前的合法完成事件，保持同一PreparedApproval和单次答复断言。没有跳过失败测试或修改生产协议。
- 修复后的主CI39134040705、五平台构建39134040825、Desktop CI39134040782全部success。主CI212个Rust套件2049 passed、0 failed、174原有ignored，真实升级/回滚和4项Octos审批全部通过。安装包39134041425仅publish=false；完整进展见../client-ci-failure-2026-10-18.zh.md。
- 安装包39134041425最终success：五平台全部成功，七种安装包及各平台SHA256上传为五份CI附件。Publish complete release明确skipped；没有Release、v0.6.0标签或Cloud部署。正式发布job中的合并校验与Release上传没有在本轮执行，不能将构建验收称作已发布或实际安装/启动验收。
- 用户随后明确“client 发布版本”，恢复Client 0.6.0正式发布授权；Cloud部署仍暂停。直接提升39134041425已验证的七种安装包，全部下载/校验/暂存放D:/BuildCache/hagency-client/releases/v0.6.0；标签必须对应实际构建7fb76731，而非后续仅更新文档的main。最终发布状态见../release-0.6.0.zh.md。
- 2026-10-19 00:02:43 +08，v0.6.0正式发布为Latest：https://github.com/hagency-org/hagency-client/releases/tag/v0.6.0。Release id416549027，七安装包及六校验清单共13附件的GitHub服务端SHA256全部与本地匹配，公开总清单下载核对一致，七个latest下载入口HEAD均200；标签实际指向7fb76731。标签自动触发重复构建39143210081已completed/cancelled，正式构建来源是成功的39134041425。保持Cloud不部署。根目录和Desktop双语README的当前下载入口更新到Client Releases，历史依赖评估链接保留。
