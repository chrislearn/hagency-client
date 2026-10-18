# Client 和 Desktop 统一工作区

更新：2026-10-18。

- 用户明确要求实际合入 Desktop、重新组织目录、所有 crates 放到 crates/，并为原 Desktop 仓库写迁移原因后归档。此前“只考察”范围已被本次指令覆盖。
- 当前工作分支 chris/consolidate-desktop-crates。Desktop 52d88b20 历史完整导入，导入 merge e9f5b4a4。
- 28 个 package 全部在根 crates/；桌面 crate 为 crates/hagency-desktop，package/bin 暂保留 rinx，默认 workspace 成员为 Desktop，hagency path 指向 ../hagency。
- 引擎原 native/ 路径改为 crates/；工具 tools/native，公共测试素材 tests/fixtures，Desktop 协作记录 dev-vibe/desktop。统一根 workspace、锁文件、patches/profiles/.cargo 与 CI；移除子工作区和子锁。
- Windows 构建必须使用 D:/BuildCache/hagency-client 的 target/cargo/tmp/npm，tools/dev.ps1 设置并核对；WSL 先核对 /mnt/d 挂载。Docker Desktop 当前磁盘在 C，不能据 Cargo target 在 D 就运行本机 Docker 构建。本次未使用 Docker。
- 本次保留 Web 控制台，不发行安装包；历史下载继续指向旧 Desktop Releases。原 0.5.0 安全审计和随包 Octos 独立锁问题保留。
- Desktop 原仓库当前 gh viewerPermission=WRITE，是否允许 archive 需执行并核实，不能提前称已归档。新目标 Client 为 ADMIN。
- 详细实现与验证：../client-desktop-consolidation-2026-10-18.zh.md；工作区指南 docs/workspace.md。
- 本地 Desktop check 与 456 单测通过/2忽略；12 个共享/引擎包共288通过/41忽略，32 Node 与8打包回归通过。独立crate选择发现rusqlite原先隐赖MatrixSDK默认feature，现显式启用fallible_uint。
- 原引擎Windows全量并非通过：ops::stop_clean仅Unix、模型测试写死/tmp/task及其他平台执行探测失败；未因合仓削弱检查或记为通过。原文件已比对一致；支持平台回归在GitHub运行39127796689和39127799243，PR https://github.com/chrislearn/hagency-client/pull/2。
