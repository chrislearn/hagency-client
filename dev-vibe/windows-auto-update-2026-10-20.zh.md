# Windows 客户端自动更新

首版在 Windows 独立安装版提供自动检查、后台全量下载和点击重启安装。登录页的“软件更新”和设置中的“关于 Hagency”使用同一界面；更新不依赖 Matrix 登录。默认开启自动检查和后台下载，启动延迟约 30 秒检查，随后按本机记录每日检查。关闭自动下载后可手动下载。

登录表单底部只保留“软件更新”入口，点击打开独立弹窗，不展开内容挤压登录表单。弹窗优先显示状态和下载/安装操作，其后是本机自动更新开关；版本说明默认折叠，展开后在限高区域滚动。设置的 About 页沿用同样的内容顺序和折叠说明。弹窗可通过关闭按钮、Escape 或点击遮罩关闭，关闭不会取消后台下载或已确认的等待安装。

更新下载完成后不会自行退出应用。点击“重启并安装”后，如果 Agent 正在处理请求、等待审批或完成结果交付，会保留现有任务，空闲后开始准备安装。等待期间可点击“稍后安装”。准备安装会禁止接纳新任务、保存工作、停止受管 owner，并等待实际 GUI 进程正常退出。普通关闭到托盘不会作为安装完成退出的依据。

受管 owner 关闭前，空闲 Agent 的轮询只等待安装互斥；如果保存或辅助进程准备失败，释放互斥后继续接收请求，无需重新启动服务。

## 实现与签名

`hagency-updater` 验证更新清单和 NSIS 安装包的 Minisign 签名、语义版本、稳定通道、平台、版本固定 URL、大小及 SHA256。下载流式写入临时文件，验证成功后才发布暂存指针；安装前及安装辅助进程中再次验签。Windows 读句柄在安装交接期间禁止覆盖或删除已验证的包。

安装辅助进程由当前可执行文件复制到更新目录，先于 GUI 单实例锁分流。它等待原 GUI 的进程句柄结束，并要求主程序在 Makepad 正常 Shutdown 返回后写入完成标记，再启动现有 NSIS 包。使用被测试的 `/P /D=安装目录` 参数；辅助进程以原用户身份重启新程序，避免 NSIS 以管理员身份启动应用。安装失败或取消后保留可重试入口，恢复旧程序的能力取决于安装器保留的文件，不承诺自动回滚数据。

第一版沿用 cargo-packager 0.10.1 的签名格式和安装包，独立封装流式下载、暂存和退出交接；没有直接使用会调用 process::exit 的安装 API。整个应用与资源、Octos 运行时一起更新。

Windows 使用随源码固定的 0.10.1 NSIS 模板，将上游被动安装时的强制结束同名进程改为中止安装。其他配置目录中的 Hagency 实例和 Agent 任务会继续运行；需要退出这些实例后再重试更新。安装与卸载均不会通过模板结束其他实例。

## 首次发布配置

需要管理员配置以下仓库项，PR 不上传或更改生产密钥：

| 类型 | 名称 | 内容 |
| --- | --- | --- |
| Repository variable | HAGENCY_UPDATER_PUBLIC_KEY | cargo packager 生成的 `.pub` 文件内容，完整的外层 Base64 |
| Repository secret | CARGO_PACKAGER_SIGN_PRIVATE_KEY | 对应私钥文件内容 |
| Repository secret | CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD | 私钥密码，可为空密码 |

密钥只生成一次，保存到受控 D 盘密钥目录并备份。可使用 `cargo packager signer generate --path D:/受控目录/hagency-updater.key`。私钥不得提交仓库。既有 `packaging/upload-release-secrets.sh` 和示例字段已改为 cargo-packager 0.10.1 实际读取的变量名。

构建时公钥嵌入二进制。正式 Windows 发布缺少签名配置会失败；未配置公钥的本地开发构建和嵌入模块不会启动自动更新。不要直接更换公钥，否则已有安装无法验证新密钥签发的更新，需要另行设计密钥轮换桥接。

Windows 打包后，流水线签名 `hagency-windows-amd64.exe`，生成并签名 `windows-update.json`，再用嵌入公钥的同一验证库核对清单和安装包。两个签名、清单和完整安装包随 GitHub Release 草稿一起上传，附件完整后才公开发布。预发布版本不发布稳定更新清单。

更新包签名不替代 Windows Authenticode，系统仍可能显示 SmartScreen 或权限提示。本次不申请系统签名证书，也不发布新版本。旧客户端没有更新模块，需要先手动安装一次含此功能的正式安装版。

## 验证入口

常规验证为 `cargo test --locked -p hagency-updater`、`cargo test --locked -p hagency --lib update_guard` 和应用库测试。Windows 的 `tools/test-windows-updater.ps1 -NsisPath <makensis.exe>` 生成临时密钥、真实 NSIS 安装器和重启探针，验证原进程退出前不安装、含空格路径安装后重启、未提交退出标记时取消、损坏包拒绝。夹具还抽取生产模板的进程检查宏，通过确定性的忙碌结果验证中止安装分支。该工具只写入 D:/BuildCache 下的独立夹具，不安装到用户真实应用目录，不写卸载注册表，不打开真实账号数据。

Windows App Build CI 运行签名与锁定测试、原生布局测试和真实 NSIS 交接夹具；其他平台继续编译并保持现有应用行为。CI 的 NSIS 测试工具下载固定 3.09 版本并核对 SHA256。
