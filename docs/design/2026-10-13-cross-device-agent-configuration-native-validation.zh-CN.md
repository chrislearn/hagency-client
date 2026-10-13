# Desktop issue #2：共享 Client native 验证

源目录：`/Volumes/Data/Works/chrislearn/hagency-client`；分支：`codex/issue-2-cross-device-config`。本记录单独保存，既有 `docs/progress.md`、`docs/agent-knowledge.md` 的操作者修改保持未提交。

实现遵循 Server 最终方案及协议附件。三个 token-free wrapper 使用当前授权设备调用固定配置 API，拒绝任意 nested Command；B 不调用自身 Provider 或本地持久化。目标 worker 与普通 native 命令共用 `calls` 控制锁及 `execute_locked`，独立三秒心跳不受该锁阻塞。Matrix owner 的 worker 只在初次授权完成且 cookie 已发布后启动；独立 `matrix_sync` 锁串行续期，修复启动时争用授权会话的实际失败。

应用前、目录/模型发现后的保存边界以及确认前重新核对原 DeviceIdentity、Agent generation/assignment 和原请求状态，包含发起设备撤销导致的 request 失效。保存模型重新验证目标当前 catalog 的 model/reference/effort 组合和目标本地工作目录；目标 Agent 自身存在活动服务或执行时拒绝其模型保存；其他 Agent 的运行不阻塞该保存。provider connect 属于主人级关联，仍在同主人任意 Agent 活动时拒绝，避免既有运行捕获旧模型或远端关联隐式停止其他 Agent。政策保存沿用既有 revision CAS。

私有原请求 journal 在本地效果前记录 Applying。重启不重放 Applying；已知 completion 仅重试相同原确认，确认返回需精确匹配原请求和结果才释放 custody。关闭先撤销原权限、停止配置 worker 并等待其 task 结束。原文件写入与 Server 撤销并非分布式原子事务；不确定/非法或过大应用结果保持 unknown，不重新执行。64 KiB input/result 保持不变；Work 同时携带 input/result，接收及 journal envelope 有独立有限容量。

远端响应逐字段投影；authUrl、credentialStore、账户信息、token 和原始 stdout/stderr 不转发。安装与版本仅来自目标可信 resolver 和有界两秒 `--version`；版本成功且 provider 契约探测成功才 versionSupported=true。Octos profile 不能证明登录验证，accountVerified=false。

实际执行验证：

- `cargo test -p hagency --lib console::native::configuration`：4 passed，0 failed，0 ignored。日志 `.run/issue-2-client-focused.log`。
- `cargo test -p hagency --lib console::owned_runtime::service::configuration_busy_tests`：1 passed，0 failed，0 ignored；证明模型冲突限定当前 Agent，provider 冲突仍覆盖同主人。日志 `.run/issue-2-client-agent-busy.log`。
- `cargo test -p hagency --lib`：216 passed，0 failed，5 existing ignored。日志 `.run/issue-2-client-lib.log`。
- `cargo clippy -p hagency --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `git diff --check` 全部通过。Clippy 日志 `.run/issue-2-client-clippy.log`。
- TCP 原生 fixture 经实际 MatrixTokenSource 授权和 NativeOwner dispatch 创建目标私有工作目录；服务器接收成功 complete 后丢失 HTTP 响应。A 重开后只发送完全相同 completion，claim poll 总数仍为 1；Applying 重开不应用或确认；旧 Server capability 明确拒绝。
- 同一真实 TCP fixture 持有 NativeOwner `calls` 锁七秒期间，观察至少两次独立 `acceptWork=false` 心跳。该验证说明控制锁不会阻塞心跳，不代替真实 provider 推理。
- 严格 allowlist、nested 登录/运行命令拒绝、秘密字段拒绝、Applying/complete 原 journal 重开保持均实际执行。

主 agent 另执行独立嵌入式 Server/两设备集成：`real_two_device_configuration` 1 passed、0 failed、0 ignored；B 独立子进程所有 provider executable 均缺失，仍发现 A 的显式 Claude fixture，保存/同键恢复/政策 CAS/两个 profile 重开/实际 owner-direct binding 下 Ledger resolver 均通过。该结果由主 agent 记录；fixture 仅状态及版本，无模型推理。

未执行真实跨物理设备、原生可视检查和真实 provider 推理，不将这些列为通过；已有 ignored 场景保留原 verdict。首次实际 Server smoke 的授权启动失败保留原失败证据，修复 worker 启动顺序后通过；首次 TCP fixture 的 native state 目录拒绝同样保留，改用专用新 `state` 子目录后实际重开通过。
