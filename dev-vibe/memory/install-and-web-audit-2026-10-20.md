# install 脚本与 Web 入口核对（2026-10-20）

- 用户要求检查 install/install-native.sh，无用则清理，并核对 Web 服务/页面的用途。
- 脚本仅调用二进制内建的 service install，没有下载、打包或独立安装逻辑；仍要求 INSTALL_DIR/hagency，而当前 GUI 为 hagency、CLI 为 hagency-cli。构建、发布与桌面端均不调用该脚本，仅 release_cutover 测试读取/执行它。删除重复且名称过期的包装脚本，用户服务统一直接调用 hagency-cli service install；无需移除内建服务管理。
- release_cutover 的 Node 入口扫描保留 deploy 两份参考模板及负向控制。原包装脚本测试改为真实 CLI 直接拒绝旧状态、保留字节和不产生凭据的集成测试，绑定同步改名为 native_service_install_refuses_legacy_state_before_supervisor。没有新增 OS/feature skip。
- deploy 模板仍被 release_cutover 的发布、升级和回滚夹具引用，保留。Rust service.rs 自行生成用户级 unit，不读取这些模板。
- Web 运行入口为 crates/hagency 的 main.rs -> owner_host::OwnerHost -> console::owner_router，默认仅 127.0.0.1:13300。mockup/scripts/build-native-console.mjs 实际打包 login、projects、agents-owned 及 owner 首页；release-native.yml 构建并内嵌静态资源，rust.yml 验证真实二进制服务页面及浏览器流程。该可选无界面入口仍受支持，不能作为死代码删除。
- 桌面端 crates/hagency-app/src/hagency/backend.rs 直接创建 NativeOwner::open_with_matrix；native_owner.rs 重导出 console::native。GUI 不启动 CLI Web listener，也不需要浏览器 bundle，但共享 console 模块下的登录、权限、Agent 运行和语音业务代码，不能整目录删除。
- NativeOwner::open_inner 仍实际绑定 127.0.0.1:0，只在该随机本机端口暴露 console/server-login/callback，接收 OAuth 登录回调。其管理命令通过进程内 Salvo Service 调用，不把完整管理 router 暴露到该端口；这个小 HTTP 回调仍有实际用途。
- mockup 还保留历史原型；mockup-pages.yml 仅历史分支 push 或手动发布设计原型静态 Pages，不是生产 OwnerHost，也不需要 Node 运行服务器。此轮未把 mockup 整体当作废代码处理。
- 根双语 README、CLI 双语 README、workspace 指南已说明替代命令及 Web/桌面关系；历史导读撤掉指向已删除脚本的链接并标明历史。CI 范围检测移除无效 install/ 路径。
- 本地构建、Cargo/cache/tmp/日志均位于 D:/BuildCache/hagency-client；metadata 确认 target_directory 位于 D 盘。没有安装真实用户服务、删除用户数据、发送消息或发布部署。
- 验证：两个 release_cutover 针对性集成测试各 1 passed/0 failed/0 ignored；实际编译测试库存解析本 spec 两项绑定，0 missing；该测试目标严格 Clippy、fmt/diff、actionlint、28 包工作区边界检查、32 项 Node 检查器自测均通过。安装脚本失效链接和旧测试选择器引用检索均无残留。
