# Rinx 内置应用

[English](README.md) | 简体中文

根目录 `system-apps.json` 中列出的应用随独立版和 OctoSense 托管版 Rinx 一同发布。
目录编译进 Rinx，本地导入包不能修改它。

- `article-editor/bundle/` 保存 App Hub 标准清单及目录资源。
- `article-editor/native/` 保存原生 Makepad 编辑器和发布流程。
- 共享文档、渲染代码保留在 `crates/article-core` 和 `crates/article-makepad`。
- Matrix 和 Octos 宿主适配器位于 `src/host/`；授权仍绑定账号和应用实例。

新增脚本应用时，创建 `apps/<name>/bundle/manifest.json`、`main.splash` 及资源，
并在目录中添加 `{"directory":"<name>"}`。清单必须声明准确的服务权限。
应用不收集 Matrix 凭据或 AI 提供商密钥。原生应用还需要编译进宿主的注册项，
只修改 JSON 不能引入原生代码。移动代码时保留已有应用 ID。

在仓库根目录运行：

```sh
bash tools/package-system-apps/check.sh
cargo test --locked --manifest-path crates/system-apps/Cargo.toml
```

检查会验证清单并计算摘要，不改写源码清单。正常构建会自动嵌入已验证的内容。
应用代码、权限和测试在同一个 Rinx PR 中审查，内置应用随 Rinx 发布；外部应用继续
使用 App Hub 包格式和发布流程。参见 [ADR 0008](../docs/adr/0008-rinx-system-app-catalog.md)。
