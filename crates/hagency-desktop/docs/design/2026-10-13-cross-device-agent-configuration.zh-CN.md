# Desktop issue #2 实施方案

最终方案与协议由 sibling server 的 [执行设备环境发现与跨设备配置](../../../hagency-server/docs/design/2026-10-13-cross-device-agent-configuration.zh-CN.md) 定义。Desktop 负责目标设备说明、环境与表单、应用状态、账号/Agent/版本/请求 fence 和中英文文案；共享 native owner/client 负责执行设备上的实际发现、校验与持久化。

先保存和独立复审方案，修正阻塞项后多 agents 实施。server、desktop、配套 client 均使用 codex/issue-2-cross-device-config 新分支。代码开发不接管设备、不自动启用 Agent、不转移 provider 凭据。
