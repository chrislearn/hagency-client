# 项目看板 Markdown 与完整客户端联调验收

日期：2026-10-23。Client / Server 分支均为 `codex/project-kanban`。

## 实现

描述和评论使用同一套原生 Markdown 编辑器：编辑／预览切换，加粗、斜体、二级标题、无序／有序列表、引用、行内代码、代码块、链接工具栏。格式变换保留中文选区，进入原生 TextInput 撤销栈，并恢复输入焦点；IME 组合输入期间不改写内容。已有描述默认预览，保存后的评论按 Markdown 渲染；标题、标签、清单条目仍为纯文本。

存储继续使用现有 Server 字符串字段，不需要 Markdown 专用 API、数据库迁移或 Cloud 改动。预览使用现有 pulldown-cmark 和原生 RinxHtml。原始 HTML 转义，图片只显示替代文字，不自动请求远端图片；链接限 HTTP/HTTPS，代码块不执行。不是所见即所得富文本编辑器。

原生验收发现 TextInput 的通用 set_visible 不会隐藏布局，已用 source_wrap 容器切换显隐；回归测试检查此容器与工具栏隐藏。详情滚动区增加 12px 右侧留白，格式按钮压缩为 26px，避免滚动条压住内容。空白行的标题／列表操作也能插入标记。

另修复不支持看板的 Server 被标为“离线”的提示。完整客户端调试启动支持 HAGENCY_APP_RESOURCE_DIR 指向相同的暂存资源；仅 debug 构建生效。

## 实际操作结果

1. 原有账号的完整客户端：项目列表 → test project → 项目详情 → Board，导航正常；远端 hagency.meldry.com 返回尚不支持看板。未升级或修改远端服务。
2. 用户选择本地新版 Server 联调后，运行同一个完整 `hagency` 二进制，独立数据目录、Alice 测试身份。使用 SDK 正常 OAuth 会话恢复、issuer / whoami / device 校验和 NativeOwner 授权；没有给生产客户端增加身份绕过路径。
3. 原生窗口：项目列表 → 本地项目 · 看板联调 → 概览 → 看板，读回既有自定义列、WIP、任务与子任务计数。
4. 新任务 #20“Markdown 编辑与预览验收”：中文选区加粗，Cmd+Z 撤销，描述预览标题、粗体、斜体、列表、引用、行内与多行代码、链接；保存到真实 Server / PostgreSQL。
5. 在完整客户端搜索 Markdown，打开 #20，切回源码编辑，追加完整客户端联调说明，预览并保存。SQL 确认任务版本从 1 变为 2，Markdown 原文保留。
6. 完整客户端评论：选区加粗、追加列表、切换预览、提交；关闭任务、切换概览／看板并重新打开，已保存评论按 Markdown 显示。
7. 重启本地 Server 和完整客户端，再从项目入口进入，#20 描述中的新增段落仍在；评论原文由只读 SQL 确认保留。测试账号数据与用户真实账号目录隔离。

本地环境中的 Pasion introspection、Matrix metadata / whoami / 空 room sync 是显式测试替身，位于 Server example 内；项目 API、NativeOwner 授权、看板存储均为真实实现。这验证完整客户端的项目到看板读写流程，不等同于完整 Matrix 聊天、真实 Pasion 浏览器登录或生产部署验收。未知 Matrix 能力返回明确错误，不伪造聊天成功。

## 可复用启动

Server 原有 project_board_fixture 启动方式不变，新增完整客户端所需的测试身份协议。数据库必须是 loopback 且名称以 kanban_ 开头，HTTP 仅监听 127.0.0.1。未携带测试 token 的同步请求实际验证返回 401。

Client 从自己的 checkout 首次准备独立会话：

```sh
HAGENCY_BOARD_FIXTURE_INFO=/tmp/hagency-kanban-interactive-info.json \
  cargo +1.99.0 run --locked -p hagency-app --example prepare_project_client_fixture
```

程序打印独立临时数据目录，若已存在则拒绝覆盖。随后将 `HAGENCY_APP_DATA_DIR` 设置为该目录运行正常 `hagency` 二进制；不需要修改 Service::for_current 或生产认证逻辑。

当前可查看的原生完整客户端包：`/tmp/HagencyLocal.app`。本地 API 为 `http://127.0.0.1:55474/`，数据库端口 55473。源码资源无法从原生启动读取时，调试包使用 `/tmp/hagency-kanban-assets` 中的相同资源。

## 检查

- Board 专项 18/18 通过：包括 Markdown 渲染边界、中文选区、预览容器、只读冲突、旧账户／请求隔离、拖动和布局。
- App 完整单元套件 597 通过、0 失败、2 项原有忽略；严格 Clippy 通过。最终结果见 `/tmp/hagency-markdown-full-tests-final.log`、`/tmp/hagency-markdown-clippy-final.log`。
- 翻译检查 2332 条、1766 个调用点、0 缺失；git diff --check 通过。
- Server fixture build 与严格 Clippy 通过：`/tmp/hagency-full-fixture-build.log`、`/tmp/hagency-full-fixture-clippy.log`。
- 本次 Server 修改仅限本地联调 example，不改生产路由或看板协议；Cloud 无改动。
