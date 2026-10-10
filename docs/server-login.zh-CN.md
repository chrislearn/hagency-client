[English](server-login.md) | [中文](server-login.zh-CN.md)

# 个人服务器登录

按[开发指南](local-development.zh-CN.md)构建运行当前 owner client。`hagency start`
打开本机控制台；`hagency open --state-dir <同一状态目录>` 通过运行中 host 的私人
IPC 获取访问链接。本机控制台访问与 server 认证是两个步骤。

选择保存的账号/服务器或输入 Hagency server origin。本机开发使用
`http://127.0.0.1:8088`，远程或命名域名需要可信 HTTPS。不追加 `/login`、
`/_pasion/` 或 API 路径。在官方 Pasion 浏览器流程中使用自己的个人账号授权。

Pasion PKCE 与实际 Matrix whoami 验证身份。原生管理使用 `/api/hagency/v1`，
server 浏览器管理使用封闭 `/api/browser/hagency/v1` BFF；原生 API 拒绝浏览器
Cookie/Origin。集成 Appservice 由 server 持有，不需要逐安装 Fleet、AS 注册、
machine-token 导入或 `[fleet_access]` 配置。

登录后创建/采纳 Project Space 与有权限的讨论 Room，再创建或选择 Agent。
永久主人身份、独立 Room 成员关系及创建策略由 server 检查；模型凭据、预算与
工具由本机主人控制。登录不执行推理，资源配置后显式启动活动范围。

保存的 profile 与近期服务器地址支持切换账号。切换停止前一主人的运行并要求新鲜
个人授权；账号状态、工作目录与账本继续隔离。模型登录与 Pasion 登录独立，不向
server 导出模型凭据。

失败时先检查[server 发现/TLS/就绪](../../hagency-server/docs/LOCAL_DEPLOYMENT.zh-CN.md)
再重试登录。创建结果与执行结果未知时保留原请求并显式恢复，不制造重复。
旧 Fleet 登录记录不是当前 owner 格式的配置指南。
