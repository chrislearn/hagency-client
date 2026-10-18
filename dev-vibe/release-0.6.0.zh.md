# Hagency Client 0.6.0

最新指示：用户随后明确要求“client 发布版本”，恢复Client 0.6.0正式发布；Cloud部署仍暂停。复用已通过的39134041425五平台CI附件，校验七个安装包及总SHA256清单后发布；标签指向实际构建提交7fb76731700daef7460d59935ba8bd4f69238ec0。发布前main相较构建提交仅增加验证记录文档，未改源码。下文保留此前暂停及验证的历史。

正式发布已完成（2026-10-19 00:02:43 +08）：[Hagency Client v0.6.0](https://github.com/hagency-org/hagency-client/releases/tag/v0.6.0)，GitHub Release id 416549027。API核对draft=false、prerelease=false，且releases/latest指向本版本；refs/tags/v0.6.0直接指向实际构建7fb76731。

- 使用gh CLI提升原CI产物，未重新编译。先逐项验证七个安装包原始SHA256，再生成总SHA256SUMS.txt；上传草稿后与GitHub服务端digest逐项比较全部13个附件（七安装包、五分平台清单、一个总清单），名称、大小、状态与SHA256全部匹配后才公开发布。公开下载SHA256SUMS.txt再次核对一致，七个releases/latest/download安装包入口HEAD全部HTTP 200。
- 下载/暂存/验证脚本和完整JSON证据均在D:/BuildCache/hagency-client/releases/v0.6.0。签名、依赖审计、移动端及安装/启动验收限制保留在发布说明，没有声称完成这些验收。
- 标签自动触发重复构建39143210081，已确认completed/cancelled；本次发布的构建来源仍是成功的39134041425，不能把重复构建记作正式发布验收来源。Cloud没有部署。

- Desktop 与本地引擎 workspace 版本均设为 0.6.0；独立组件保持自己的 API 版本。
- 发布入口 `.github/workflows/desktop-release.yml`，支持匹配版本的 v* 标签或 main 手动 publish=true。五平台各自测试、构建；统一核验七安装包与 SHA256SUMS 后发布 Latest，拒绝覆盖已公开 Release。
- Windows 发布构建输出、Cargo 缓存及临时文件使用 D:/BuildCache/hagency-client；metadata 实际目录检查加入 CI。发行文件名保留 hagency-desktop-*，源仓库为 hagency-org/hagency-client。
- 本机 D 盘库检查通过；actionlint 检查通过。源码 c3fabb9d99a2d9e26b4dedd17eee5787e9711bae 的 Desktop CI 39130936653通过。首轮39130756689因Ubuntu Python3.10缺少tomllib失败并取消，改用兼容的版本提取后重跑39130940057。Linux x64通过测试进入打包，其余平台构建中时按用户新指示取消，不能称七包验收完成。
- 39130940057 最终已确认completed/cancelled，全部五平台均cancelled，下游发布没有执行。GitHub API复核没有v0.6.0标签和任何Client Release。用户明确恢复前不重跑正式发布。
- 没有扩大到 Octos 上游依赖改造；历史依赖评估仍有效，在发布说明中公开链接，不能声称审计干净。签名、移动端、加密 Room 的限制保留。

后续用户反馈CI失败：修复7fb76731已推送；主CI39134040705、五平台构建39134040825、Desktop CI39134040782全部success。主CI2049 passed/174原有ignored，升级/回滚和4项Octos审批全部通过。安装包39134041425仅以publish=false验证，不创建标签/Release。详情及最终打包结果见[CI修复记录](client-ci-failure-2026-10-18.zh.md)。正式发布/Cloud上线的暂停指示仍有效。

安装包39134041425最终success：五平台七种安装包与各平台SHA256文件全部生成并上传为CI附件，Publish complete release明确skipped。再次核对没有Client Release和v0.6.0标签。没有执行安装/启动或签名验收，也没有执行正式发布job中的合并校验步骤。
