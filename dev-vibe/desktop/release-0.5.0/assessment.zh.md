# Hagency Desktop 0.5.0 发布检查

检查日期：2026-10-16。当前结论：核心功能已实现，正式发布暂缓；未创建 `v0.5.0` 标签或公开 Release。

## 已完成的修改与验证

- 主人引擎改为 `https://github.com/chrislearn/hagency-client.git` 的 `main` 分支，锁定 `39e990c8d483d6bd29b4135742f486e7b83ad137`。不再需要同级源码或 CI checkout/move action。
- 版本已准备为 0.5.0；发布流程支持七个安装包、合并校验文件和显式 Latest。发布说明已列出 Agent 加密 Room、签名、移动端和安装后启动验证的限制。
- macOS 首轮出现 `No matching IconType`，原因是新增普通 1024px PNG 被转为不支持的 1x ICNS 类型。现明确使用已生成的 `resources/icon.icns`；增加 `tools/check-release-icons.py`，在昂贵编译之前验证真实图标输入。
- 图标修复提交 `3233da33` 的主 CI、五平台独立构建均通过。首轮发布前测试在五平台通过；首轮仅 Linux 两架构完成安装包，macOS ARM64 打包失败。修复版打包因后续安全审计取消，不能称七个安装包已经全部通过。
- Cloud 中英文 Agent 指南、下载页、侧栏、快速入门、README 和页脚已完成；7 项官网测试和 8 次实际页面浏览器验收通过，390px 页面无横向溢出。Cloud 提交 `6955ad3` 的双架构镜像发布 8855 成功，生产部署待本轮确认。

## Desktop 主锁文件的安全补丁

仅更新兼容版本的依赖及必要构建依赖，没有修改客户端 API 或添加审计豁免：

| 依赖 | 原版本 | 更新版本 |
| --- | --- | --- |
| rustls | 0.23.37 | 0.23.46 |
| rustls-webpki | 0.103.6 | 0.103.15 |
| aws-lc-rs / aws-lc-sys | 1.14.1 / 0.32.0 | 1.18.2 / 0.46.0 |
| quinn-proto | 0.11.13 | 0.11.18 |

更新后 Windows `cargo check --locked --lib` 通过，`cargo test --locked --features agent_chat --lib` 456 项通过、2 项按原配置忽略。主锁文件漏洞记录由 24 条降为 5 条，另有 21 条维护状态、unsound 或 yanked 等警告记录。不能把先前提交的远端 CI 结果算作安全补丁的五平台验证。

## 尚未满足的发布条件

主锁文件剩余记录见 [desktop-after.json](desktop-after.json)：

| 依赖 | 记录 | 当前证据 |
| --- | --- | --- |
| idna 0.3.0 | RUSTSEC-2024-0421 | 来自 mdurl；Hagency formatter 对 mdurl 只调用 percent encode/decode，直接 punycode 使用另一版本 idna。未添加全局豁免。 |
| lopdf 0.36.0 | RUSTSEC-2026-0187 | Octos 通过 pdf-extract 处理 PDF。代码存在从文件字节调用 `extract_text_from_mem` 的实际路径，需要升级或修补。 |
| quick-xml 0.37.5 | RUSTSEC-2026-0194、0195 | Octos RSS/office 解析使用 Reader 和属性迭代；重复属性检查的二次耗时需要处理。未发现 NsReader 调用，不能把 0195 直接认定为已证实可利用。 |
| rsa 0.9.10 | RUSTSEC-2023-0071 | 来自非默认 TSP 功能；未出现在默认功能的依赖树中。 |

`lopdf` 的深度嵌套 PDF 栈溢出由上游 [RUSTSEC-2026-0187](https://rustsec.org/advisories/RUSTSEC-2026-0187.html) 确认，修复版本为 0.42.0 起。`quick-xml` 的 namespace 分配问题有明确的 [NsReader 触发条件](https://rustsec.org/advisories/RUSTSEC-2026-0195.html)；不能只按包名推断所有调用均受影响。

另外，安装包还包含独立编译的 Octos：`packaging/octos.lock.json` 固定源码 `fe08d8e6b3b32e672b0f956a2b692c3c8205b167`，通过其自身 Cargo.lock 和 `--locked` 构建。其完整上游锁文件有 36 条漏洞记录及 14 条其他警告，包含 TLS、证书校验、PDF/XML 等；**Desktop 主锁文件的更新不会更新这份独立锁文件**。记录不等于已证明 36 条路径全部存在于发行功能中，但正式发布前必须对实际构建功能完成升级或有依据的逐项分流。

下一步需要升级/修补上游 Octos 的依赖约束和独立发行锁文件，核对 Hagency formatter 的间接依赖及非默认 TSP 范围，再对最终同一提交重跑构建、测试和七安装包发布。不能仅将版本号改为 0.5.0 后宣称已经齐备。

## 可复核证据

- 审计数据库：RustSec `edb58f5974b83eb8302332821cb6d389777aa351`，1329 条记录，数据库更新时间 2026-10-14。
- [修改前主锁文件](desktop-before.json)、[补丁后主锁文件](desktop-after.json)、[独立 Octos 锁文件](bundled-octos.json)。这些是完整锁文件匹配结果，不是可利用性数量。
- 修复提交 [主 CI](https://github.com/hagency-org/hagency-desktop/actions/runs/38837925139) 和 [五平台构建](https://github.com/hagency-org/hagency-desktop/actions/runs/38837925226) 全部通过。
- 发布运行 [38832134391](https://github.com/hagency-org/hagency-desktop/actions/runs/38832134391)、[38837925791](https://github.com/hagency-org/hagency-desktop/actions/runs/38837925791) 已取消。正式下载入口仍是 [现有 GitHub Releases](https://github.com/hagency-org/hagency-desktop/releases)。

## 本轮最终验证

安全补丁提交 `7ad1b8dca67222bd8312952799b3dc397e12062a` 的 [主 CI 38842267586](https://github.com/hagency-org/hagency-desktop/actions/runs/38842267586) 和 [五平台构建 38842267688](https://github.com/hagency-org/hagency-desktop/actions/runs/38842267688) 均成功。远端 macOS 原生库 461 项通过、2 忽略；本地 Windows 456 项通过、2 忽略。翻译缺失0、主题与真实图标校验通过。机器审计快照附有被审计锁文件的源码提交和SHA-256；Octos锁与固定提交逐字节一致。

未启动安全补丁版本的正式安装包发布；仍没有v0.5.0标签或公开Release。是否扩大范围升级上游以满足发布条件，以及Cloud生产上线，分别等待用户选择。
