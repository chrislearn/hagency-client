[English](README.md) | [中文](README.zh-CN.md)

# Hagency 使用指南

本指南介绍如何通过 Hagency 为 Palpo Matrix 服务器添加 AI agent，以及如何在
Rinx Matrix 客户端中与 agent 协作。第一次使用时请按顺序完成各步骤。后面的
章节介绍日常使用和常见问题。

控制台可以切换中英文。下文先写中文界面上的名称，括号内是英文界面上的名称。
Palpo 网页端的界面是英文的，因此 Palpo 的页面和按钮名称保持英文原文。

## 本指南使用的术语

- **Hagency**：运行 AI agent 并把它们借给 Matrix 服务器上各个项目的服务。
- **Palpo**：存放项目和账号的 Matrix 服务器（homeserver）。Palpo 有自己的
  网页管理后台。
- **Rinx**：你用来和 agent 对话的 Matrix 聊天软件。
- **Hagency 控制台**：Hagency 的网页。运维者在这里设置 Hagency、连接 Palpo、
  管理资源、审批 agent 申请。
- **车队（fleet）**：Palpo 服务器眼中的一个 Hagency 安装。Palpo 为车队预留一段
  Matrix 账号名，全部以 `hf_` 开头，Hagency 用这些账号名创建车队的 agent。一个
  Hagency 只服务一个车队。
- **资源**：Hagency 提供给 Palpo 的一组模型、推理档位和每月 token 上限。项目在
  已发布的资源上定义 agent。
- **Agent**：Hagency 在你的服务器上以 Matrix 账号形式创建的 AI 工作者。
- **所有者**：为项目申请该 agent 的 Matrix 用户。所有者会收到 agent 的私聊和
  审批卡片。
- **访客**：和 agent 同在一个房间、但不是它所有者的任何人，例如项目成员。访客
  用 @ 提及 agent 来和它对话。
- **私聊（DM）**：agent 与其所有者之间的加密私密聊天。
- **审批室**：项目专属的私密房间，Hagency 的审批机器人在这里向所有者发送审批
  卡片。

## 角色分工

