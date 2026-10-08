# Hagency 实体 ID 改为 ULID

2026-10-08。实施仓库：`chrislearn/hagency-server`；客户端与 Desktop 对实体 ID 使用不透明字符串，已核对无需修改生成或解析代码。

后续[执行设备模型](2026-10-08-agent-execution-device.zh-CN.md)取消独立执行实例，因此不再分配 `ins_` 实体。下表及实测记录中的实例 ID 是当时版本的历史证据；其他实体的 ULID 规则保持有效。

## 生成与格式

新增 `entity_id()`，使用锁文件已包含的 `ulid 3.0.0`，生成26字符小写 Crockford Base32 ULID；保留类型前缀。ID 包含48位毫秒时间戳与80位随机部分。进程共用有互斥保护的单调生成器，同一毫秒或进程运行期间时钟回拨时继续递增。生成器溢出或锁中毒返回503，不降级到重复 ID 或猜测时间。

| 实体 | 前缀 |
| --- | --- |
| Hagency 用户记录 | usr_ |
| 用户会话记录 | ses_ |
| Hagency 执行设备 | dev_ |
| Project | prj_ |
| Agent 永久身份 | agt_ |
| Agent 与 Room 的 Binding（含主人 DM） | bnd_ |
| 执行实例 | ins_ |
| 服务端 dispatch 记录 | evt_ |
| 服务端回复 outbox 记录 | rep_ |

带这些四字符前缀的新实体 ID 总长30字符，原为68字符。Agent 的 Matrix 账号由 appservice namespace、Agent ID 和 server_name 组成，默认 `@_hagency_agt_<ulid>:<server>`。采用小写满足新 Matrix 账号 localpart 与当前 appservice exclusive regex，namespace 策略不变。

## 身份与凭据边界

原 `token()` 拆分为 `entity_id()` 和 `secret_token()`。后者仍产生独立32字节随机数并编码成64字符十六进制，用于 session/device bearer、回复 worker claim 及现有随机探测键。对外 DTO 的 `token` 字段名、认证校验与 token 哈希规则均不变。ULID 不作为秘密、授权证明或租约。

本次更新改动分配新实体 ID 的代码；数据库仍为 text 主键、原外键、唯一性约束与不可变身份触发器。原 Agent/主人/设备/执行实例、已开始执行、消息事务及账本记录继续使用原标识，不批量重写，也不创建替代傀儡。Agent 创建幂等请求、同主人同安装标识的设备重登记、执行实例更新继续返回原 ID。

Matrix 登录 device_id、Room/Event ID 由原 Matrix/Pasion 流程维护。原客户端安装标识、执行请求 `exec_` 标识、SHA-256 摘要、工作目录账号隔离哈希和确定性 Matrix transaction ID 保留生成规则，它们不是本次服务端实体分配的范围。

## 时间序的使用限制

进程内保证生成序；跨进程、重启与机器时钟偏差不承诺全局严格顺序。ID 分配顺序不代表数据库提交顺序、消息处理完成顺序或权限变化顺序。现有业务时间、epoch/generation、审计和事件 created_at 检查仍负责对应语义，不能用 ULID 时间戳替代。客户端不从 ID 推断身份、预算或权限。

## 验证与复审

- 独立 PostgreSQL 测试48/48通过，包括新 lowercase ULID、全局 Agent 与傀儡账号对应、各实体前缀、创建幂等重试、设备稳定 ID/凭据轮换、跨设备拒绝、dispatch/reply、不可变事务与未知结果恢复。测试使用一次性数据库。
- 严格 agent-service lib/tests Clippy 通过；OpenAPI 51+4操作及8个验证器回归通过，审核 token helper 改名后更新源码指纹。API DTO、路径、字段类型与数据库 schema 不变。
- 已核对 client/Desktop 没有原实体 ID 的64位后缀解析假设。保留的64位检查对应摘要、授权凭据或隔离哈希。
- 实际嵌入式 Palpo/Pasion/Appservice 集成通过：一次性三个数据库中的全新 ULID 傀儡完成真实创建、入群、mention 收取、ACK/start、幂等回复和主人 DM；设备和执行实例均核对为短 ULID，bearer 仍64字符。此集成只用固定 fixture 回复，不调用 AI 模型。

## 本地部署验收

新镜像 `hagency-server:ulid-20261008` 已更新 `hagency-local-https-server-1`；镜像源码指纹与构建清单一致，可信 HTTPS `/readyz` 返回200。更新前确认没有运行中的请求，通过 Desktop 停止 testgeny 的两个 Room，再重建服务容器。随后恢复 Project Room 和主人 DM，原设备与执行实例不变，lease epoch 6 已生效。

更新时现有9条 completed/sent 均保留且都有 Matrix event ID，没有额外发测试消息或清理账本。已有 Agent 和设备会继续显示原较长 ID；新建实体才使用新格式。没有新增 schema、旧格式导入或标识转换代码。

参考：[ULID 规范](https://github.com/ulid/spec)、[Matrix 账号标识](https://spec.matrix.org/latest/appendices/#user-identifiers)。
