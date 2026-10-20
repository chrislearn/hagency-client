# Windows 自动更新

2026-10-20：用户授权在新分支实现 Windows 自动检查、后台下载、点击重启安装和 Agent 忙时延后，并创建 PR。实际工作在 hagency-client，分支 chris/windows-auto-update，从 origin/main 创建。

新增 hagency-updater 安全下载与暂存库、应用 updates 原生 UI/worker/独立安装辅助进程、process 任务与安装互斥 gate。Agent 实际执行从准备到结果交付持有 task guard，native 命令、启动恢复、语音和 owner 创建也接入；空闲轮询不阻止更新。等待阶段可取消，安装阶段阻止新任务，先保存并加入 owner，再等待 GUI 进程退出及正常 Shutdown 完成标记。

发布沿用 cargo-packager 0.10.1/NSIS，清单与包均使用其 Minisign 格式签名，Client 内置公钥；streaming 校验、大小限制、版本固定 URL、禁降级/预发布、安装前复验、持有文件锁。登录页和 About 均可操作；仅官方 Windows 独立打包启用自动网络检查，开发/未配置 key/嵌入不启用。没有上传生产密钥或发布版本。

正式发布需 vars.HAGENCY_UPDATER_PUBLIC_KEY、secrets.CARGO_PACKAGER_SIGN_PRIVATE_KEY 和可选密码 CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD。详细配置、边界和验证入口见 ../windows-auto-update-2026-10-20.zh.md。测试与 PR 最终状态后续追加。

本地最终验证：应用库 507 passed/0 failed/2 原 ignored，更新库 8 passed，任务/安装互斥 2 passed，owned_runtime 51 passed，native 接口 5 passed。应用+更新库 all-targets 严格 Clippy 与引擎生产 lib 严格 Clippy 通过；Windows 引擎全 all-targets 严格 Clippy 仍被既有 Unix-only 测试辅助函数 unused/dead_code 阻断，未增加 allow、删除断言或跳过测试。完整跨平台约束由 PR CI 继续验证。

真实 NSIS 夹具 install/cancel/tamper/busy 四场景全部通过：原父进程未退出不安装，正常提交后在含空格路径安装并启动新探针，缺少正常退出提交、包损坏或进程忙碌均不修改安装。结果在 D:/BuildCache/hagency-client/updater-handoff/ed83cb0c-26d9-46db-a597-22f6aeb77df1/results.json；busy 场景抽取生产模板的真实宏，用确定性结果替代 FindProcess。未测试真实 UAC 交互或正式安装包覆盖用户旧版。

发布复核发现 cargo-packager 0.10.1 默认 NSIS 的 /P 模式会 KillProcess 所有同名程序。现固定其模板到 packaging/windows/installer.nsi，改为发现实例就 SetErrorLevel 2 / Abort，安装和卸载均不再强制停进程；metadata 指向此模板。模板保留上游许可与来源，修正注释拼写；typos 仅添加合法 NSIS 指令 StrCpy。其他配置目录中的客户端需要退出后再重试，不会强制终止其 Agent。

computer-use 在独立 D 盘目录确认中文登录页“软件更新”入口、已下载状态、两个本机开关和安装按钮显示正常；正常 Alt+F4 退出后测试 GUI 二进制摘要保持不变，未产生重启标记，证明普通退出不自动安装。未通过 UI 点击执行安装；实际安装由上述隔离 NSIS 夹具执行。测试进程已关闭，未登录或操作真实账号。全部编译/缓存/夹具在 D:/BuildCache/hagency-client，cargo metadata 已核对。

其他检查：2036 翻译项/1525 调用/0 缺失，248 源文件样式零违规，主题、29 包工作区、5 项打包资产测试、33 项 Node 检查器测试、生产调用图、actionlint、typos 和 diff 检查通过。GitHub 只查询了仓库级 variable 名称，列表未显示更新公钥；未检查组织级变量，没有读取 secret 值或配置生产签名。
