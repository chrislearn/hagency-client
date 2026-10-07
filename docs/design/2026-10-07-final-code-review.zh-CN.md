# 重构与登录修正的最终代码复审（2026-10-07）

范围是 chrislearn 下 hagency-client、hagency-server 的全部未提交修改，包括新增文件、生产入口、旧代码删除、测试替代、构建发布及文档。hagency-org 不是此次实现的目标；没有修改 Palpo。

## 方法和覆盖

按 Git 变更清单划分三条复审线，并由主代理复核修补后的授权与部署衔接。客户端登录审查覆盖 OAuth/PKCE、浏览器来源、有限本机会话、完整身份 pin、账号切换、Owner UI、CLI/IPC、安装服务、CI 与生产入口；本地执行审查覆盖 SQLite、预算结算、收件箱、provider 凭据目录、租约、任务停止、审批与精确工具许可；服务端审查覆盖必装 AS、永久 owner、Project/Space 与 Room、Matrix 权限、身份会话、设备、退休清理、恢复和部署。删除的 Fleet 产品路径核对了替代规格和独立 SDK 测试；旧库不会自动迁移，生产 CLI 不恢复 Fleet。

## 发现和处理

| 项目 | 问题 | 修正与验证 |
|---|---|---|
| 重新登录 | 本机会话过期使主按钮无法使用 | 已授权服务器允许开始 Pasion；回调只接受已授权完整身份，成功后停止旧任务并创建有限授权；浏览器与 HTTP 多账号回归 |
| OAuth 凭据清理 | 已签 OAuth 后，服务器 grant/身份校验/本地保存失败留下凭据 | 整个完成阶段统一错误清理；撤销记录持久重试，未签服务器 token 也可撤 OAuth；grant 拒绝及重启读取回归 |
| IPv6 IPC | `[::1]` 的合法本机链接被误拒 | 校验 IPv6 host 后仍要求 loopback；真实 IPC → HTTP 会话兑换回归 |
| 身份文件 | 512 字节限制拒绝合法长身份 | 32 KiB 私密有界读；长身份、超限、符号链接和权限错误回归 |
| 账本初始化 | 空库判断先于事务锁，竞争建表 | 取得 IMMEDIATE 锁后再读库状态；12 组 × 8 线程首次打开回归 |
| 收件箱容量 | 拒绝/失败终态持续占正文和活跃槽 | 仅有完整结算证明的终态压缩正文并释放活跃槽；保留身份、开始证据与费用，unknown/未结算/未送达回复不压缩 |
| Matrix 权限 | 旧格式字符串 power level 被默认低阈值替代 | 数字及十进制字符串统一解析；存在但非法的值拒绝授权；旧格式与非法值回归 |
| Agent 退役 | 前 20 条不可访问 scope 阻塞后续退休 | 独立复合分页游标越过失败 Room；PostgreSQL 阻塞前段的退休收敛回归 |
| Matrix 状态完整性 | Palpo 返回的状态可能遗漏无法读取的事件 | 同固定 frame 的 IDs 与完整事件集合一致性校验；缺失、重复、错误 frame 回归，失败拒绝授权 |

## 验证和边界

最终客户端严格 workspace/all-targets Clippy、格式及 diff 检查通过；lib **132 passed、0 failed、1 ignored**，登录 HTTP **5 passed、0 failed、2 ignored**，本地账本 **51/51**，登录和 Owner Console 两套真实 Chrome 浏览器检查通过。最终实际 binary 的静态包服务专项（启用 native-console-browser feature）**1 passed、0 failed**，日志 `.run/manual-owner-audit-final-rail-20261007.log`。规格库存 **1187 当前绑定、0 missing、0 deferred、2 superseded、82 retired**；检查器 **32/32**，生产入口 **4/4 wired**。完整工作区的此前 1917 passed 是上一轮结果，本次按修补范围重新验证，没有将其冒称为修补后的完整重跑。

登录套件首次运行有一项旧断言失败：已验证的无本机 cookie 重新登录现在会撤销 bootstrap 授权，持久会话数量由原预期 1 改为 0。这是重新授权路径的明确行为，已同步断言，最终完整登录套件通过；没有更改权限要求以绕过失败。

客户端日志：`.run/manual-owner-audit-final-lib-20261007.log`、`.run/manual-owner-reauth-login-suite-final-20261007.log`、`.run/manual-owner-audit-final-clippy-20261007.log`、`.run/manual-owner-audit-final-spec-20261007.log`、`.run/manual-owner-audit-gates-20261007.log`。账本专项原始日志 `/tmp/hagency-local-audit-fix-tests.log` 与 `/tmp/hagency-local-audit-fix-clippy.log`；IPv6/marker 聚焦回归 7/7、严格 lib Clippy 由复审代理执行记录确认。

服务端 PostgreSQL **41/41**、backend 专项 **2/2**、全目标 check/严格 Clippy、OpenAPI **46+4** 和 **8 项自检**、diff 检查通过；原始输出在复审代理工具记录 session 53188，未另存日志，不引用客户端日志作为服务端证据。Docker 构建 exit 0，私有日志 `.run/server-audit-docker-build-w1va7bwh.log`，构建时源码指纹 `87bed3952cea3b26a2afeee7e6c1ae87135019aba73651a8f1a4d547fc712c34`。

客户端静态包 `.run/manual-owner-console-reauth-20261007` 和重新构建的 binary 已在 13300 更新；服务端新镜像已在 8089 重建容器，数据库与数据卷保留。两端 ready 成功，服务端确认必装 AS startup roundtrip。实际浏览器在客户端重启后的旧会话上，按钮可用、没有红色过期警告，并进入 `8089/_pasion/login` 用户名/密码页，随后返回客户端供手测；截图 `.run/manual-owner-reauth-20261007.jpg`。

本轮不调用付费 Codex 模型，不宣称完成真实付费任务或生产整机恢复验收；加密 Agent Room 仍按批准方案延后。有限容量继续统计保留的不可变证明；压缩确定终态不等于永久无限队列。服务端事实观察存在最长约 30 秒的窗口，执行历史全量检查仍有 O(N) 扫描和全局锁成本；数字字符串含额外空白保守拒绝。没有发现新的 owner 转让、跨账号使用账本/provider、服务器审批本地资源或旧 Fleet 回到生产入口路径；这些是审查结论，不是对未来代码的无缺陷保证。

## 后续界面重整复审

新增复审覆盖 OwnerRail/OwnerAccountMenu、ProjectsControl/OwnerProjectControl、owner-projects request helpers、Agent 页面拆分、原生资产白名单/CLI fixture/发布打包、Chrome与CI门槛，以及server Project/Room metadata和客户端OAuth Space候选。设计与完整验证见实施文档“工作区导航、账号菜单与 Project 创建重整”。

额外修复了 Matrix 命令404后的误清除（保留原id/input，只同ID重试）、组件卸载后迟到mutation响应导航旧Project（mounted/generation fencing）、恢复读取卸载后清记录（alive条件）。Chrome回归实际模拟表单卸载和迟到response，原记录仍保留且无错误导航。Agent创建跨刷新幂等恢复仍是记录的产品边界，没有把Matrix命令保证泛化到所有创建。

