[English](README.md) | [中文](README.zh-CN.md)

# Hagency 使用指南

本指南面向通过 Palpo Matrix 服务器使用 Hagency 的人：出借 AI agent 的
Hagency 所有者，以及和 agent 一起工作的项目所有者与成员。内容包括：哪些事情
只需做一次、你会看到哪些房间、谁能和 agent 对话。

先说明几个常用词：

- **Hagency**：运行 AI agent 并把它们借给项目使用的服务。
- **Palpo**：项目所在的 Matrix 服务器（homeserver）。它有一个网页端
  “Palpo web”，用于管理账号、项目和 Hagency 接入。
- **Matrix 客户端**：你平时用的聊天软件，例如 Element 或 Rinx。
- **Hagency 控制台**：Hagency 所有者的网页，用来审批申请、管理 agent 和 token。

## 角色分工

| 角色 | 是谁 | 做什么 |
| --- | --- | --- |
| Matrix 服务器管理员 | Palpo 的管理员 | 每个 Hagency 只需一次：在 Palpo web 中 **Add Hagency**。 |
| Hagency 所有者 | 被指定为所有者的普通 Matrix 账号 | 下载配置、连接 Hagency、完成验证、审批 agent 申请。 |
| 项目所有者与成员 | 普通 Matrix 账号 | 创建项目、申请 agent、与 agent 对话。 |

为什么需要管理员？添加 Hagency 会安装一个 Matrix **App Service**（应用服务）：
即一段专属的账号名空间 `@hf_<fleet>_*`，Hagency 可以在其中创建账号并以这些
账号的身份行事——包括它的代表账号、审批机器人和各个 agent。这是服务器级别的
授权，只有管理员能给，任何 Matrix 服务器（包括 Synapse）都是这个规则。这一步
做完之后，就不再需要管理员权限了。

## 把 Hagency 连接到 Palpo 服务器

1. **管理员**：在 Palpo web 打开 **Add Hagency**，填写名称、所有者的 Matrix
   ID，连接方式选 **Hagency connects outbound to Palpo**（由 Hagency 主动
   连接 Palpo），然后点 **Authorize and install**。
2. **所有者**：在 Palpo web 打开 **My Hagency access**，点
   **Download Hagency configuration**，得到一个 JSON 文件。
3. **所有者**：在 Hagency 控制台进入 **项目方** → **连接 Palpo 项目服务器**。
   选择刚下载的 JSON 文件，填写 **Matrix 地址**（homeserver 的 URL，例如
   `https://matrix.example.org`），点 **连接**。无需重启。
4. **所有者**：回到 Palpo web，点 **Verify connection & create reception**。
   显示就绪后，这台服务器上的项目就可以申请 agent 了。

一个 Hagency 只连接一个 Palpo 集群。

登录 Palpo web 时请使用完整的 Matrix ID，例如 `@alice:example.org`。目前只写
短名（`alice`）会被拒绝。

## Matrix ID 与服务器名

Matrix ID 的格式是 `@名字:server_name`。`server_name` 在服务器首次搭建时确定，
之后永远不能更改，所以请一开始就用你真正的域名（例如 `example.org`），
matrix.org 也是这样做的。如果服务器跑在非标准端口上，可以在该域名下放一个
`.well-known/matrix/client` 文件，客户端据此找到服务器。

## 你会看到的房间

| 房间 | 是什么 | 你要做什么 |
| --- | --- | --- |
| 接待室（reception room） | Palpo 与 Hagency 之间的“信箱”。agent 申请和连接验证以特殊事件的形式送到这里。 | 什么都不用做。客户端里基本看不到内容。 |
| 项目房间 | 人和 agent 一起工作的地方。 | 用 @ 提及 agent 来找它办事。 |
| 审批室 | 项目专属的加密私密房间，只有你和审批机器人。 | agent 要做有风险的操作时，审批卡片会发到这里，由你批准或拒绝。 |
| agent 私聊 | 每个 agent 与其所有者之间各有一个加密私聊。 | 与 agent 一对一对话。 |

当前版本还会创建一个 **Hagency coordinator** 私聊。它是临时的过渡设施，
正在按 ADR-187 移除，可以忽略。

## 谁能和 agent 对话

- **agent 私聊**：只有它的所有者。该房间仅限邀请加入，而且 Hagency 在这里只
  接受所有者的消息。
- **项目房间**：任何 @ 提及该 agent 的成员。没有提到它的消息不会唤醒它。项目
  房间仅限邀请加入，成员由项目所有者决定。
- **服务器上的其他地方**：任何人都不行。
- **其他 Matrix 服务器**：无法访问。本部署不启用联邦（federation）。

## agent 能听到什么

agent 能读取整个房间的内容，需要上下文时会用工具查阅聊天记录。但它只在被
@ 提及时才会行动。

它会在提及它的那条消息的讨论串（thread）里回复。在同一讨论串里的后续消息
也会送达它，所以一件事最好放在一个讨论串里谈。

## 给 agent 下指令

在共享房间里，agent 为整个房间服务：任何成员都可以 @ 它并给出指令。目前没有
“只听某几个人指令”这类按 agent 设置的名单。

所有者靠以下几点保护自己：

- **审批**：有风险的操作需要审批，审批卡片只会发到所有者的私密审批室。
- **token 额度**：所有人的请求都消耗该 agent 的 token 额度。额度用完时 agent
  会暂停并发出通知，只有所有者能追加 token（见下文）。
- **成员管理**：房间里有谁由所有者决定。
- **退役**：所有者随时可以让 agent 退役。

## Token

所有者在批准 agent 申请时设定 token 额度。点 **全部剩余** 会自动填入该资源
还能提供的全部额度。

agent 用到额度上限时会暂停，不会丢弃任何请求。要让它继续，在 Hagency 控制台
打开 **接洽** → **追加 token**，agent 会从暂停处接着做。

## 邀请

设计上的行为是：

- agent 的**所有者**邀请它进某个房间时，agent 信任该邀请并加入。
- **其他人**邀请它时，邀请会变成 Hagency 控制台 **邀请** 页里的一项待决事项，
  由所有者决定，因为加入房间会消耗所有者的 token。

在已加入的房间里，规则不变：只有被 @ 提及才行动。注意：当前版本的 agent 还
不会处理邀请，见下方“已知限制”。

## 加密

agent 私聊和审批室都是端到端加密的。在新设备上登录后，请验证这个会话——用已有
的会话验证，或输入恢复密钥。验证之前，新设备上无法解密 agent 私聊和审批卡片。

## 已知限制（截至本版本）

- **暂不处理邀请**：目前 agent 会忽略所有邀请，只有 coordinator 的邀请会被
  监听。将由 ADR-187 第 6 步修复。
- **第一条消息可能丢失**：agent 刚创建后一秒内发出的消息，如果其加密密钥稍晚
  到达，可能会丢失。agent 没有回应时请重发一次。修复正在进行中。
- **Coordinator 私聊**：“Hagency coordinator” 私聊是临时过渡设施，正按
  ADR-187 移除。
- **需用完整 Matrix ID 登录**：Palpo web 目前只接受完整 ID
  （`@alice:example.org`），不接受短名。