| 角色 | 做什么 |
| --- | --- |
| Palpo 管理员 | 把这个 Hagency 添加到 Palpo 服务器，只需一次。 |
| Hagency 运维者 | 登录 Codex，运行 Hagency，在控制台完成设置（编程代理、Palpo 连接、资源），审批 agent 申请。 |
| 所有者 | 在 Palpo 中创建项目及其审批室，申请 agent，接受 agent 的私聊邀请，处理审批卡片。 |
| 访客 | 项目成员，以及和 agent 同在一个房间里的其他人。他们在共享房间里用 @ 提及 agent 来和它对话。见[与 agent 协作：所有者与访客](#与-agent-协作所有者与访客)。 |

一个人可以同时担任多个角色。

## 开始之前

你需要准备：

- 一台可通过 `https` 访问的 Palpo 服务器，例如
  `https://matrix.your-server.example`。
- 该 Palpo 服务器上的一个管理员账号。
- `hagency` 程序。发布构建已内置控制台。当前版本 `nv0.1.0-rc.1` 是项目
  [GitHub Releases 页面](https://github.com/hagency-org/hagency-rs/releases)上的
  预发布版本（pre-release）；其中的 `.tar.gz` 资产和 `SHA256SUMS` 由发布工作流
  构建，再手动附加到该版本上。下载 `SHA256SUMS` 和对应平台的归档
  `hagency-nv0.1.0-rc.1-<target>.tar.gz`，其中 `<target>` 为
  `aarch64-apple-darwin`、`x86_64-apple-darwin`、`x86_64-unknown-linux-gnu` 或
  `aarch64-unknown-linux-gnu`（例如 Apple 芯片的 Mac 上为
  `hagency-nv0.1.0-rc.1-aarch64-apple-darwin.tar.gz`）。先用
  `shasum -a 256 -c --ignore-missing SHA256SUMS`（Linux：
  `sha256sum -c --ignore-missing SHA256SUMS`）校验归档，再用 `tar -xzf` 解压；
  其中只有 `hagency` 二进制。在 macOS 上，二进制没有代码签名，请运行一次
  `xattr -d com.apple.quarantine hagency`。`./hagency --version` 输出 `0.1.0`；
  `rc.1` 后缀只出现在标签和资产名称中。仓库 README 的
  [获取 hagency 二进制](../../README.zh-CN.md#2-获取-hagency-二进制)一节完整介绍如何获取它，
  包括如何从源码构建。
- 运行 Hagency 的机器上已安装的 Codex。你在第 1 步中自己登录它。
- 一个已在 Rinx 中设置好交叉签名（cross-signing）的所有者账号，例如已设置
  安全备份，或已验证过会话。所有者有交叉签名密钥之前，Hagency 不会为其创建
  agent。

第 1 到第 5 步由运维者完成。第 6 到第 8 步需要所有者参与。

## 第 1 步：登录 Codex

车队的 agent 使用运行 Hagency 的机器上的 Codex 登录运行，不使用在控制台中添加
的账户。

在那台机器上，由你自己登录 Codex：

```bash
codex login
```

机器上没有浏览器时，加上 `--device-auth`。

Hagency 从不替你登录，也从不读取或保存你的凭据。它只询问 Codex 是否已登录。

## 第 2 步：启动 Hagency 并打开控制台

1. 用以下两种方式之一启动 Hagency：
   - **作为服务运行（推荐）：**

     ```bash
     hagency service install
     ```

     服务以你的身份运行，并在你登录时重新启动。在 Linux 上它是一个
     `systemd --user` 单元，因此不需要 `sudo`。要在你退出登录后继续运行，
     运行一次 `loginctl enable-linger $USER`。
   - **在终端中运行：**

     ```bash
     hagency start
     ```

     它会一直运行，直到你按 Ctrl-C。

   两种方式都把 Hagency 的数据保存在本机的一个默认目录中。仓库 README 的
   [启动 Hagency](../../README.zh-CN.md#3-启动-hagency)一节介绍这两个命令。
2. Hagency 会输出一个控制台链接，并在浏览器中打开它。如果没有打开浏览器，
   请自己在这台机器上的浏览器中打开输出的链接。
3. 控制台会打开，并保持登录，直到你点击 **结束访问（End access）** 或关闭
   浏览器。重启 Hagency 不会让你退出登录。在你生成新链接之前，这个链接一直
   有效，请妥善保管。

之后要生成新链接：

```bash
# macOS
hagency console-access --state-dir "$HOME/Library/Application Support/Hagency"
# Linux
hagency console-access --state-dir "${XDG_DATA_HOME:-$HOME/.local/share}/hagency"
```

## 第 3 步：在控制台中设置编程代理

1. 在控制台菜单中打开 **设置（Setup）**。页面有三步：**编程代理（Coding
   agents）**、**连接 Palpo（Connect Palpo）** 和 **提供资源（Offer a
   resource）**。每一步完成后都会显示一个勾。三步全部完成之前，控制台的
   其他每个页面都会显示一行提示“设置尚未完成”，并附有指向 **设置** 的
   链接。
2. 在 **编程代理** 下，Hagency 显示 Codex 的路径、版本以及是否已登录。
   - 如果 Codex 未安装，安装它，然后点击 **重新检查（Check again）**。
   - 如果 Codex 未登录，在这台机器的终端里运行 `codex login`（第 1 步），然后
     点击 **重新检查**。
3. Codex 已登录时，无需点击任何按钮。页面加载时，Hagency 会自行完成运行
   Codex 所需的配置。页面随后显示“Hagency 已配置好运行这个代理。”

页面会显示 Codex 的登录方式：**ChatGPT 订阅（ChatGPT plan）** 或 **API 密钥（API key）**。ChatGPT 订阅登录仅供个人使用。在把这个代理提供给他人之前，请考虑
用 API 密钥登录 Codex。页面会给出这条提示，但不会阻止你继续。

## 第 4 步：连接 Palpo

**Hagency Server + Pasion 的推荐流程：** 在客户端此步骤或项目方页面输入服务器地址和 Hafleet 名称，点击“登录并连接”，通过 Pasion 登录后自动创建和配置 Hafleet。管理员需开放 `[hafleet_access].allow_self_service`。首次绑定仍需本地访问链接；后续登录必须是同一服务器和账号。详见[登录与接入](../server-login.zh-CN.md)。

以下手工步骤适用于旧版服务器或管理员提供的配置。


以下 Palpo 页面属于 Palpo，不属于 Hagency，因此具体布局可能随 Palpo 版本不同
而变化。

1. 用管理员账号登录 Palpo 网页管理后台。
2. 在服务器上添加一个 Hagency。填写将拥有它的 Matrix 账号，并选择出站连接
   方式。
3. 用拥有这个 Hagency 的账号登录 Palpo 网页管理后台。
4. 打开 **My Hagency access**，点击 **Download Hagency configuration**。
   Palpo 会下载一个 JSON 文件。请妥善保管，它包含车队的凭据。
5. 在控制台的 **设置** 页面中，转到 **连接 Palpo**。
6. 在 **配置文件（Configuration file）** 处选择该 JSON 文件。控制台会显示
   “*你的服务器* 上的车队：”和车队 ID。
7. 在 **Matrix 地址（Matrix address）** 处填写服务器的 Matrix 地址，例如
   `https://matrix.your-server.example`。地址必须使用 `https`。
8. 点击 **连接（Connect）**。控制台显示“已连接 *你的服务器*。”，无需重启。
9. 回到 Palpo 网页管理后台，点击
   **Verify connection & create reception**。Palpo 报告成功后，服务器上的
   项目就可以申请 agent 了。

添加 Hagency 需要管理员，因为这一步要为车队预留一段账号名。这一步完成后，
日常使用不再需要管理员权限。

同样的导入也在 **项目方（Project sides）** 页面的 **连接 Palpo 项目服务器（Connect a Palpo project server）** 面板中。

车队不需要协调者 agent。Hagency 自己创建 agent 和审批机器人，你也不会
看到“Hagency coordinator”私聊。

一个 Hagency 只连接一个 Palpo 车队。导入第二个车队会被拒绝，并提示“本 Hagency
已连接另一个 Palpo 车队。”

## 第 5 步：提供资源

1. 在 **设置** 页面中，转到 **提供资源**。这一步需要先完成第 3 步。
2. 在 **模型（Model）** 中选择模型和推理档位。列表中只有 Hagency 已认定资格的
   组合。对 Codex 来说，就是 `gpt-5.6-sol` 搭配 `low`、`medium` 或 `high`
   推理档位。
3. 在 **每月 token 上限（Monthly token ceiling）** 中保留 20,000,000，或填写
   另一个正整数。
4. 点击 **提供给 Palpo（Offer to Palpo）**。页面显示“已提供。项目所有者现在
   可以基于这个资源申请代理。”

要再提供一种模型或推理档位，重复以上步骤。

在 **我的资源（My resources）** 中管理资源。每一行显示资源的模型、每月 token
上限，以及它是 **已包含（Included）**（已发布给 Palpo）还是 **已撤下（Withdrawn）**。

- 要修改资源，在该行点击 **编辑资源配置（Edit configuration）**。选择模型和
  推理档位，填写 **每月 token 上限**，然后点击 **保存配置（Save
  configuration）**。资源上有已预留或运行中的 agent 时，不能修改该资源。
- 要停止提供某个资源，点击 **从本地资源目录撤下（Withdraw from native
  catalog）**。点击 **加入本地资源目录（Include in native catalog）** 可以重新
  提供。
- 车队不要使用 **托管账户（Managed accounts）** 和 **登记资源（Enroll
  resource）**。在那里创建的资源绑定到托管账户，车队的 agent 无法在其上运行。
  没有任何资源时，**我的资源** 会引导你前往 **设置**。

Hagency 每 15 秒把已发布的资源发送给 Palpo。所有者申请 agent 时从中选择。

仓库 README 介绍了命令行方式，包括用于创建资源的
[运维 API](../../README.zh-CN.md#用运维-api-创建资源)。

## 第 6 步：申请并批准 agent

1. **所有者**：在 Palpo 中用 **Create project and approval room** 创建或登记
   项目。然后为项目定义一个 agent：agent 名称、一个已发布的资源、角色、申请的
   token 数和每日速率（daily rate）。这些是 Palpo 的页面，不在 Hagency 里，
   具体布局可能随 Palpo 版本不同而变化。
2. **运维者**：在控制台中打开 **接洽（Engagements）**。在 **请求（Requests）** 标签页的 **待裁定（Pending verdicts）** 中可以看到这条申请，
   以及候选资源和该资源的剩余 token。
3. 在 **Token 数（Tokens）** 中保留申请的数量，或填写另一个正整数。
   **全部剩余（All remaining）** 会填入该资源还能提供的全部额度。
4. 点击 **批准（Approve）**。要拒绝申请，点击 **拒绝（Reject）**，再点
   **确认（Confirm）**。

批准后，Hagency 会创建 agent。agent 加入项目房间，并邀请所有者进入一个新的
私聊。

## 第 7 步：接受私聊邀请并与 agent 对话

1. **所有者**：在 Rinx 中接受 agent 私聊的邀请。在你加入之前，agent 会一直
   等待，没有时间限制。如果 Hagency 在你加入之前重启，见[常见问题](#常见问题)。
2. 在私聊中发送消息。在私聊里，agent 会回复所有者的每一条消息。这个私聊只属于
   你和 agent；如果有其他人加入，agent 就不再在这里回复。
3. 在项目房间里，任何成员都可以 @ 提及 agent 来提问。agent 会在那条消息的
   讨论串（thread）中回复。后续消息请留在同一个讨论串里。

## 第 8 步：审批 agent 的操作

有些操作（例如运行某些命令）需要所有者批准。

审批室就是所有者在第 6 步用 **Create project and approval room** 在 Palpo 中
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

## 与 agent 协作：所有者与访客

和 agent 对话的人，要么是它的**所有者**，要么是**访客**。所有者是申请这个
agent 的人。访客是和 agent 同在一个房间里的其他任何人：项目成员，或 agent
加入的其他房间里的人。

| | 所有者 | 访客 |
| --- | --- | --- |
| 在哪里和 agent 对话 | 私聊、项目房间，以及 agent 加入的任何房间 | 项目房间，以及 agent 加入的未加密房间 |
| 怎样让它回复 | 在私聊或只有所有者一个人的房间：任何消息。在共享房间：@ 提及 agent | @ 提及 agent，它会在该消息的讨论串中回复 |
| 追问 | 在同一讨论串中回复 | 在同一讨论串中回复 |
| 邀请 agent 进入其他房间 | agent 自动接受 | 由运维者在控制台中接受或拒绝 |
| 审批 agent 的操作 | 可以，用审批室中的卡片 | 不可以。访客会看到“Agent *name* is waiting for approval from its owner.”，需要等待 |
| token | 所有人（包括访客）的工作都消耗所有者的额度 | 消耗所有者的额度 |
| 追加 token 或结束 agent 的工作 | 联系运维者，由运维者在控制台中操作（**追加 token（Add tokens）**、**结束接洽（Retire）**） | 不可以 |

### 作为所有者

1. 在 Rinx 中接受 agent 的私聊邀请，并在私聊中与它对话。私聊只属于你和
   agent；如果有其他人加入，agent 会停止在私聊中回复。
2. 在项目房间中 @ 提及 agent，并把对话留在该消息的讨论串中。
3. 要在其他房间使用 agent，用它的完整 Matrix ID 邀请它（见
   [在其他房间使用 agent](#在其他房间使用-agent)）。如果房间里还有其他人，
   创建房间时请关闭加密。
4. 留意你的审批室。无论是谁提出的请求，需要审批的操作都会以卡片的形式发给你。
   在你回答之前，agent 会一直等待。
5. agent 因 token 用完而暂停时，请联系运维者追加额度（见
   [管理 token](#管理-token)）。

### 作为访客

1. 在项目房间，或 agent 所在的其他未加密房间中，@ 提及 agent。它会在你那条
   消息的讨论串中回复；追问也请留在那里。
2. 如果 agent 发出正在等待所有者审批的消息，说明所有者需要先回答一张卡片。
   请联系所有者，而不是 agent。
3. 你可以邀请 agent 进入你的房间，但要等运维者接受后它才会加入，因为它在那里
   的工作会消耗所有者的 token。
4. 在有其他人的加密房间中，agent 不工作，只会发一条通知。请使用未加密的房间。
5. 你不能和 agent 私聊：在私聊中它只回复所有者。

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
- **其他人邀请**：邀请会出现在控制台的 **邀请（Invitations）** 页面中，等待
  运维者点击 **接受（Accept）** 或 **拒绝（Decline）**，因为在那个房间工作会
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

1. 在控制台中打开 **接洽（Engagements）**。
2. 在该 agent 所在行点击 **追加 token（Add tokens）**。
3. 填写数量，或点击 **全部剩余（All remaining）**，然后点击 **追加（Add）**。

agent 会继续工作，并发出“Resumed: N tokens available.”（已恢复：可用 N 个
token）。

要结束 agent 在某个项目中的工作，在该行点击 **结束接洽（Retire）**，再点
**确认（Confirm）**。

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
  agent 的创建，控制台也没有恢复它的操作。运维者打开 **接洽（Engagements）**，
  在该 agent 所在行点击 **结束接洽（Retire）**，再点 **确认（Confirm）**。随后
  所有者在 Palpo 中重新定义该 agent，运维者批准新的申请。
- 确认所有者已设置交叉签名。Hagency 要读到所有者的交叉签名密钥后才会继续。
- 确认连接后已在 Palpo 中点击 **Verify connection & create reception**。
- 查看 Hagency 日志中的“fleet service stage”一行，它会显示车队在等什么：
  `awaiting_runtime_config`（Hagency 还没有为 Codex 完成配置：请完成
  **设置 → 编程代理**，见第 3 步）或 `awaiting_reception`（Palpo 尚未验证
  连接）。服务日志在 macOS 上是 `~/Library/Logs/Hagency/hagency.log`，在
  Linux 上用 `journalctl --user -u hagency` 查看。

**日志显示 `refused_config`，或设置页面提示“编程代理已变化”。**
Codex 已更新，而 Hagency 的配置仍指向旧的 Codex 二进制。

1. 在控制台中打开 **设置（Setup）**。它会自动更新配置（如果提示一直存在，点击
   **重新检查（Check again）**），并把旧文件保留为备份。
2. 按页面提示重启 Hagency。Hagency 只在启动时读取配置，因此在重启之前，
   运行中的服务一直使用旧配置：
   - macOS：`launchctl kickstart -k gui/$(id -u)/io.hagency`
   - Linux：`systemctl --user restart hagency`
   - 在终端中运行时：按 Ctrl-C 停止 `hagency start`，再重新运行它。

设置页面使用默认的 Codex 目录（`$CODEX_HOME` 或 `~/.codex`）写入配置。如果你
当初用 `hagency setup --codex-home` 或 `--no-local-codex` 完成设置，或者没有
控制台，请改用相同的选项运行 `hagency setup --state-dir <state> --force`，
然后重启 Hagency。

**“批准”按钮是灰色的。**
没有已发布的资源能满足这条申请。请按第 5 步检查 **我的资源**。

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

- **agent 等待所有者期间重启会使它停滞。** 见[常见问题](#常见问题)。
- **控制台无法接受所有者变更后的密钥。** Hagency 信任它第一次看到的所有者交叉
  签名密钥。如果所有者之后重置了交叉签名，控制台没有信任新密钥的操作。
- **控制台不显示加入的房间。** 控制台不会列出 agent 创建之后加入的房间。
- **不在与他人共享的加密房间中工作。** agent 会留在这类房间里，但不在那里工作
  （见 [agent 在房间里的行为](#agent-在房间里的行为)）。
- **只支持一台 homeserver。** agent 只与车队所在 Palpo 服务器上的人和房间协作。
  其他 Matrix 服务器上的用户和房间无法与它们协作。
