# Client CI 失败修复

用户反馈 client CI 发布失败；此前“先不发布上线”的要求继续有效，仅验证构建/测试，不创建标签、Release 或部署 Cloud。

实际运行分开核对：

- `39130940057` 是按用户要求取消的安装包发布，不能计作新打包故障；之前 Ubuntu Python3.10 的 tomllib 兼容问题已在 c3fabb9d 修复。
- `39130936761` 的五个桌面构建全部成功，只有汇总任务失败。它没有checkout，但继承了全局 `crates/hagency-desktop` 工作目录；汇总改用 runner.temp，无需源码。
- `39130936684` 引擎CI有四个失败。升级/回滚 fixture 尚在读取、复制旧 native/，改为 crates/，并纳入根 .cargo/config.toml。临时源码和 N+1 产物采用 cargo metadata 的真实 target_directory，支持外部 D 盘缓存；保留真实离线 N+1 构建及状态恢复检查。
- Octos 的两项审批 fixture 把“观察到 flush”误当作“对端开始回应”的先后约束。接口明确允许写回执前返回事件；测试改为按序保留最多两项完成事件、恢复同一 PreparedApproval，继续核对单次答复及 allow/deny 结果，没有修改生产协议或审批策略。

修复提交：7fb76731700daef7460d59935ba8bd4f69238ec0，已推送main。

已完成校验：

- actionlint与fmt/diff通过；Windows本地源码指纹/共享模板回归1项通过。
- [主CI 39134040705](https://github.com/hagency-org/hagency-client/actions/runs/39134040705)整轮success。212个Rust测试套件共2049 passed、0 failed、174按原配置ignored；原来的两个全局真实主机skip保留，没有新跳过。升级、回滚与4项Octos审批测试全部ok。
- [五平台构建39134040825](https://github.com/hagency-org/hagency-client/actions/runs/39134040825)整轮success，五平台和之前失败的汇总job全部success；移动端仍按原规则不在push事件运行。
- [Desktop CI 39134040782](https://github.com/hagency-org/hagency-client/actions/runs/39134040782)整轮success，原生库测试、构建、无默认功能检查、资源/翻译/布局检查均通过。
- [安装包验证39134041425](https://github.com/hagency-org/hagency-client/actions/runs/39134041425)整轮success，head为7fb76731，publish=false。五个平台全部success：Linux x64/ARM64各DEB+AppImage、macOS Intel/ARM64各DMG、Windows x64 NSIS，共七种安装包。各平台的文件数量检查、SHA256生成和附件上传全部通过，API已核对五份installers-*附件存在且未过期。
- Publish complete release明确skipped；正式发布时的合并SHA256SUMS核验及上传Release步骤本轮没有执行。最终API复核Client Release列表为空，远端没有v0.6.0标签；Cloud未部署。安装包构建通过不等同于实际安装/启动或签名验收。

WSL已确认/mnt/d挂载来自//localhost/D$，全部target、Cargo缓存和临时目录在D盘；本地WSL受9p上缓存文件mtime权限限制，未完成构建。Windows上现有Octos测试在open阶段因非POSIX cwd被拒绝，不能将这两次本地尝试记为审批回归通过。原CI故障以已成功的远端Linux全套结果验证。
