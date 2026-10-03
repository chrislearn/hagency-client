[English](README.md) | [中文](README.zh-CN.md)

# Hagency 使用指南

本指南介绍如何通过 Hagency 为 Palpo Matrix 服务器添加 AI agent，以及如何在
Rinx Matrix 客户端中与 agent 协作。第一次使用时请按顺序完成各步骤。后面的
章节介绍日常使用和常见问题。

控制台可以切换中英文。下文先写中文界面上的名称，括号内是英文界面上的名称。

## 本指南使用的术语

- **Hagency**：运行 AI agent 并把它们借给 Matrix 服务器上各个项目的服务。
- **Palpo**：存放项目和账号的 Matrix 服务器（homeserver）。Palpo 有自己的
  网页管理后台。
- **Rinx**：你用来和 agent 对话的 Matrix 聊天软件。
- **Hagency 控制台**：Hagency 的网页。运维者在这里连接 Palpo、管理资源、
  审批 agent 申请。
- **车队（fleet）**：Palpo 服务器眼中的一个 Hagency 安装。Palpo 为车队预留一段
  Matrix 账号名，全部以 `hf_` 开头，Hagency 用这些账号名创建车队的 agent。一个
  Hagency 只服务一个车队。
- **资源**：Hagency 提供给 Palpo 的一组模型、推理档位和每月 token 上限。项目在
  已发布的资源上定义 agent。
- **Agent**：Hagency 在你的服务器上以 Matrix 账号形式创建的 AI 工作者。
- **所有者**：为项目申请该 agent 的 Matrix 用户。所有者会收到 agent 的私聊和
  审批卡片。
- **私聊（DM）**：agent 与其所有者之间的加密私密聊天。
- **审批室**：项目专属的私密房间，Hagency 的审批机器人在这里向所有者发送审批
  卡片。

## 角色分工

| 角色 | 做什么 |
| --- | --- |
| Palpo 管理员 | 把这个 Hagency 添加到 Palpo 服务器，只需一次。 |
| Hagency 运维者 | 运行 Hagency，登录 Codex，在控制台连接车队、管理资源、审批 agent 申请。 |
| 所有者 | 在 Palpo 中创建项目及其审批室，申请 agent，接受 agent 的私聊邀请，处理审批卡片。 |
| 项目成员 | 在共享房间里用 @ 提及 agent 来和它对话。 |

一个人可以同时担任多个角色。

## 开始之前

你需要准备：

- 一台可通过 `https` 访问的 Palpo 服务器，例如
  `https://matrix.your-server.example`。
- 该 Palpo 服务器上的一个管理员账号。
- 用本仓库构建出的 `hagency` 程序和控制台文件。构建方法见仓库的
  [README](../../README.zh-CN.md)。
