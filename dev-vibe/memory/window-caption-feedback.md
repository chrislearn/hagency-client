# 原生窗口标题栏反馈

更新：2026-10-22。

- 用户反馈 Mini apps 等弹出窗口右上角按钮悬停不明显。现场关闭按钮可以变红；最小化/最大化继承 Makepad Window 的 #E9E9E9 悬停底色，与浅色 #F3F3F3 标题栏仅有很小差异，并非统一缺失鼠标事件。
- 应用 theme.rs 在共享 Window 原型上调整最小化/最大化按钮，悬停与按下分别使用 page/ink 混合 12%/20%，图标随 RINX_INK，关闭按钮保留红色反馈。使用 uniform 保留 shader 输入元数据，覆盖主窗口与后续创建的 Mini apps、文章、Moments、阅读器窗口。
- cargo build --locked -p hagency-app --example miniapp_window 成功；既有 theme::tests 4 passed。离线示例使用独立 D 盘资料目录，真实 D3D11 读回确认浅色/深色最大化悬停与深色关闭按钮红色反馈，日志无脚本/shader错误。
- 验证图片、日志和 profile：D:/BuildCache/hagency-client/miniapp-caption-20261022。构建及缓存由 tools/dev.ps1 设置并通过 cargo metadata 核对 D 盘 target。用户在重启查看后确认效果，并要求提交推送到 origin/main。
- 用户随后要求重新编译并重启本地程序。主窗口经正常关闭路径保存状态后结束旧 PID 44080；cargo build --locked -p hagency-app --bin hagency 成功。仍使用 HAGENCY_APP_DATA_DIR=D:/BuildCache/ui-review-data 启动新 PID 47364，账号恢复并打开 Mini apps。主程序日志 app.stdout.log/app.stderr.log 在上述验证目录；未修改托盘偏好设置。