本轮最终四套浏览器门槛通过，真实已登录UI核对通过；client lib134 passed/0 failed/1 ignored、strict all-targets Clippy、真实binary静态资产专项1 passed、CLI13/13、32项checker通过。最终规格库存1188条、0 missing/0 deferred、2 superseded/82 retired，日志 `.run/manual-owner-projects-final-spec-20261007.log`。server PG41/41、metadata/member隔离/名称变更、OpenAPI和Clippy通过；Space候选2unit+1真实OAuth HTTP通过。两端已使用本轮构建更新，ready与ASroundtrip通过。未运行模型或另建手测项目。完整原始日志与构建指纹记于实施文档。


## 变更库存

以下包含所有 Git 已跟踪修改、删除和未跟踪新增文件，SHA-256 固定文件内容；删除标为 deleted。生成文件、私有 .run 运行数据不作为源码提交。此库存用于查遗漏，不能替代上面的语义审查。

### hagency-client (170 files)

| Status | Path | SHA-256 |
|---|---|---|
| ` M` | `.github/workflows/ci.yml` | `6f2d44a42bc52beb42f811e7ea6f826c1a1e3d0d60b82ce2a300bfb1c62d5eef` |
| ` M` | `.github/workflows/release-native.yml` | `fbb35b2b5dc0bfe0fe86119e805d151a280d4aeaf312fe9baa9dbfbf1a758ac9` |
| ` M` | `.github/workflows/rust.yml` | `3e750aad20b9adc25c703fd58b8adff81e2cc8e64765dce709fda948545c2071` |
| ` M` | `Cargo.lock` | `d36f63d463dc0c0b1ee7476db8f90a2a3408056f33cff7fd526d494a61618e6e` |
| ` M` | `Cargo.toml` | `8e8dfd2d5122a417866153c588f765321464868e1e08437e32647200b27f804f` |
| ` M` | `README.md` | `89688659fc32eccbe50fb630b4218add46f51766a5ea7c9adaa10d7ed212e36d` |
| ` M` | `README.zh-CN.md` | `c50540518f5fb7c32dba0cf986e4a0693be9eb5d18a9c10bd5736ae2945d994c` |
| ` M` | `deploy/hagency-native.service` | `3c2f79065f915fbb6015eecd129aec55dea3a30621a9d6428ca14f4b230823b4` |
| ` M` | `deploy/io.hagency.native.plist` | `ea4c5e178b070350f1b4df5010a9b9781e7dfb6847e7c7e27802aed927e6118a` |
| ` M` | `docs/design/2026-10-07-server-appservice-client-refactor.zh-CN.md` | `61ee28bdd7a6007c8de6a6551657183e280ed3e7b3d4e5ba6fa779e265be9a46` |
| ` M` | `install/install-native.sh` | `cfcebfdca8b900c549d5c3792c76885520277681dc4d1140fe2e6b7210712e55` |
| ` M` | `mockup/app/globals.css` | `c2cbfca07fa4037065edb2d9622f5a122ed4f7f7460a6a978f519b310741ee2b` |
| ` M` | `mockup/app/project-sides/import-palpo.jsx` | `2e078fd58ec50e83b331615214cbeaef5be713edf5903cb1854b78e48a26e6ec` |
| ` M` | `mockup/app/projects/new/page.jsx` | `ade26e6da320f5fdb250bef57dc07ba00f736856961a0733b25230301c99e6e0` |
| ` M` | `mockup/app/projects/page.jsx` | `fa9109718a2eaf77f582883985308f6992dfcc4a211869872d1371afbf4d6018` |
| ` M` | `mockup/components/Rail.jsx` | `151f4b8540004f8d8e4f5ba1dbf56378e0e7616447808b368f404ba38661e4bf` |
| ` M` | `mockup/components/ServerLoginControl.jsx` | `0b5a12a07afcf37db9f8a35013b7a24be1b008420a252de529d93d206af9e049` |
| ` M` | `mockup/lib/i18n.js` | `b6267ff8077566cd551714cc3ff5de2d8f5722160e3978dcada46ac4a25591a9` |
| ` M` | `mockup/scripts/build-native-console.mjs` | `5edecea97fb23909a3e37d93410130d078b9e78f0b015d7fade535fe764e979b` |
| ` M` | `native/hagency/Cargo.toml` | `84aabcda23815416f60fcba9c468e5e30823edd985658e47743f0f7eb12d9c78` |
| ` M` | `native/hagency/src/console.rs` | `5d4422f1380856c10e8d2c58fde60a059b90c626e9a63c4cd9aa4ab0b1adeffd` |
| ` M` | `native/hagency/src/console/assets.rs` | `d58e24b65274a1622ebd86d4401938d9c6300da6f7957e01b03498cf390dd4c2` |
| ` M` | `native/hagency/src/console/authority.rs` | `f3ea9e99b6ad2e9f942e4c94a1f650f639cdc9e1d2fd19ee129f14ca31b1304a` |
| ` M` | `native/hagency/src/console/server_login.rs` | `75a387e114401cfeae118cbd292def555257e414a274ba8d7f649ee35233ab0b` |
| ` M` | `native/hagency/src/lib.rs` | `d8797d72093f844b0825aeca8dcd2e2577a5134f4cdc68d04995a11bff082607` |
| ` M` | `native/hagency/src/main.rs` | `354c91152c3e330703ba620fe769f9ce66bebeda464df5322a0ff7a055d0f4c2` |
| ` M` | `native/hagency/src/service.rs` | `d7216b7a388cebfcddd8eefeac9e6a0bfd61a4509f24e625308412a0a103bb84` |
| ` M` | `native/hagency/tests/bootstrap.rs` | `839c8f6e57fe54f43fe6c496872b14da6335ef73b1fe58af50472d23f4db9c72` |
| ` M` | `native/hagency/tests/bootstrap/accounts.rs` | `e4fc8c14f6ffe00e8c10475d8f08db57e3a09756237396d650e550d92422d812` |
| ` D` | `native/hagency/tests/bootstrap/approval.rs` | `deleted` |
| ` M` | `native/hagency/tests/bootstrap/fixture.rs` | `2bccf2e4c1858244968a2c589654a28e22269c0844f7c0d56643147c6e0de7fe` |
| ` M` | `native/hagency/tests/bootstrap/scope.rs` | `04df328abd3886e47bedf290e33c0f6e75581cd9f8061797026c36c914feef80` |
| ` M` | `native/hagency/tests/cli.rs` | `fdc629976fadb585677946410f27784ab907f421b208f4907b76001f1ffc2fcc` |
| ` D` | `native/hagency/tests/configured_fleet.rs` | `deleted` |
| ` D` | `native/hagency/tests/configured_fleet/mod.rs` | `deleted` |
| ` M` | `native/hagency/tests/console.rs` | `4dfc8b42e4c9234de9667f8809799dd01a1bea9fc590c69816bdcbb36a663a97` |
| ` D` | `native/hagency/tests/console/browser.rs` | `deleted` |
| ` D` | `native/hagency/tests/console/live_actions.rs` | `deleted` |
| ` M` | `native/hagency/tests/console/rail.rs` | `382b328ee479f2355b38524d60f8f386f0a896f90f7928ca0baf329fd25fca18` |
| ` M` | `native/hagency/tests/console/server_login.rs` | `e3100e5594a98aad18c48807ef42c49a299a181f7565bfe6e583539f61e40436` |
| ` D` | `native/hagency/tests/console/status_strip.rs` | `deleted` |
| ` D` | `native/hagency/tests/file_service.rs` | `deleted` |
| ` D` | `native/hagency/tests/file_service/fixture.rs` | `deleted` |
| ` D` | `native/hagency/tests/file_service/recovery.rs` | `deleted` |
| ` M` | `native/hagency/tests/inline_factory.rs` | `103d63d77c45f7fcee38b8f73d72b0145bbe91a60b2f8c3dde4a0718c49cf2ff` |
| ` M` | `native/hagency/tests/inline_factory/mod.rs` | `34406d329d2012e7e6109d130453286af9dfc11230e1435a6a3b2da753233a4a` |
| ` D` | `native/hagency/tests/login.rs` | `deleted` |
| ` M` | `native/hagency/tests/mcp.rs` | `b7b515cf7abd7621151bf41d8004480de9154a6d774391f63edd6cd19762cefe` |
| ` M` | `native/hagency/tests/mcp_coordination/fixture.rs` | `798984001938673c2ab52684b711db7474441bf396ffb264b9bd9cbded9fb51e` |
| ` M` | `native/hagency/tests/ops.rs` | `8b575aed0fee979087321282430630086360744cb1617c223c46bf27ee42ee12` |
| ` M` | `native/hagency/tests/owned_matrix/support.rs` | `51b852f4c92a67f251683c8447e9c30d4ecfa4a0ebd9c5d5815e5cdb78bb5b38` |
| ` M` | `native/hagency/tests/owned_mcp.rs` | `309a0453c4d94181ff13121079a59a67eedf82801f40414396aec1ad5700d1ba` |
| ` M` | `native/hagency/tests/palpo_service.rs` | `3f9f67bc3c6f2d68d91da68b6d6e9bb556e87e74307e049edd7adbc1511ffea1` |
| ` M` | `native/hagency/tests/palpo_service/fixture.rs` | `baff02d8192a9ce99655823b13dd33f26aadea630696df101f12b58a154f5403` |
| ` D` | `native/hagency/tests/received_files.rs` | `deleted` |
| ` D` | `native/hagency/tests/received_files/fixture.rs` | `deleted` |
| ` D` | `native/hagency/tests/received_files/recovery.rs` | `deleted` |
| ` M` | `native/hagency/tests/release_cutover.rs` | `d87e6f1a7bb40482d5ff98906e5d27a256a7335b362a4c8f5a79251fc681132d` |
| ` M` | `native/hagency/tests/service_launchd.rs` | `1aa4f58854c45be9adaecf7353c1650fa1126073eef12bf1497328355e9a3949` |
| ` M` | `native/hagency/tests/service_linux.rs` | `f4af97cd6468095ca6087cd3d261544c1c984fffbe4c7a8d4d7d3df19f186cae` |
| ` M` | `native/hagency/tests/setup.rs` | `d9955548f876fcbec589ab9865519368109a366b23df8c613b1e618e70b44a6b` |
| ` M` | `native/hagency/tests/task_client.rs` | `5c2b4ea102881e506baba68d250232018bf3bd526d816f74542a246fb2eabb7e` |
| ` M` | `native/hagency/tests/task_client/mcp.rs` | `6e5c82c13b76992dabc033d61f873a6edce97e99e975ad63ec4e807890f2844d` |
| ` M` | `native/hagency/tests/warm_runtime.rs` | `1f04eb18445423fdcdd4436d1f994708893646fb1b97afd3571e7c2e53524a65` |
| ` M` | `native/scripts/check-production-callers.mjs` | `0516762ae4d9792ea1543462ea393f0af947b616266aa3d4b15f0641a4bdfe01` |
| ` M` | `native/scripts/check-production-callers.test.mjs` | `790fa0fe639d726369f1164058852003c99c0b80af01982d3dbea848bfc4d5a7` |
| ` M` | `native/scripts/check-spec-bindings.mjs` | `eb36620f746faa7a27b99ad1d577885fbc0354a1835ea2dc95ad2346ac7a3e88` |
| ` M` | `specs/task-rust-attempt-containment.spec.md` | `19ea8047db3029ecd848776ab0c8873901c03ae14dc6b04aaecbabdd5eb80ab2` |
| ` M` | `specs/task-rust-bridge-never-decides.spec.md` | `3c35e1ed563c823694be33d3a424f32b7f5ede1eec5d2bc9b58fabda1ed0b953` |
| ` M` | `specs/task-rust-codex-approvals.spec.md` | `addb662cd1c422dda2f2e7258e45babeb8c73b9eee91ac6cee3edf3219e199c3` |
| ` M` | `specs/task-rust-console-accounts.spec.md` | `591f5fc2f41d8331d8f24391abbfb08cbb7ff13b4d5d5db07b1ab9609833b266` |
| ` M` | `specs/task-rust-console-agent-lifecycle.spec.md` | `3cfaf78ec4e7f45a171a4fb93a50153c375fd6ee5fea55fae18e069e6ce16003` |
| ` M` | `specs/task-rust-console-agents.spec.md` | `33614c54226541935b4676adfc62856ecfda6e5cbb34f8429cb97cf1660f144c` |
| ` M` | `specs/task-rust-console-outcome-workflow.spec.md` | `315df40dc20e7ffadf3297629aae46516d12f68e28e540ed3f98d89b136575fe` |
| ` M` | `specs/task-rust-console-project-sides.spec.md` | `175eb9bdbcc870ed57911ba69dd6729d97b8eaa30ab97feeeeef53fc97d82f77` |
| ` M` | `specs/task-rust-console-readiness-fact.spec.md` | `398ae0b5fbdf220263f27b0ea227b57660bd866f8a38408ebfe262695f028195` |
| ` M` | `specs/task-rust-console-readiness.spec.md` | `9be5eec9db7fd12df0588b7853646b03bc554b5640fbd53198ce7acf9d26b6f3` |
| ` M` | `specs/task-rust-continuous-outcome-resolution.spec.md` | `d0477cd437b89b608e045ac943ce0a336d0f7cc66858389fc0eb7f9fe4f4fbb6` |
| ` M` | `specs/task-rust-delegated-task-delivery.spec.md` | `9d0da09243d46380f35c1d1c5115044b2c660c119f120a3713e58c0f7c5a239c` |
| ` M` | `specs/task-rust-development-bootstrap.spec.md` | `40b07a596aa58c7b7122af454d7d4cb37a277901826c5215b8cffdacd826b25f` |
| ` M` | `specs/task-rust-factory-failure-diagnostics.spec.md` | `d4f823533def88a2ded1a7f033e1e63f3a1bbc5cb68e3a9807f3c889420da27a` |
| ` M` | `specs/task-rust-factory-project-inbox.spec.md` | `92295c44b3da4649695063a70687bd7cfe8d62dfedd1bda72ea88d69366d66bd` |
| ` M` | `specs/task-rust-factory-room-generations.spec.md` | `e5cf292c3726d33291f8852ac3f97a6ba6071ed183ef97034cbd7c2cc13184a7` |
| ` M` | `specs/task-rust-file-owned-observation.spec.md` | `859db70281ef129b15d18ce281e14dedf82e60185b23f4cb702ba55c5d5e466d` |
| ` M` | `specs/task-rust-file-service-integration.spec.md` | `e24d6698f6c32a28212955824e445008d08ffc4be1efd2e6327c06f9fca605d5` |
| ` M` | `specs/task-rust-file-settlement-guidance.spec.md` | `0369d3b5642248056f7a10f5536eb07489a92bf2753a8188f33106b987bf2a76` |
| ` M` | `specs/task-rust-fleet-approval-service.spec.md` | `2c623dc08c971384b62fbbb5c7405d45506285faa954835f38c5b20b1246aff4` |
| ` M` | `specs/task-rust-fleet-executable.spec.md` | `19810c106aa5bc3efd55309314f118256ca1e91109e223b788b212f99a3dba7a` |
| ` M` | `specs/task-rust-fleet-media-executable.spec.md` | `b126717c16166679c2ae9ca3e7dc62887f89253abdb94229d6f2c7b51b2afd4b` |
| ` M` | `specs/task-rust-foundation.spec.md` | `9833d466dc7a188c868e10d2f2ee7b0d490e26c55bc37bc1a50ad513ef76dea2` |
| ` M` | `specs/task-rust-guardian.spec.md` | `c73a672a11f563bbdac888ffd0958e11a378e0e12604e84994c1ea34c855b64a` |
| ` M` | `specs/task-rust-inline-agent-home.spec.md` | `2b5c72d1e75e291bcb5447e140dbe2d5b1c754d94ddbdc1a6e8542da85c26d01` |
| ` M` | `specs/task-rust-inline-agent-rooms.spec.md` | `ee0ced660e907034dfe11db1aa38f1c5ee22bcd01021a6019c68ae6a8112775f` |
| ` M` | `specs/task-rust-inline-appservice-account.spec.md` | `4106148235d1b6f2765324e0ac2f9be0536966d61b2fa543dbdb52be4ce1419f` |
| ` M` | `specs/task-rust-inline-provision-account.spec.md` | `95ee9183996ea57cd437fc3a530d41d9352926f568adbc70ed7794364f44001e` |
| ` M` | `specs/task-rust-local-codex-factory.spec.md` | `90c2fc7f37e662234343c39c66b67422e42d86120ae8ae7b7cd8025545275f51` |
| ` M` | `specs/task-rust-managed-device-login.spec.md` | `c65cf3c401026e18bcf5c02fe5ccf085b1327401ea28024259f22dbf239c727f` |
| ` M` | `specs/task-rust-matrix-request-pacing.spec.md` | `08b31a0262128facbb133b778276db6ae7601712b84da4211e699d98777f083e` |
| ` M` | `specs/task-rust-matrix-sdk-budget.spec.md` | `7be84c66e27889693bfefd9a397be878f789c767674fb6d460b6d3ebe8f9942f` |
| ` M` | `specs/task-rust-matrix-trust-session-enrollment.spec.md` | `3b0fe0934cbb8ac6dd455ee97f6450969f7fc651e38d3a2ef77737f06219e4ed` |
| ` M` | `specs/task-rust-native-approval-roundtrip.spec.md` | `1aa30d89aa4eee9cb2c35c8cce139e013cc112bb482cb9e50f86d6f264374a8b` |
| ` M` | `specs/task-rust-native-console-resource-configuration.spec.md` | `d5e10649a9f5ca92bed3f0b33a78fc3e30311eb78455ca38d437dd8d855a8bf1` |
| ` M` | `specs/task-rust-native-console-resource-publication.spec.md` | `9675ef86e5d5a442f73310de0614457ec7cf79d078efc01b38d6625202042350` |
| ` M` | `specs/task-rust-native-console-usage.spec.md` | `bf7a0921b816a975d12c5c1774cf969fbdbf829667d36893184782ad04ae29b8` |
| ` M` | `specs/task-rust-native-file-completion-guard.spec.md` | `e7265673cc29fee426c6c2db87a7f6a00530dd1609f59e6eb0a21ef2f046b3ef` |
| ` M` | `specs/task-rust-native-managed-account-binding.spec.md` | `9ea1accd31c3731117405039745343b1017bb2520c57cb320d1762b18e7f19ef` |
| ` M` | `specs/task-rust-native-package-scan.spec.md` | `d443893a0e68ff4c77db33b27245b3991db031d6912ff90937b4919635558855` |
| ` M` | `specs/task-rust-native-palpo-service.spec.md` | `23249826a471f3605123a23a9aade8f49d9b2b78289011e1a818d3e03784d1de` |
| ` M` | `specs/task-rust-native-upgrade.spec.md` | `a8f4b3c7ba0630b983b5b2f43d04500fec957da7460ce027826f1e8468231281` |
| ` M` | `specs/task-rust-owned-attempt-evidence.spec.md` | `4ef31ebcc2380197dbaa78dd9062533cd24078f31c328e6d0bce4f2f418047f6` |
| ` M` | `specs/task-rust-private-approval-delivery-fixture.spec.md` | `3e28e641ffa0c9dff12bab33f6013bcd37362d38abbb0343126dc5a4cf2970d8` |
| ` M` | `specs/task-rust-private-approval-wiring.spec.md` | `0945dc63c8bdfa63a3e30cecf2f6b8d240e071a9ba4f11b0e35d1fdac6d3db0b` |
| ` M` | `specs/task-rust-private-directory-creation.spec.md` | `07ed54bd3dea392cf5679c2e4929653f106de52b419aee464220db38fabc3663` |
| ` M` | `specs/task-rust-receive-executable.spec.md` | `52676d0bfef4154d28fcfbd66d877462259f97e6003d49176b7305269b9cae46` |
| ` M` | `specs/task-rust-sha256-dev-profile.spec.md` | `4f7c7662fc7f099403319ae2800e6aeee61d88cf6409d2108c6df3ae81af107d` |
| ` M` | `specs/task-rust-startup-boundary-observation.spec.md` | `a6ae914bb0f71b6859a1cde5c8545054aade7536aac156f8ec14606b3fac1070` |
| ` M` | `specs/task-rust-task-client.spec.md` | `e310d4da031ceffa4853e3c19fba13080c4f429a3a0239638696cac806b52d96` |
| `??` | `docs/design/2026-10-07-agent-architecture-adr.zh-CN.md` | `3ec53fd9fe5d0f474cf8e7270e01460014dcbbd8dd97e99d4da7d360f59fa6a5` |
| `??` | `docs/design/2026-10-07-agent-data-protocol.zh-CN.md` | `a532efb56cbaa23e90fc7dce9c77fb8fa19c1a48003fc36338249aaaf45c6ea9` |
| `??` | `docs/design/2026-10-07-retired-product-test-coverage.zh-CN.md` | `9eeb1bdf85bf121a6615292d79fd7ac35f9cf7c9ae14467a81508424161237a5` |
| `??` | `mockup/app/agents-owned/page.jsx` | `1eca0269c15366563cebf46d5d636b3d4643270d8d40c8cd96984626b6f2b016` |
| `??` | `mockup/app/login/page.jsx` | `7dd24b07f0583782a1443112e0910f1de84e6d0c3ee8babeb3301ccf69955605` |
| `??` | `mockup/components/OwnedAgentControl.jsx` | `d7362cc384122b30870807f63a5a06e46325185fa811e87e903d46296e584b0f` |
| `??` | `mockup/components/OwnerAccessGate.jsx` | `60bf981c7853115e5687e25127ca542e48e22c56a4a08666acd3086bd9545a32` |
| `??` | `mockup/components/OwnerAccountMenu.jsx` | `9c77539067f654efa877260d467394c8bf302a91b4c33aa611e8bf99fff0a043` |
| `??` | `mockup/components/OwnerProjectControl.jsx` | `4825811c722a77ad3c17a2db953ef29285a8e318d24f7e0345274d48f9b86388` |
| `??` | `mockup/components/OwnerRail.jsx` | `0007f7b2cbe4da51b06a9644d00a2c8990d50b1a16add64ce735c7e6abdbdbf9` |
| `??` | `mockup/components/OwnerRuntimeControl.jsx` | `696bab343bff427198bd92c7ad7b22bb45506c52a7a0371af35d185a3de09943` |
| `??` | `mockup/components/OwnerShell.jsx` | `4ca279941227b64112bb7d321f1db30ba4db8e0ab0e259d56a06b2bbaabae3ef` |
| `??` | `mockup/components/ProjectsControl.jsx` | `d11948ba8a0697139b4af9a5d72b209373bc90c37798b7c76993e2a0616d2771` |
| `??` | `mockup/lib/owner-projects.js` | `fe9a8fc8b7763c13e0e1d3a4c3f1780c38bdda4df6cfabd4d1ff8b8824f370c4` |
| `??` | `mockup/native-owner/layout.jsx` | `0658494b2d16bce8de4a1b0d179f291c9261bcd8cb38aa683e174f2d557da8a0` |
| `??` | `mockup/native-owner/page.jsx` | `cad62486b34a7b5a95108dff5f31f8874270fd1c3148a54ed444ed7d4ca74398` |
| `??` | `mockup/scripts/check-owner-account-menu.mjs` | `d2035b2d59b987844d4262d611cd5a234465c72d8dbdb712822c728197c074b1` |
| `??` | `mockup/scripts/check-owner-console.mjs` | `28ee22e763dae415a0a28787bea157c34e2b6a49b6b7191651cb4638d518e173` |
| `??` | `mockup/scripts/check-owner-login.mjs` | `9e7b46965a26136cf549fd7bc4ee30301e63540b183a4c63f78afd804df1c2ff` |
| `??` | `mockup/scripts/check-owner-projects.mjs` | `5dc209237d5df75567a4916356af8722f3a37b9d627959117a023814614624fb` |
| `??` | `native/hagency-agent-local/Cargo.toml` | `3757b06697f215a4a883e6de097a4525fc9e6edf6f885f043b8b31106e64a466` |
| `??` | `native/hagency-agent-local/README.md` | `5fd9578c32376b6a231bdda02925a4502a79bd5dca226a0a7b35eb4789181c38` |
| `??` | `native/hagency-agent-local/examples/codex_handshake.rs` | `772db6c084695b84c9c067e3a996120957e313e2b33da700f29d4d4ec5036b78` |
| `??` | `native/hagency-agent-local/examples/codex_host_files_handshake.rs` | `681688272bdbe47b39cc06f0961cb56370123c511d3ba1d422d30f2e9a91d0a0` |
| `??` | `native/hagency-agent-local/src/codex.rs` | `6c88449457c828a7cb4d2882d02fc900820882e0ed5cafb03143d7d079e5c3b2` |
| `??` | `native/hagency-agent-local/src/codex/host_files.rs` | `d14a57a55faba25c88dfe10da68d9d7340273468184ee0992d7ac3a47d00d139` |
| `??` | `native/hagency-agent-local/src/inbox.rs` | `aac3dbd8ddc3858160c1ba6cc4477dbe70739f356c1bf86f419ca0f4dc9c9032` |
| `??` | `native/hagency-agent-local/src/inbox/tests.rs` | `f02d95986d583321edc9f73ca525546c86ae096682c97af90cd9edbc4af8ad2c` |
| `??` | `native/hagency-agent-local/src/lib.rs` | `a8d2a8156609380cb4fa4db6a06a334f12e3613f9ac528e71277f7d424fd7540` |
| `??` | `native/hagency-agent-local/src/room_files.rs` | `16b8a51db4538d7d0261cde23637a0ea05707a82aab9adfd5744217b7a3f0fcf` |
| `??` | `native/hagency-agent-local/src/room_files/tests.rs` | `7ac45fb1734bf495edcdf6285121b8d5aac2efd78d7b71b869da20f60626e4f8` |
| `??` | `native/hagency-agent-local/src/schema.sql` | `ca2206370aafafc1958ceccf0d2944591440c1eb3f5e7a33f5631cedbc4cde37` |
| `??` | `native/hagency-agent-local/src/tests.rs` | `13a66aaa307d296060f95c28ee5cc9815d5f67e637dbcdad0d637811251852c9` |
| `??` | `native/hagency/README.md` | `92b0b75cc0aee43620dd06b2b1b24a230784308f8370c2e2fd11359dc8d6f925` |
| `??` | `native/hagency/README.zh-CN.md` | `408b2026a590589a78e346c5f50db222231b2869ac631db2e08527786fbb0914` |
| `??` | `native/hagency/src/console/device_execution.rs` | `6d3ce815c09460f7e7fcfa7d2fafb90e988eda710552bd4e09e6ee2c412d44b2` |
| `??` | `native/hagency/src/console/owned_agents.rs` | `f591d56a1db1e518df8e5b1a0bcca31d26efc65a8b2e171b8f47044412990395` |
| `??` | `native/hagency/src/console/owned_runtime.rs` | `3f1af92e86e0c9179f5e20f3ff5eee4ebb555073cfb0d52468f395c67a3683a2` |
| `??` | `native/hagency/src/console/owner_projects.rs` | `b928871289a4826b746f35fc5d2287f880899624fca07f77d96966e5dfe8f28d` |
| `??` | `native/hagency/src/console/owner_provider.rs` | `ee487cf8467284d7d19bf52dacf486a0be776e1b0aa81590ac56150a2e54db15` |
| `??` | `native/hagency/src/console/server_login/matrix_creations.rs` | `a3839a2a6d86e56bae35111e3d5eb1fd09f4b3b2a7474305f71a8104077f0eb3` |
| `??` | `native/hagency/src/console/server_login/matrix_creations/tests.rs` | `edf2abd9f9b304af7134acfb776f8ec089cd4a6531e0243933bc4124f8db9a10` |
| `??` | `native/hagency/src/console/server_login/profiles.rs` | `46df2258c37170b6e42d38f93198729837a7264d8847ae8bcd9fc97c9a6470cf` |
| `??` | `native/hagency/src/owner_host.rs` | `7da633a24c7c798cdf31e24c231ee06118790209161b7f1e73779b6d8829a5ab` |
| `??` | `native/hagency/tests/fixtures/sdk_mcp_test_peer.rs` | `b959edfb0a16bb1bf5d796d37367a957e2569c82d1dd5cafeaef24eece943b9d` |
| `??` | `native/hagency/tests/owner_cli/mod.rs` | `ab5997647eddafde741a3a5912735ef25d5e3b734e171bb26da1d1b43074a49d` |
| `??` | `native/hagency/tests/release_state/mod.rs` | `716bb0e12fbcc01b0bdaefbfa4fce9e34b92bc72c3433ebed054fa32dc8d3c53` |
| `??` | `native/scripts/check-spec-bindings.test.mjs` | `8eba74b75e5863b6d6b154c8de9f4a2f25f719a54f5506840999c9268e6c14b7` |
| `??` | `specs/native-bootstrap-sdk.spec.md` | `f9ac46c5d61037b8904e5463ce85ead8fb52db93ce981a6bc7f95c7dee5d2c20` |
| `??` | `specs/native-owner-client.spec.md` | `32c15ed209f29d2ab2c493860a8ddd36cc6a25100b36887e640552a21aceff1e` |
| `??` | `specs/native-setup-sdk.spec.md` | `d203ec400519d8c052e96b4f8906eb490d58591a7565354c5bc6e6cd0ae5b2f9` |
| `??` | `specs/owned-ledger-continuity.spec.md` | `13069fdc0b739217fcaaf685994f7ad5996643378c19b7b1fa19d25c66de5266` |
| `??` | `specs/task-owner-profile-isolation.spec.md` | `78ab1a817bfa72d8de4d1b1ded77ee02b2ffa9aa6be1a2004ea6df1c721c1be3` |