- 运行 Hagency 的机器上已登录的 Codex（见第 4 步）。
- Hagency 状态目录中的 `fleet-runtime.json` 文件。它保存本机 Codex 运行设置，
  其中 `profile` 必须是 `palpo_fleet_runtime_v1`。各字段见仓库 README 的
  [配置](../../README.zh-CN.md#配置)一节。没有这个文件时，车队可以连接，但不会
  创建任何 agent。
- 一个已在 Rinx 中设置好交叉签名（cross-signing）的所有者账号，例如已设置
  安全备份，或已验证过会话。所有者有交叉签名密钥之前，Hagency 不会为其创建
  agent。

## 第 1 步：在 Palpo 中添加 Hagency

这些页面属于 Palpo，不属于 Hagency，因此具体布局可能随 Palpo 版本不同而变化。

1. 用管理员账号登录 Palpo 网页管理后台。
2. 在服务器上添加一个 Hagency。填写将拥有它的 Matrix 账号，并选择出站连接
   方式。
3. 用拥有这个 Hagency 的账号登录 Palpo 网页管理后台。
4. 打开 **My Hagency access**，点击 **Download Hagency configuration**。
   Palpo 会下载一个 JSON 文件。请妥善保管，它包含车队的凭据。

添加 Hagency 需要管理员，因为这一步要为车队预留一段账号名。这一步完成后，
日常使用不再需要管理员权限。

## 第 2 步：启动 Hagency 并打开控制台

在运行 Hagency 的机器上执行以下命令。请把路径换成你自己的路径。

1. 创建一个新的空状态目录（只需一次）：

   ```bash
   hagency init --state-dir /path/to/state
   ```

2. 启动 Hagency，并启用 Palpo 连接和控制台：

   ```bash
   hagency serve \
     --state-dir /path/to/state \
     --listen 127.0.0.1:13300 \
     --palpo-transport \
     --console-assets /path/to/console-assets
   ```

   - `--listen` 必须是带端口的本机回环地址。默认值是 `127.0.0.1:13300`。
   - 不要加 `--agent-driver`。这个参数会启动带协调者（coordinator）agent 的旧方式，
     此时 Hagency 不会自己运行车队。

3. 在另一个终端中生成控制台链接：

   ```bash
   hagency console-access --state-dir /path/to/state
   ```

   如果你改过 `--listen`，这里也要传入相同的 `--listen`。

4. 用同一台机器上的浏览器打开该链接。控制台会打开，并保持登录，直到你点击
   **结束访问**（End access）或关闭浏览器。重启 Hagency 不会让你退出登录。在你
   生成新链接之前，这个链接一直有效，请妥善保管。

## 第 3 步：在控制台中连接车队

1. 在控制台中打开 **项目方**（Project sides）。
2. 找到 **连接 Palpo 项目服务器**（Connect a Palpo project server）面板。
3. 在 **配置文件**（Configuration file）处选择第 1 步下载的 JSON 文件。控制台
   会显示“*你的服务器* 上的车队：”和车队 ID。
4. 在 **Matrix 地址**（Matrix address）处填写服务器的 Matrix 地址，例如
   `https://matrix.your-server.example`。地址必须使用 `https`。
5. 点击 **连接**（Connect）。控制台显示“已连接 *你的服务器*。”，无需重启。
6. 回到 Palpo 网页管理后台，点击
   **Verify connection & create reception**。Palpo 报告成功后，服务器上的
   项目就可以申请 agent 了。

车队不需要协调者 agent。Hagency 自己创建 agent 和审批机器人，你也不会
看到“Hagency coordinator”私聊。

一个 Hagency 只连接一个 Palpo 车队。导入第二个车队会被拒绝，并提示“本 Hagency
已连接另一个 Palpo 车队。”

## 第 4 步：登录 Codex 并检查资源

车队的 agent 使用运行 Hagency 的机器上已登录的 Codex 运行，不使用在控制台中添加
的账户。

1. 在 Hagency 所在的机器上，把 Codex 登录到车队的 Codex 目录。是哪个目录，
   取决于 `fleet-runtime.json`：
   - **有 `local_codex` 块时：** `local_codex.codex_home` 指定的目录。例如：

     ```bash
     CODEX_HOME=/path/to/codex-home codex login
     ```

   - **没有 `local_codex` 块时：** 状态目录中的 `runtime-home` 目录。Codex 把它
     同时用作 `HOME` 和 `CODEX_HOME`。车队服务在第 3 步之后加载
     `fleet-runtime.json` 时，Hagency 会创建它（仅服务用户可访问）。然后在那里
     登录：

     ```bash
     CODEX_HOME=/path/to/state/runtime-home codex login
     ```

   如果机器上没有浏览器，改用 `codex login --device-auth`。登录信息由 Codex 保存
   在该目录中，Hagency 不保存你的凭据。
2. 检查资源；如果还没有资源，就创建首个资源：
   - **（a）检查我的资源。** 在控制台中打开 **我的资源**（My resources）。每一行
     显示资源的模型、每月 token 上限，以及它是 **已包含**（Included，已发布给
     Palpo）还是 **已撤下**（Withdrawn）。没有任何资源时，控制台会提示在
     “托管账户”页面从托管账户登记首个资源。车队请忽略这条提示，也不要使用
     **托管账户**（Managed accounts）和 **登记资源**（Enroll resource）：在那里
     创建的资源绑定到托管账户，车队的 agent 无法在其上运行。
   - **（b）如果列表为空，运行一次运维 API。** 控制台无法创建车队的首个资源。
     由运维者在运行 Hagency 的机器上创建，请使用你自己的 `--listen` 地址和状态
     目录：

     ```bash
     curl -s -X POST http://127.0.0.1:13300/api/native/v1/resources \
       -H "Authorization: Bearer $(cat /path/to/state/operator.token)" \
       -H 'Content-Type: application/json' \
       -d '{"presetId":"local_codex","seatId":"local_codex_seat","framework":"codex","model":"gpt-5.6-sol","provider":"openai","reasoning":"medium","ceiling":{"tokens":20000000,"period":"monthly"},"published":true}'
     ```

     仓库 README 的[首次运行](../../README.zh-CN.md#首次运行)一节也有这一步。
   - **（c）规则。** 只有当资源的模型和推理档位是 Hagency 已为至少一个角色认定
     资格的组合时，Palpo 才能看到它。对 Codex 来说，就是 `gpt-5.6-sol` 搭配
     `low`、`medium` 或 `high` 推理档位。其他组合会被保存，但不会发布。有
     `local_codex` 块时，`seatId` 还必须等于 `local_codex.seat`，且 `framework`
     为 `codex`、`provider` 为 `openai` 或省略。不匹配的资源会被接受并发布，但
     Hagency 拒绝在其上运行 agent。

   之后其余操作都在控制台中完成（见下面第 3 至 5 项）。
3. 要修改资源，在该行点击 **编辑资源配置**（Edit configuration）。选择模型和
   推理档位，填写 **每月 token 上限**（Monthly token ceiling），然后点击
   **保存配置**（Save configuration）。页面只提供 Hagency 支持的模型和推理档位。
   资源上有已预留或运行中的 agent 时，不能修改该资源。
4. 要在同一个 Codex 登录上再提供一种模型或推理档位，点击 **新建资源配置**
   （New resource configuration），在 **来源配置**（Source configuration）中选择
   一个现有资源，设置模型、推理档位和上限，然后点击 **创建另一项配置**
   （Create another configuration）。新资源创建后立即发布。
5. 要停止提供某个资源，点击 **从本地资源目录撤下**（Withdraw from native
   catalog）。点击 **加入本地资源目录**（Include in native catalog）可以重新
   提供。

Hagency 每 15 秒把已发布的资源发送给 Palpo。所有者申请 agent 时从中选择。

## 第 5 步：申请并批准 agent

1. **所有者**：在 Palpo 中用 **Create project and approval room** 创建或登记
   项目。然后为项目定义一个 agent：agent 名称、一个已发布的资源、角色、申请的
   token 数和每日速率（daily rate）。这些是 Palpo 的页面，不在 Hagency 里，
   具体布局可能随 Palpo 版本不同而变化。
2. **运维者**：在控制台中打开 **接洽**（Engagements）。在 **请求**
   （Requests）标签页的 **待裁定**（Pending verdicts）中可以看到这条申请，
   以及候选资源和该资源的剩余 token。
3. 在 **Token 数**（Tokens）中保留申请的数量，或填写另一个正整数。
   **全部剩余**（All remaining）会填入该资源还能提供的全部额度。
4. 点击 **批准**（Approve）。要拒绝申请，点击 **拒绝**（Reject），再点
   **确认**（Confirm）。

批准后，Hagency 会创建 agent。agent 加入项目房间，并邀请所有者进入一个新的
私聊。

## 第 6 步：接受私聊邀请并与 agent 对话

1. **所有者**：在 Rinx 中接受 agent 私聊的邀请。在你加入之前，agent 会一直
   等待，没有时间限制。如果 Hagency 在你加入之前重启，见[常见问题](#常见问题)。
2. 在私聊中发送消息。在私聊里，agent 会回复所有者的每一条消息。这个私聊只属于
   你和 agent；如果有其他人加入，agent 就不再在这里回复。
3. 在项目房间里，任何成员都可以 @ 提及 agent 来提问。agent 会在那条消息的
   讨论串（thread）中回复。后续消息请留在同一个讨论串里。

## 第 7 步：审批 agent 的操作

有些操作（例如运行某些命令）需要所有者批准。

审批室就是所有者在第 5 步用 **Create project and approval room** 在 Palpo 中
创建的私密房间。Palpo 会邀请 Hagency 的审批机器人加入，只有当房间里恰好是
所有者和审批机器人时，Hagency 才会受理 agent 申请。

1. agent 需要审批时，会在项目房间发出“Agent *名字* is waiting for approval
   from its owner.”（agent 正在等待所有者审批）。
2. Hagency 的审批机器人会在所有者的审批室发送一张审批卡片。卡片写明 agent、
   项目、工具、输入内容和过期时间。
3. 点击其中一个按钮（按钮文字为英文）：
   - **Approve once**：只允许这一次操作。
   - **Allow for this task**：在当前任务剩余的时间里允许这类操作。
   - **Always allow this operation**：为该 agent 和项目保存这条精确的规则。
   - **Deny**：拒绝该操作。

   有些卡片只提供 **Approve once** 和 **Deny**。用文字回复不算作答复，请使用
   按钮。

Hagency 为每位所有者使用单独的审批机器人设备。一位所有者的卡片绝不会为另一位
所有者加密。

## 在其他房间使用 agent

你可以把 agent 带进同一服务器上的其他房间。

### 邀请 agent

1. 在 Rinx 中打开房间，用 agent 的完整 Matrix ID 邀请它，例如
   `@hf_...:your-server.example`。agent 的完整 ID 可以在它的私聊成员列表中
   看到。
2. 也可以在 Rinx 的邀请对话框中输入 agent 的名字。搜索只能找到与你同在某个
   房间的人，以及服务器用户目录返回的人，所以用完整 ID 最可靠。

接下来会发生什么，取决于是谁发出的邀请：

- **所有者邀请**：agent 会自动接受，通常在 10 秒内完成。
- **其他人邀请**：邀请会出现在控制台的 **邀请**（Invitations）页面中，等待
  运维者点击 **接受**（Accept）或 **拒绝**（Decline），因为在那个房间工作会
  消耗所有者的 token。

### agent 在房间里的行为

| 房间 | agent 的行为 |
| --- | --- |
| 有其他人的未加密房间 | 回复 @ 提及它的消息，回复在该消息的讨论串中。 |
| 只有所有者一个人的房间（加密与否均可） | 回复所有者的每一条消息。 |
| 有其他人的加密房间 | 不在这里工作。它会发一条通知，说明自己无法在有其他人的加密房间中工作。 |

在与他人共享的加密房间里，只有所有者能读到 agent 的回复，所以 agent 不在那里
回答。如果其他人继续发消息，它最多每 15 分钟重复一次通知。Matrix 房间一旦开启
加密就无法关闭。如果要让 agent 和其他人一起工作，请新建一个不加密的房间，再
邀请 agent 进去。

agent 只读取它加入之后发送的消息。如果你在它加入完成前就发了消息，请重新
发送。

审批和 token 在每个房间的规则都相同。审批卡片总是发到所有者的审批室，绝不会
发到共享房间。

## 管理 token

运维者批准申请时，会设定该 agent 的 token 额度。

agent 用完额度后会暂停，并发出“Paused: used N of M tokens. The owner can add
tokens in the Hagency console.”（已暂停：已用 N/M 个 token，所有者可以在
Hagency 控制台追加 token）。不会丢弃任何工作。只有运维者能打开控制台，所有者需请运维者追加 token。

追加 token 的步骤：

1. 在控制台中打开 **接洽**（Engagements）。
2. 在该 agent 所在行点击 **追加 token**（Add tokens）。
3. 填写数量，或点击 **全部剩余**（All remaining），然后点击 **追加**（Add）。

agent 会继续工作，并发出“Resumed: N tokens available.”（已恢复：可用 N 个
token）。

要结束 agent 在某个项目中的工作，在该行点击 **结束接洽**（Retire），再点
**确认**（Confirm）。

## 加密与你的设备

agent 私聊和审批室都是端到端加密的。

- agent 在私聊中的回复会送达所有者的全部会话，无论是否已验证。
- 审批卡片只会送达所有者已验证的会话。在一个尚未验证的 Rinx 新会话中，审批
  卡片会显示“Unable to decrypt”（无法解密）。这是有意的设计。请用另一个会话
  或恢复密钥验证当前会话，之后就能看到新的卡片。

## 常见问题

**agent 一直没有出现，或一直没有发来私聊。**
- 确认所有者已接受私聊邀请。所有者加入之前，agent 会一直等待。
- 如果 Hagency 在 agent 等待所有者加入私聊期间重启过，Hagency 永远不会完成该
  agent 的创建，控制台也没有恢复它的操作。运维者打开 **接洽**（Engagements），
  在该 agent 所在行点击 **结束接洽**（Retire），再点 **确认**（Confirm）。随后
  所有者在 Palpo 中重新定义该 agent，运维者批准新的申请。
- 确认所有者已设置交叉签名。Hagency 要读到所有者的交叉签名密钥后才会继续。
- 确认连接后已在 Palpo 中点击 **Verify connection & create reception**。
- 确认状态目录中有 `fleet-runtime.json`。Hagency 日志中的“fleet service
  stage”一行会显示车队在等什么：`awaiting_runtime_config`（缺少该文件）或
  `awaiting_reception`（Palpo 尚未验证连接）。

**“批准”按钮是灰色的。**
没有已发布的资源能满足这条申请。请按第 4 步检查 **我的资源**。

**agent 不理会我在群聊房间里的消息。**
在有其他人的房间里，agent 只在被 @ 提及时回复。

**搜索时找不到 agent。**
Matrix 的用户搜索通常只能找到与你同在某个房间的人。请用 agent 的完整 Matrix ID
邀请它。

**agent 说它无法在加密房间中工作。**
新建一个不加密的房间，并邀请 agent 进去。

**审批卡片显示“Unable to decrypt”。**
当前会话尚未验证。请验证它，或在已验证的会话中处理卡片。

**agent 停下来并发出“Paused”。**
它的 token 额度已用完。请在 **接洽** 中追加 token。

**消息中的链接没有预览。**
链接预览由 homeserver 生成，不由 Hagency 生成。请让 Palpo 管理员为你需要的
网站开启预览。

**控制台拒绝配置文件。**
- “这不是从 Palpo 下载的 Hagency 配置。”：请选择通过
  **Download Hagency configuration** 下载的文件。
- “此文件没有出站连接，将被拒绝”：请在 Palpo 中以出站连接方式重新添加
  Hagency。
- “本 Hagency 已连接另一个 Palpo 车队。”：一个 Hagency 只服务一个车队。

## 已知限制

- **安装车队需要手动一步。** 安装程序和它的服务文件按旧的协调者方式设置
  Hagency。要运行车队，安装 Hagency 的人需要从服务中去掉 `--agent-driver`，并
  手动编写 `fleet-runtime.json`。
- **agent 等待所有者期间重启会使它停滞。** 见[常见问题](#常见问题)。
- **控制台的空资源提示不适用于车队。** 见[第 4 步](#第-4-步登录-codex-并检查资源)第 2 项。
- **控制台无法接受所有者变更后的密钥。** Hagency 信任它第一次看到的所有者交叉
  签名密钥。如果所有者之后重置了交叉签名，控制台没有信任新密钥的操作。
- **控制台不显示加入的房间。** 控制台不会列出 agent 创建之后加入的房间。
- **不在与他人共享的加密房间中工作。** agent 会留在这类房间里，但不在那里工作
  （见 [agent 在房间里的行为](#agent-在房间里的行为)）。
- **只支持一台 homeserver。** agent 只与车队所在 Palpo 服务器上的人和房间协作。
  其他 Matrix 服务器上的用户和房间无法与它们协作。
