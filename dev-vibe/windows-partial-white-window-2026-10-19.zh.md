# Windows 局部白屏：GPU 完成回执未推进

## 现场

用户反馈未主动更改设置，标题栏、左侧栏与右侧栏消失为白色，中央部分消息和输入区仍可见。
PID 68504 仍响应，后台帧提交持续约 144 Hz，没有 panic、D3D11 device lost 或 Map/CreateBuffer 错误。
切前台不恢复；触发缩放重绘后变为全白，Ctrl+0 恢复原缩放也不能恢复显示。
正常 Alt+F4 可保存窗口/应用状态并退出。保留了账号数据，未清库或重新授权。

## 根因与证据

当前 Makepad 固定提交 `1f3b1dedfbb81424eb8dbf69e5e2c634fa73dc54`。
`platform/src/os/windows/d3d11.rs` 每个窗口 pass 提交 serial，替换 instance buffer
需要先从 RetainedAllocationBudget 预留空间；旧 charge 仅在 GPU completed serial
推进后才能回收。D3D11 绘制循环本身不调用 `poll_texture_lifetimes`。
`Cx::frame_completion_serial` 才会非阻塞查询/创建完成回执，普通 Desktop 路径未调用它；
只有部分纹理释放等特殊操作会顺带轮询。因而普通重绘可长期累计旧 charge，预算满后
`reserve` 返回 None，render_view 跳过绘制并请求下一次 repaint，形成持续空绘制。

新增完全离线的 `render_residency` 原生示例，不读取账号或联网，4 MiB 预算加速重绘：

- 不启用维护：tick 660，submitted 781、completed **0**、bytes 4,194,288，refusals 7,905；
  实际窗口只剩底色，标题和正文消失。
- 直接轮询的初步对照：tick 300，submitted 607、completed 605，bytes 339,096、refusals **0**；
  实际标题和正文持续显示。随后改为直接复用正式 RenderMaintenance 进行最终验证。

实际故障实例没有可供读取的运行时 allocation 计数，不能声称已从原进程直接测得预算耗尽；
以上是源码因果链与同一渲染器的可重复对照验证。

## 修改

- `src/render_maintenance.rs`：仅 Windows，在 Draw 前非阻塞轮询完成回执；
  250 ms timer 负责空闲时最后一帧的完成推进，Shutdown 停 timer。
  不主动 redraw、不调整预算、不清数据、不修改 Makepad Cargo 缓存。
- App::handle_event 在处理主题/UI 之前运行维护，standalone/embedded 共用。
- 示例：`tools/dev.ps1 run --locked -p rinx --example render_residency` 复现旧路径；
  加 `-- --poll` 使用正式维护。两者均在真实 D3D11 窗口中检查。

## 验证

- 正式 RenderMaintenance 的离线 D3D11 压力测试：tick 3420，submitted 7167、completed 7163，
  bytes 339,320、records 24、refusals **0**；57 个定期样本均 completed>0、占用<1 MiB、零拒绝。
  同预算旧路径窗口已空白，正式维护路径实际窗口标题/正文持续可见。
- `cargo build --locked -p rinx --bin rinx` 及复现示例构建成功；完整 lib 测试
  **461 passed / 0 failed / 2 原有 ignored**，日志 `white-screen-suite.log`。
- 样式扫描 235 个源文件零违规；git diff --check 通过。
- 正式实例 PID 58692，`white-screen-fixed.{stdout,stderr}.log` 自动 LoginSuccess，
  标题、左栏、原 renewal 聊天、右侧详情完整恢复。截图中的原草稿按原文恢复在输入框，未发送。
  实际 Ctrl+plus 放大、Ctrl+0 恢复均正常完整重绘、保留草稿；主题 Light、侧栏展开、约180px输入区保持。
- 原故障实例日志 `composer-restored-retry.{stdout,stderr}.log` 保留，
  对照日志 `residency-before-4m.stdout.log` 与 `residency-fixed.stdout.log`。

所有编译产物、缓存和日志位于 `D:/BuildCache/hagency-client`，tools/dev.ps1
已执行 cargo metadata 验证实际 target。未修改 schema.rs、未提交或推送。

## 后续：快速闪动，上一轮修复不完整

用户随后反馈同一 PID 58692 快速闪动、中央聊天区域空白，说明前述简单场景的
4800 次重绘不能代表复杂聊天的回收压力。已更正验收范围，不将首次修复视为最终完成。

进一步发现：上游 `RetainedAllocationBudget::collect_for_frame` 每帧只调用一次
`collect`，每次最多访问 64 条记录。D3D11 普通 draw call 的每次脏实例上传都会新建 charge，
复杂窗口每帧替换超过 64 个缓冲区时，即使 completed serial 正常推进，旧记录仍持续积压；
预算边界上仅部分 draw 得到分配，形成绘制缺失/闪动。

离线示例扩展为 256 个独立 draw list 的格子，避免同 shader 自动批处理把压力合并掉：

- `--poll-only` 对照上一轮修复：tick1080，submitted2219/completed2215，bytes4194276，
  records33274、refusals405565；原生截图中标题/正文消失、格子成片缺失。
- `--poll` 使用新维护：最终tick3180，submitted6695/completed6691，bytes78136，
  records551、refusals0；中途32个连续样本均占用<1MiB且零拒绝，原生截图标题、正文与
  全部256格完整显示。压力窗口已正常关闭，仅保留正式主程序。

RenderMaintenance 在轮询完成回执后按库存数量执行有上限的额外回收（最多1024条访问/事件）。
仍遵守 live owner 和 GPU completed 条件，不放宽内存预算、不提前释放资源。
新增两个回归验证256批次能持续回收，及未完成/仍有活跃owner的分配不被提前回收。
`HAGENCY_RENDER_TRACE=1` 可开启每10秒仅含serial/字节/记录数/拒绝数的诊断。

同时发现 App::sync_titlebar_sidebar 每个事件都重应用图标样式/选中动画。
现在缓存实际visible/collapsed/主题revision/语言，仅变化时应用；LiveEdit、ScriptReapply、
热重载时显式失效，保留侧栏/主题/语言变化的正确刷新。

完整lib回归 **463通过/0失败/2原有忽略**；Windows主程序及复杂压力示例构建成功，
样式扫描238源文件0违规、diff检查通过。正式实例PID44620自动恢复登录，原聊天/左右栏完整，
草稿按原文恢复且未发送；诊断日志 `flicker-fixed.stdout.log` 多个连续样本占用约0.2MiB、
refusals0。压力对照日志 `flicker-before-complex-final.stdout.log` /
`flicker-after-complex.stdout.log`，全部位于D:/BuildCache/hagency-client。

## 独立提交范围

本次渲染修复提交仅包含 RenderMaintenance、App 的维护调用、模块注册、离线压力示例及此报告。
App/lib 使用按差异暂存，既有界面和语音开发仍留在工作区。标题栏缓存依赖尚未提交的标题栏
改版，与该改版一并保留，未纳入本次独立提交；上述463项全量及原生验收来自当时完整工作区。
另将本次暂存快照导出到D盘，单独编译运行渲染回归，确认提交不依赖其他未提交代码。
独立快照编译成功，两项渲染回归均通过（458项未选中）；日志为
`D:/BuildCache/hagency-client/render-commit-review-tests.log`。
