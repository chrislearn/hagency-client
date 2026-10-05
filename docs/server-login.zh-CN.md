[English](server-login.md) | [中文](server-login.zh-CN.md)

# Pasion 登录与 Fleet 自动接入

使用本地 `hagency-client` 项目，编译后的命令仍叫 `hagency`。通过
`hagency start` 启动；源码构建使用独立前端资源时，可以运行
`hagency serve --state-dir <state> --palpo-transport --console-assets <assets>`。
首次绑定使用 `hagency console-access` 生成的本地访问链接，状态目录和监听
地址要与运行中的客户端一致。

在设置页面或项目方页面填写 Hagency Server 地址、Fleet 名称，点击
**登录并连接**，在服务器的 Pasion 页面登录。Rust 后端通过 PKCE 换取 token，
验证账号、创建自己的 Fleet、保存返回的配置并启动 outbound 连接。
连接验证会自动重试，初次尝试后持续约三分钟；超时会显示失败。
仅接入 Fleet 不会执行编程 agent，编程运行时和资源提供需要单独设置。

服务器需要启用 Pasion 委托认证，在服务器自己的 `hagency.toml` 中配置：

```toml
[fleet_access]
allow_self_service = true
max_per_user = 3
```

自助接入默认关闭。未开放时，有效账号仍能登录本地，但不会创建 Fleet。
App Service 的注册权限由服务器持有，用户只取得自己 Fleet 的凭据。
本地状态目录保存一个安装 ID，重复登录会复用同一个 Fleet；每个 Fleet
对应自己的 App Service。客户端通过 outbound 连接，无需公网 IP 或供服务器
回调的域名，OAuth 的浏览器回调使用回环 IP。远端服务器必须使用 HTTPS；
本地开发只允许回环 IP 的 HTTP 地址。

首次绑定需要已有的本地操作员访问权限，随后服务器地址、Pasion subject 和
Matrix 账号固定在私有文件 `server-login.json` 中，其他账号不能取得本机控制权。
Pasion token 仅保存在 Rust 进程内存中，不进入浏览器存储或绑定文件。本地
后端通过已绑定服务器验证 token，服务器再使用私有服务凭据查询 Pasion。
验证结果最多缓存 30 秒，过期、撤销或无法验证的 token 会被拒绝。本地会话
最长 15 分钟，重启后需要重新登录；`console-access` 保留为本地恢复通道。

`palpo-transport.json`、`palpo.machine_token` 和 `palpo-appservice.json` 仍是
私有运行配置。机器凭据与人的登录分开，因此浏览器退出登录不会停止已经
配置好的 Fleet。管理员提供的旧配置仍能通过 **导入已有配置（可选）** 导入。
已配置其他 Fleet 的本地安装会拒绝自动切换；继续使用现有配置，或为新的
自动接入初始化独立状态目录。

服务器原生 API：`GET /_hagency/client/v1/discovery`、`GET .../identity`、
`POST .../fleets` 和 `POST .../fleets/{id}/connect`。认证请求使用 Pasion
Bearer token，不携带浏览器 Cookie 或 Origin。创建接口只接受 `installationId`
和 `name`，所有者由服务器验证后的身份确定。
