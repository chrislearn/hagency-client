# Hagency 跨平台应用命名统一

2026-10-19。用户确定使用 `hagency-app`，要求更新目录、Cargo 包、可执行文件、构建和安装包，并测试、提交、推送 main。

| 项目 | 新名称 |
| --- | --- |
| 前端目录 | `crates/hagency-app` |
| Cargo 包 | `hagency-app` |
| Rust library | `hagency_app` |
| 图形应用可执行文件 | `hagency` / `hagency.exe` |
| 应用显示名称 | Hagency |
| 引擎 Cargo 包 / library | `hagency`，继续提供本地路径依赖 |
| 独立浏览器控制台 CLI | `hagency-cli` / `hagency-cli.exe` |
| 安装包 | `hagency-{platform}-{arch}.{extension}` |
| 原生应用 CI | `.github/workflows/app-{main,builds,release,licenses,aur-publish}.yml` |

CLI 原先已占用 `hagency`，所以一起改为 `hagency-cli`，避免两个 Cargo package 生成同名文件。CLI 命令语义不变。已安装的可选 CLI 用户服务需使用新命令重新安装，以更新 supervisor 保存的可执行文件路径。

## 构建与资源

- `tools/dev.ps1` 默认启动 `-p hagency-app --bin hagency`，仍把构建、缓存和临时目录定位到 `D:/BuildCache/hagency-client`，用 Cargo metadata 核对输出目录。
- 更新根 default-members、Cargo.lock、Rust 示例和测试导入、内置应用目录及文章测试的跨 crate 文件引用。锁文件经结构比较，只将主包 `rinx` 替换为 `hagency-app`，没有更新依赖版本。
- Makepad 资源发现使用 `hagency-app.path`，打包资源命名空间使用 `hagency_app`。`package-app.py` 按 metadata 的实际 target 目录收集 `hagency`。
- macOS launcher、签名脚本和 plist 使用 `Hagency.app`、`hagency`；Windows VERSIONINFO 显示 Hagency；Linux desktop entry 为 `hagency.desktop`。iOS 构建将 Rust package、bundle 目录和可执行文件分开处理；OpenHarmony 不再假设 crate 内 target。
- 发布资产归集抽为 `collect-release-assets.py`，先确认每种格式恰有一个安装包，再生成统一名字与 SHA256 文件。测试覆盖现有五个平台/架构组合，以及缺失和重复安装包拒绝。

## 保留的兼容标识

已有账户、加密密钥和偏好仍使用原 ProjectDirs 存储身份。新运行可设置 `HAGENCY_APP_DATA_DIR`；原 `HAGENCY_DESKTOP_DATA_DIR`、`RINX_DATA_DIR`、`ROBRIX_DATA_DIR` 保留为别名。

已发布的 bundle identifier、iOS org/app 签名身份、旧 URL scheme、宿主模块及 mini-app 协议 ID 不属于平台显示名称，保留以维持升级、权限和集成兼容。Android 的 Java 扩展包对齐已有 packager identifier `org.hagency.desktop`，避免生成的新项目无法按包名找到扩展。内部 `Rinx*` widget recipe、独立 `rinx-miniapp-*` 等子库不在本轮重命名范围。

历史来源、许可证、审核记录和旧 Release 链接不改写。已有 v0.6.0 附件是旧提交构建的 `hagency-desktop-*`，本轮 main 推送不重建、替换或重命名这些已发布二进制。下一次应用发布使用新名称，并需同步 Cloud 的稳定下载链接。

## 验证

- GUI lib：`test --locked -p hagency-app --features 'agent_chat,octosense-module' --lib`。
- 引擎 lib、CLI 与 supervisor 集成：`test --locked -p hagency --lib` 和 `--test cli --test service_linux --test service_launchd`。
- 跨 crate 资源：`test --locked -p article-core -p rinx-system-apps`。
- 宿主与全部目标：`check --locked -p hagency-app --no-default-features --features agent_chat` 和 `check --locked -p hagency-app --all-targets --features 'agent_chat,octosense-module'`。
- 工作区边界与可执行文件重名检查、样式审计、主题字面量、i18n、图标、打包路径、资产命名/校验和、YAML 和 shell 语法检查。
- Windows 主程序实际启动检查。其他 OS 的完整安装、签名和设备运行不由本机检查替代。

测试还修正了此前未覆盖到的夹具问题：辅助标题的主题断言跟随 muted 语义；嵌入模式无独立标题栏时不求值其图标；Windows 状态夹具通过正式代码创建私有子目录，Python 协议夹具通过显式解释器启动；CLI 测试明确无 Unix guardian 的平台行为，并为无 HOME 的服务参数检查提供隔离 state 路径。没有放宽生产权限检查或执行真实模型调用。

全部目标检查还补齐两个实验 HTML 比较示例的 `required-features = ["html_preview"]`，并为依赖 Unix 进程组清理的真实远端 kernel 夹具声明 Unix 条件。默认目标编译不代表已构建实验 HTML 预览或完成真实 kernel/其他 OS 的实机验证。
