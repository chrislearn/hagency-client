Use case: ui-mockup
Asset type: a production UX reference atlas for a native Chinese/English article-authoring mini app inside Robrix, a WeChat-style messenger.
Primary request: design TWELVE related mobile states as one coherent, exceptionally legible interface atlas. This is a real writing product with direct rich-text editing, inline photographs, article themes, cover art, full article review and publication management. Show meaningful realistic article content, not grey skeleton placeholders. This is a UX reference to implement with real Makepad widgets, not a runtime bitmap.

Canvas: 3840 by 2160 pixels. Exactly six evenly spaced phone-sized flat application panels across and two rows, read left-to-right then top-to-bottom. Each panel shows a complete portrait screen with approximately 430:860 content aspect ratio, consistent scale, clear gutters and no overlap. The small state number/caption belongs OUTSIDE each screen. No physical device bezels, hands, perspective, explanatory arrows or decorative background. Use the canvas efficiently so every screen and its Chinese text remain sharp. Preserve comfortable white space inside the UI rather than adding margins around the atlas.

Style: familiar WeChat mobile proportions, restrained product design, white article paper, #ededed navigation backgrounds, #07c160 primary action, #191919 primary text, subtle grey borders, thin consistent outline icons, PingFang-like Simplified Chinese typography. Small back chevron inside a generous hit target; circular plus with thin stroke. Page title 18px equivalent, article title 26px, article body 16px, captions 12px. Avoid large pill buttons, excessive drop shadows, purple gradients and marketing decoration. Buttons are modest rectangles with 6px corners. The editor chrome stays consistent when an article theme changes.

Shared fixture across all screens:
App name: "图文创作".
Article title: "把周末还给山野".
Author: "林间来信".
Summary: "在山路与晨光之间，找回生活的节奏。".
Main photo: natural editorial photograph of layered green mountain ridges, soft dawn mist, warm light, no people, no text or logo.
Body opening: "清晨六点，城市还没有醒来。我们沿着山路出发，把消息提醒留在身后。".
Section title: "01 走进山野".
Quote: "慢一点，才能看见更多。".
Second paragraph: "风穿过松林，光落在石阶。这个周末，我们只做一件事：认真感受眼前的风景。".
Photo caption: "晨光里的山谷".
Chat destination: "周末徒步小组".
Version example: "版本 2".
Use these exact Simplified Chinese labels, naturally typeset, with no invented nonsense text. The mock article is sample content, not someone's private message.

Row 1 / screen 01, Article library:
Navigation "图文创作", small back, overflow. Search "搜索文章". Segmented tabs "草稿", "已发布", "已撤回", with 草稿 selected. Two well-designed article rows: a mountain cover thumbnail with the shared title, summary, status "刚刚保存"; another plain draft titled "一杯茶的时间". Clear green "新建文章" action with small plus. Keep the list airy, title and thumbnail hierarchy precise. This is the app home, not a chat list.

Row 1 / screen 02, True visual editor:
Top bar back, "编辑文章", subtle "已保存", "预览". Below is an editable paper-like document: the shared title and author, a flowing paragraph, section heading, mountain photo and caption. Text is already typeset, with NO Markdown syntax markers. A selected phrase in the first paragraph has pale green selection and small selection handles; a compact floating toolbar shows bold B, italic I and link icon. A plus insertion control between blocks is small and circular. Bottom toolbar labels "文字", "图片", "样式", "封面", "更多". Editor is for direct writing, not a source/preview split. Show a credible native focus state without a software keyboard covering most of the page.

Row 1 / screen 03, Image library:
Navigation "插入图片", small close, "插入 1" green action. Prominent simple "从设备选择" button; label "本篇素材"; 2-column grid of mountain, forest and trail photographs, the selected mountain has a green check. Bottom explanation "选择后插入正文". No external stock marketplace, no AI generation button, no upload-success badge before publication.

