# Client CI 失败修复

用户反馈 client CI 发布失败；此前“先不发布上线”的要求继续有效，仅验证构建/测试，不创建标签、Release 或部署 Cloud。

实际运行分开核对：

- `39130940057` 是按用户要求取消的安装包发布，不能计作新打包故障；之前 Ubuntu Python3.10 的 tomllib 兼容问题已在 c3fabb9d 修复。
- `39130936761` 的五个桌面构建全部成功，只有汇总任务失败。它没有checkout，但继承了全局 `crates/hagency-desktop` 工作目录；汇总改用 runner.temp，无需源码。
- `39130936684` 引擎CI有四个失败。升级/回滚 fixture 尚在读取、复制旧 native/，改为 crates/，并纳入根 .cargo/config.toml。临时源码和 N+1 产物采用 cargo metadata 的真实 target_directory，支持外部 D 盘缓存；保留真实离线 N+1 构建及状态恢复检查。
- Octos 的两项审批 fixture 把“观察到 flush”误当作“对端开始回应”的先后约束。接口明确允许写回执前返回事件；测试改为按序保留最多两项完成事件、恢复同一 PreparedApproval，继续核对单次答复及 allow/deny 结果，没有修改生产协议或审批策略。

校验：actionlint与fmt/diff通过；本地与远端回归结果完成后补记。WSL已确认/mnt/d挂载来自//localhost/D$，全部target、Cargo缓存和临时目录在D盘。Windows上现有Octos测试在open阶段因非POSIX cwd被拒绝，不能把它算作Linux审批回归；使用Linux验证原CI故障。
