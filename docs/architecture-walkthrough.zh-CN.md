[English](architecture-walkthrough.md) | [中文](architecture-walkthrough.zh-CN.md)

# 桌面架构导读

原生桌面应用是产品入口，原 CLI 与 Web 控制台已移除。旧 Fleet/Engagement 导读可从
Git 历史查阅。当前操作见[桌面使用指南](../crates/hagency-app/docs/hagency-quickstart.md)。

## 登录和账号身份

桌面登录位于 `crates/hagency-app/src/login/`，通过 Matrix、Pasion/OAuth 建立账号身份。
`crates/hagency-app/src/hagency/backend.rs` 将当前账号和凭据来源交给
`hagency::native_owner::open_with_matrix`。

引擎绑定当前 Matrix 账号并核对服务器身份。切换账号会撤销旧账号的本地授权。
OAuth 跳转仍由随机本机端口上的登录回调处理；这个监听器不提供管理页面或管理 API。
详见[服务器登录](server-login.zh-CN.md)。

## 桌面管理与 Agent 执行

`crates/hagency/src/native_owner.rs` 导出共用入口，底层 `console/native.rs` 将授权后的
请求交给进程内 Salvo 服务。`owner_host.rs` 持有私密状态目录和进程独占锁。
启动过程无需静态页面，也不启动本地 Web 管理服务器。

共用的 `console` 模块继续承担 Project、Agent 生命周期、模型配置、执行审批和预算逻辑。
执行、运行时、进程托管及 Matrix 库的权限检查保持原有边界。

## 代码与构建入口

代码目录见[工作区地图](workspace.md)和[引擎地图](native-engine.md)。Windows 使用
`tools/dev.ps1`，将产物、Cargo 缓存和临时文件放在 `D:/BuildCache/hagency-client`，
并核对 Cargo metadata。桌面打包及发布由 `app-*` GitHub 工作流负责。

历史设计、兼容向量和归属记录保留原始来源，不作为当前安装或浏览器控制台操作说明。