### hagency-server (141 files)

| Status | Path | SHA-256 |
|---|---|---|
| ` M` | `Cargo.lock` | `a9ee558e7be887616b762aa1b8a61e43d1f922921ef4ab596c0948f37cd6215e` |
| ` M` | `Cargo.toml` | `0408ba045a6a3ee4c47e8e259d90c749f101a104437c60f9dffd5ef9edfdc58f` |
| ` M` | `Dockerfile` | `9ac024f8a430cf89fc5b7505fb54f95b0b221d32c2b6a848df436bc01967b41b` |
| ` M` | `README.md` | `747ef94bea757e77b5c5cc8767fddd76c498d93710c0d9fc67b494f678e61896` |
| ` M` | `README.zh-CN.md` | `555edb663cd515c4ebf2fa0d9e2c6bde8a4444fffcf4e820926c84b1180edbd3` |
| ` M` | `compose.yaml` | `b8fcc65f17aed4c0e297b5e8544fa09f679e9da7a9bf9e6284f88282b9bce876` |
| ` M` | `config/examples/hagency.toml` | `b7c7588d41261733e0abcda99da9276f006c17dedace9368887c3e9fe55b0baf` |
| ` M` | `config/examples/pasion.toml` | `c685a8ec441b704f4fb1ec81576775b5e23362468285d01653b5612e720c22b6` |
| ` M` | `crates/backend/Cargo.toml` | `dd00e063b071b30fefcf6032298e37809efbde48f8372fea157e28efe08fdda4` |
| ` D` | `crates/backend/examples/admin_contract_server.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/accounts.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/api.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/fleet.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/mod.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/native_client.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/outbound.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/store.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/upstream.rs` | `deleted` |
| ` D` | `crates/backend/src/admin/workflow.rs` | `deleted` |
| ` M` | `crates/backend/src/config.rs` | `e7f62c4327e46147db0ac7a62dff82d7d10d48f44d6492ea10b8af624da32adb` |
| ` M` | `crates/backend/src/frontend.rs` | `fb50de5aecdfdd27859791cbef786f7ed1b6f2a2b668ac4f5adf876f31234992` |
| ` M` | `crates/backend/src/lib.rs` | `49f4c1bafdfddcb1a5dfc7c4f0b5c612e31689ff8c6d6dc9f97c673867bdf660` |
| ` M` | `crates/backend/src/main.rs` | `a4afc437c157f12dae434051b389a96f433986677eeb7b8a6c66757efce8b917` |
| ` M` | `crates/backend/src/pasion.rs` | `591a5ce661a1afb7843674796d05b7a1404d3d6326e6d84146d3ab97316ea254` |
| ` M` | `crates/backend/tests/config.rs` | `8a7f551e26af157127cad2e1c140f96b0f29f6f2ad960e7c249258b11ad02d9f` |
| ` M` | `crates/frontend/README.md` | `298cde4541be3ec2a90c0e931bdc3315aa167412959a01d18ee7de0ea8ceaa5b` |
| ` M` | `crates/frontend/README.zh-CN.md` | `739932b78cfa9c3abc2ab194c7f5546306232a47c9caae49f543a33b17d10e59` |
| ` M` | `crates/frontend/src/api/auth.rs` | `cd1382d3476f7f5ae8d7c6a81980b4c25bc4376fa04c3e4eda932ef92989d389` |
| ` D` | `crates/frontend/src/api/hagency.rs` | `deleted` |
| ` M` | `crates/frontend/src/api/mod.rs` | `813edcb0fe9455ea678ad3d5fa534fa92b74ad87ba766715d3b2530c7b8542b2` |
| ` M` | `crates/frontend/src/components/sidebar.rs` | `289e87a5291f8066c40295f53e641f95d6ac15c50acf9a49f7adbad3b6fafdad` |
| ` M` | `crates/frontend/src/main.rs` | `f44a281eba7f933eeabe71e8c9fcaf80386696c46d16773c9897b268e2ba4b9c` |
| ` D` | `crates/frontend/src/pages/hagency/accounts.rs` | `deleted` |
| ` M` | `crates/frontend/src/pages/hagency/common.rs` | `c521da88f0d021abc5c400008621bf455e9cb0bd064d1c6142ddb346c64ea488` |
| ` D` | `crates/frontend/src/pages/hagency/connections.rs` | `deleted` |
| ` D` | `crates/frontend/src/pages/hagency/inbox.rs` | `deleted` |
| ` M` | `crates/frontend/src/pages/hagency/mod.rs` | `a93257a1d05a08b33187f0cb724e5301834b34a5b6737dc15767b2cd7f8a3e20` |
| ` M` | `crates/frontend/src/pages/hagency/projects.rs` | `a892dc16c6c24559877731b451d4d62c1d234e4c03ec82b5b5191de07dec75e9` |
| ` D` | `crates/frontend/src/pages/hagency/requests.rs` | `deleted` |
| ` M` | `crates/frontend/src/pages/login.rs` | `388ecc5c6df53f5c8afe926328748dcb302fc74a40ae912a28169fd4f53ef90e` |
| ` M` | `crates/frontend/src/router.rs` | `8501d4050fe75570f53978bb4ed7c3f111c393f6eab7a8d378e5befc3fb964de` |
| ` M` | `crates/frontend/src/utils/config.rs` | `604bf223d8bc01a80f0a6bc80e4ab99bf5b2832e00958c99209b485b2f8da9b3` |
| ` M` | `crates/frontend/src/utils/storage.rs` | `4ae0bed71b3c17d9f1051dfe7e6a009b2037e0e8737e3c6e57715f76aec2aae9` |
| ` D` | `crates/hagency-contract/Cargo.toml` | `deleted` |
| ` D` | `crates/hagency-contract/README.md` | `deleted` |
| ` D` | `crates/hagency-contract/README.zh-CN.md` | `deleted` |
| ` D` | `crates/hagency-contract/src/budget.rs` | `deleted` |
| ` D` | `crates/hagency-contract/src/canonical.rs` | `deleted` |
| ` D` | `crates/hagency-contract/src/ids.rs` | `deleted` |
| ` D` | `crates/hagency-contract/src/lib.rs` | `deleted` |
| ` D` | `crates/hagency-contract/src/policy.rs` | `deleted` |
| ` D` | `crates/hagency-contract/tests/coordinator.rs` | `deleted` |
| ` D` | `crates/operations/Cargo.toml` | `deleted` |
| ` D` | `crates/operations/README.md` | `deleted` |
| ` D` | `crates/operations/README.zh-CN.md` | `deleted` |
| ` D` | `crates/operations/src/api.rs` | `deleted` |
| ` D` | `crates/operations/src/intents.rs` | `deleted` |
| ` D` | `crates/operations/src/lib.rs` | `deleted` |
| ` D` | `crates/operations/src/machine.rs` | `deleted` |
| ` D` | `crates/operations/src/matrix.rs` | `deleted` |
| ` D` | `crates/operations/src/notifications.rs` | `deleted` |
| ` D` | `crates/operations/src/outbound.rs` | `deleted` |
| ` D` | `crates/operations/src/store.rs` | `deleted` |
| ` D` | `crates/operations/src/updates.rs` | `deleted` |
| ` D` | `crates/operations/src/views.rs` | `deleted` |
| ` D` | `crates/operations/src/workflow.rs` | `deleted` |
| ` D` | `crates/operations/tests/notifications.rs` | `deleted` |
| ` D` | `crates/operations/tests/workflows.rs` | `deleted` |
| ` M` | `docs/OPERATIONS.md` | `b930bb1afb0e5adeab24bf45b8384af59f77cd70b5fd0f25df335e9d5f9d78da` |
| ` M` | `docs/OPERATIONS.zh-CN.md` | `a233cf5ffd2435eb8ca68971629c0b584f3d04170412317461438fe4b6428888` |
| ` M` | `docs/README.md` | `e730ce0afe8dcdf409e535a70c319f46b028a231d10da2ab3af09abddc01635f` |
| ` M` | `docs/README.zh-CN.md` | `031548233e371a8aa5410a59a7d6330f30765bd46513a6567b48be08e3ef6048` |
| ` M` | `docs/VALIDATION.md` | `05d0623e63cf941e6795575ff71ceb30a31e83de1771def8c59f6b17d92378aa` |
| ` M` | `docs/VALIDATION.zh-CN.md` | `74dc34ed0ce82c832142b87623a51ae312ccd459bb5ac7dff3f73346a52940cf` |
| ` M` | `docs/WEB_ADMIN_PARITY.md` | `a638294425a6aa7bba155f1900033f11035b39e9046ab8a7542fed1303eb521b` |
| ` M` | `docs/WEB_ADMIN_PARITY.zh-CN.md` | `7f680e8262172e75a4de75dbecb6602c75172d1bbc9679782ee85f8eb525101b` |
| ` M` | `docs/guide.md` | `f2370faedad3657379c5c828b1fd87a104d053bff5e10f5f22a0109a1797627f` |
| ` M` | `docs/guide.zh-CN.md` | `e20f35e4a6ffb6db23b70d0a3ae42e5b5b7ed539eb12e067e13bbcaa62b1f352` |
| ` M` | `justfile` | `a091d2f77b9066ab81b760d8b006cd88f713fcb79dcb4a47990d47778a7f7c8c` |
| ` M` | `scripts/docker-entrypoint.sh` | `0fb52fe6a49f2c00a19bae77baed362c7ceae43c18296a216034842cb6e8e0ce` |
| ` D` | `tests/client-enrollment.e2e.mjs` | `deleted` |
| ` M` | `tests/docker-smoke.mjs` | `465c67eb37041c407d1e14f1cfb25cd6bb377e902a1bdf6bdc40408b215ba68d` |
| ` D` | `tests/fixtures/palpo.mjs` | `deleted` |
| ` D` | `tests/http-contract.mjs` | `deleted` |
| ` D` | `tests/integration.mjs` | `deleted` |
| ` D` | `tests/native-client-contract.mjs` | `deleted` |
| ` M` | `tests/pasion-integration.mjs` | `3da2bd769ca4a132ccb4356fa135a1bfa5adf9b16d9e647350bba0df593b1291` |
| ` M` | `xtask/src/dev.rs` | `d2176854e1531d6cc67dc0f64a5439627c4fc67376edfcc67494b795b4536755` |
| `??` | `crates/agent-service/Cargo.toml` | `9d9faa0ae584841db38d8f94359e74b01cc1d525d883a622efe031b501af240b` |
| `??` | `crates/agent-service/legacy-cutover.md` | `90a614928353071afbe196532e5331b91940cd3458b44eabff6e32b0162eadde` |
| `??` | `crates/agent-service/openapi/README.zh-CN.md` | `1b9f0c1ccf85c88b618222a555b91a9a46de72fccf5885fdedb4386d7a8e511f` |
| `??` | `crates/agent-service/openapi/appservice-v1.openapi.json` | `82e0e9569426a06242fe679586bb799b62f0ec576a8392edb764b60846848511` |
| `??` | `crates/agent-service/openapi/hagency-v1.openapi.json` | `ebec1870610f276df1e8ada93bcd00c29ab9e85a7ec6fa90b0a6a7d1fd32a90f` |
| `??` | `crates/agent-service/src/api.rs` | `b48e6c4ac37c3834e1e507ee74e059008fee4e1e636773f3afde0459ec0ea809` |
| `??` | `crates/agent-service/src/api_discovery_tests.rs` | `d1f5de294acde428d9be142ced901b122a025cb67aec190d488dfb290a21fa71` |
| `??` | `crates/agent-service/src/api_transport.rs` | `c8823ecab631d2cc1acab33236442490a2239eb53097f3d7962737fb6f5dab16` |
| `??` | `crates/agent-service/src/appservice.rs` | `c5d9a3bba1bb4e3b3dcc0624cb5acd19751f0a9a61cf18e38fd1d1f3a282892f` |
| `??` | `crates/agent-service/src/domain.rs` | `b0f0e4d6a929615976cdaf122cc520f257d628db153daac995108f0f73f274fd` |
| `??` | `crates/agent-service/src/domain_discovery.rs` | `be604e13c41eaa94fa4dfb80382e4f38d4b785a55c75b0c91690e6b5b83c1482` |
| `??` | `crates/agent-service/src/domain_schema.sql` | `226a33b10ae394ca96a7620c1d60be01df90cc4eb67a782e7c2c76cc82311775` |
| `??` | `crates/agent-service/src/domain_tests.rs` | `c1aa4f37dbb768df23b0773d56beb68196f02fef71a211c04a7f369b4e40915a` |
| `??` | `crates/agent-service/src/gateway.rs` | `e980fc58aa2fdaaa8b285eeaacd60ff32c20765b29a06c1f5a02a82a064cfe35` |
| `??` | `crates/agent-service/src/identity.rs` | `e1653e769079f24fe9f67525346795c13d705640742ca08aa7f237434e151072` |
| `??` | `crates/agent-service/src/lib.rs` | `e24e452eaefeba0c767ff8d7453bf4128b5f46844825a952dbb8972d7bf9c1f6` |
| `??` | `crates/agent-service/src/matrix_client.rs` | `ee83a147a31d7087fcef99128a94dfd440e45f7cfaec83e797ed61d97770286a` |
| `??` | `crates/agent-service/src/schema.sql` | `502116a37f5338b09d18720339b691d0efc4ee72eee8423475e62e37391ea271` |
| `??` | `crates/agent-service/src/store.rs` | `152709294b7736c555eadc33f29bc7c7c0a0b10b40c95bdd5985a3f9efff1872` |
| `??` | `crates/agent-service/src/transport-README.md` | `760edae32eb14bc916888099a5dfe5a928868fe57d587702889079d9be24a817` |
| `??` | `crates/agent-service/src/transport.rs` | `f216c54ad1e8ecebd342a8b9638792f12aed5bb3cef2be471e64bc598ae2b42e` |
| `??` | `crates/agent-service/src/transport_history.rs` | `b7f48bb284938bae50da7988fb569ea5e891749e41792f8eb4033b734d9d328c` |
| `??` | `crates/agent-service/src/transport_history_tests.rs` | `5cbccaad3356853af869099055d65982dee3a38280472fa0f04e64182959cd96` |
| `??` | `crates/agent-service/src/transport_schema.sql` | `478b020f711e5d97416a4a9a1884bc7ce8bc2715f32d8c558690103091a5a28d` |
| `??` | `crates/agent-service/src/transport_tests.rs` | `69b4de74f64eb0fccae93c2e2d628b87871b5b4da01038a6a5de38c3f8e7a715` |
| `??` | `crates/agent-service/src/worker_fairness_tests.rs` | `9689ada30eb7c7791a382d36ae1a8230f615f904b5ef994bcbcd5e624d3aaa2b` |
| `??` | `crates/agent-service/src/workers.rs` | `7f66247077e760f6f252c88acb9ba50d04e93680bc94cd0145ae85aeb541e139` |
| `??` | `crates/backend/src/agent_appservice.rs` | `7cc4af846baa7833725a15608082b72dfea67f3d4b739b3823090a1f0bff5731` |
| `??` | `crates/backend/src/browser_auth.rs` | `ee4800fc2d63d21c2f702ce6e205d857e502d1ba3de76c81774b09296e45a4cc` |
| `??` | `crates/frontend/src/api/browser_auth.rs` | `fc876d4eaaa3279d4dbc6b7fbd956f6210b7f7ada0e09736efe524cbe778f44b` |
| `??` | `crates/frontend/src/pages/hagency/agents.rs` | `dc7a487e4c3088b73e043f93a97d334182244e849586406e09df4e271ff264f1` |
| `??` | `crates/frontend/src/pages/hagency/project_controls.rs` | `854b6cef3a3374a3099223f307b20acfef10f0a3b18e967baee85715a5e1f5c4` |
| `??` | `crates/frontend/src/pages/hagency/room_discovery.rs` | `42fc7f720bf024ae1abc8f1c371e99f6038abbada0ed4801b62cfcfb940b7701` |
| `??` | `docs/BACKUP_RESTORE_VALIDATION.md` | `e0b2c98806a665d1de3b4403e8d388ab25da7d0a6111a3d42af6dd4e491d7977` |
| `??` | `docs/BACKUP_RESTORE_VALIDATION.zh-CN.md` | `c48cce1503c287f19b68585fdac76499a18c6558f845cd6df7607f5b65d2743c` |
| `??` | `docs/CRYPTO_CAPABILITY_GAPS.md` | `5366840260882b520185f2e8a170516d3ba54a964c5bf2d535d781b9096efe4e` |
| `??` | `docs/CRYPTO_CAPABILITY_GAPS.zh-CN.md` | `62096a34afc9e8bde2902cbfa0d685050250fc9cb5424684db49cd2d229acd4d` |
| `??` | `docs/RECOVERY.md` | `8b88f04bf8d24dfe90ab48cd224cea8e8fddf876e1e7609ccc2b854827fc6a39` |
| `??` | `docs/RECOVERY.zh-CN.md` | `afbcb14226970a08fbb0d9e08696ece79d1276463a359fdbf9f1c1d38bc61e45` |
| `??` | `docs/validation/docker-owner-v1.zh-CN.md` | `7d04e1f97d9503cff431f344bff31d1382a57b5b89da167fee7dcfe6ee43a34a` |
| `??` | `refactor-drafts/appservice-domain/.gitignore` | `5bcbda12c2c9d288a1f67f1f963eb3f25ff0edeb647aae2bca8b6e92a3df6130` |
| `??` | `refactor-drafts/appservice-domain/Cargo.lock` | `f09e1c9288c4b38f17fda3783e8cc5e32946e7ece5e0915f34d85c3b48a8fc6d` |
| `??` | `refactor-drafts/appservice-domain/Cargo.toml` | `129f4dc852e382c7e9d78f8aa62ac7d4b665f92421612ae85d76e3be74c7d4a5` |
| `??` | `refactor-drafts/appservice-domain/README.md` | `b1ed1e8a66298d832cbe680c682d3ff9f7faea58cbb061c6c878dc40340f4ee7` |
| `??` | `refactor-drafts/appservice-domain/src/authorization.rs` | `1302a66298d9449653e1bd7b685989ed1bba85626c61f12b43fd38af86a4b8ee` |
| `??` | `refactor-drafts/appservice-domain/src/lib.rs` | `f8c78bc043e59bcdb853ecd33a2f766d17ee07fb556c76d75ba2101c635cb209` |
| `??` | `refactor-drafts/appservice-domain/src/model.rs` | `a56a02c7d6cf2211813ee003977034a6ab7371508b71fac257040c6c19597e56` |
| `??` | `refactor-drafts/appservice-domain/src/schema.sql` | `3a0ffcfa4d81eabe01028e52ce4a6ad323808f1fad297aa25b84769644cdcf3a` |
| `??` | `refactor-drafts/appservice-domain/src/store.rs` | `94241b665c1638b333cc93e95425d8071b63abc91b482f8a416c07b28bed2f32` |
| `??` | `refactor-drafts/appservice-domain/src/store/tests.rs` | `e07333f11fdd3fdeb6355aa6e4a255a9b718a7204106368af6cf8bb7cd42daaf` |
| `??` | `scripts/test-agent-integration.py` | `8ff2b14d7ebe2b61528a3eddfa6ccd9a650d93606a5c15efe39b57e02ca88e76` |
| `??` | `scripts/test-agent-service-postgres.py` | `1a4b64aa780b5147d444ac3318cdbe6137e130a5e1839ca9cfa4e9c3c658c94e` |
| `??` | `scripts/validate-agent-openapi.py` | `184fae7d6bb130cc2746c3942a1a6ec05d8b9859adc1b25d6501bda4cd95e279` |