Row 1 / screen 04, Inline image settings:
Navigation "图片设置", "完成". Large mountain photo in the same body width, fine green selected outline. Fields "图注" with "晨光里的山谷", "替代文字" with "薄雾中的绿色山谷". Width choices "100%", "75%", "50%", first selected. Controls "上移", "下移", "替换图片" and restrained red "移除图片". All controls fit above safe bottom area.

Row 1 / screen 05, Theme selection:
Navigation "文章样式", "完成". Four article theme cards in a 2 by 2 grid: "经典绿", "书页暖白", "海盐蓝", "极简墨色". Each card contains a tiny real typeset heading, paragraph, quote rule and photo; the paper colors and accent rules visibly differ, but chrome stays green/white. The 经典绿 card has a tasteful green check outline. Below, "字号" with "标准 / 大字" and "行距" with "舒适 / 紧凑". Footer note "只改变样式，保留内容".

Row 1 / screen 06, Cover and summary:
Navigation "封面与摘要", "完成". Mountain image in a 2.35:1 crop with subtle crop grid/focal point. Small live previews labeled "宽封面" and "方形缩略图". Action "更换封面". Title/author fields with fixture content, "摘要" text field with fixture summary and counter. Toggle "在全文顶部显示封面" on. Avoid suggesting the crop can erase content from the original image.

Row 2 / screen 07, Full article review:
Navigation "全文预览", small back, compact width icon. Fully typeset article reading screen: mountain cover, large title, author, introductory paragraph, section heading, photo/caption, quote. This is a real long document with a visible scrollbar/reading progress and content continuing below the viewport. Bottom fixed small metadata "约 3 分钟 · 6 张图片" and clear "发布检查" action. No editing toolbar or fake review approval checkmark.

Row 2 / screen 08, Pre-publication review:
Navigation "发布检查". Small cover/title card. Checklist rows "标题与正文" and "图片资源" with green checks; "封面与摘要" with check. Destination row "发布到" and "周末徒步小组" with chevron. Readable neutral explanation "确认后上传图片并发表". Primary "继续". The destination can be changed and there is no false guarantee of content moderation.

Row 2 / screen 09, Final publish confirmation:
Navigation "确认发布". A polished wide-cover article card with title, author and summary. Details "当前账号" with a fictional "@lin:example.org", "接收会话" with shared chat name, "图片" with "6 张". Primary "确认发布", secondary "返回修改". Text "仅发布到所选会话". This is an explicit confirmation before publishing.

Row 2 / screen 10, Publication record:
Navigation "发表记录". Green modest success indicator "已发布". Article cover/title, destination, timestamp example "今天 10:24", "版本 1". Action rows "阅读全文", "修改文章", "撤回文章"; withdrawal uses restrained red text. Footer "本地草稿已保留". No invented view/like counts.

Row 2 / screen 11, Update review:
Navigation "确认更新". Compact cover/title card, "版本 1 → 版本 2". Change summary rows "正文：新增 1 个段落", "图片：替换 1 张", "主题：经典绿". Destination is fixed to the original chat. Primary "确认更新", secondary "继续编辑". Helpful text "原文章将显示最新版本". No suggestion that saving a draft automatically updates the published article.

Row 2 / screen 12, Withdrawal confirmation:
Navigation "撤回文章". Small article cover/title, selected destination. A calm centered confirmation panel says "确认撤回这篇文章？" and "本地草稿会保留。已下载或另行转发的副本无法收回。". Clearly separate secondary "取消" and red primary "确认撤回" buttons. Do not show 已撤回 until this action succeeds. This screen must be as considered and legible as the editor.

Constraints: consistent fixture, spacing, navigation and icon family across all twelve panels; good Chinese typography; all panels complete and uncropped; one palette shared by app chrome; native widget-realizable flat surfaces; no browser chrome, no WebView, no logos from unrelated apps, no watermarks, no API keys, no backend implementation labels. The result should look like an integrated, mature writing mini app, not twelve unrelated landing pages.
