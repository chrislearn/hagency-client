# Editor v2 research (2026-09-21)

This is a design input, not a claim of feature parity with every current WeChat account tier. No real WeChat account was accessed.

## Primary product references successfully retrieved

- [96 editor's own product interface](https://bj.96weixin.com/index/index): lists paragraph/title/image styles, theme/template libraries, image tools, saved articles, Markdown and live article previews. This informs the division between content editing, media, styles and review; it is a third-party editor, not Tencent's own editor.
- [Yiban's own help center](https://assets.yiban.io/help): describes image insertion/library, cover creation, typography controls, article statistics and preview workflows. This informs cover/media management and review states. Its paid/plugin features are not assumed to be native WeChat features.
- [Micro-layout's own editor guide](https://weipaiban.cn/docs/wechat-editor): describes template/style libraries and its own visual editing workflow. This is supplementary competitor context, not a WeChat API specification.

## Tencent source availability

Direct retrieval of Tencent's old and current documentation routes failed in this tool environment:

- https://developers.weixin.qq.com/doc/offiaccount/Draft_Box/Add_draft.html
- https://developers.weixin.qq.com/doc/subscription/api/draftbox/draftmanage/api_draft_add.html
- https://developers.weixin.qq.com/doc/service/guide/product/draft.html
- https://developers.weixin.qq.com/doc/offiaccount/Publish/Publish.html
- https://developers.weixin.qq.com/doc/offiaccount/Publish/Delete_posts.html

Consequently this plan does not assert current Tencent editing quotas, withdrawal windows, cover restrictions or account eligibility. These do not govern the Matrix implementation. The six requested capabilities are explicit product requirements from the user.

## Primary implementation references

- [Matrix event replacements](https://spec.matrix.org/latest/client-server-api/#event-replacements): edits are replacement events referencing the original; the original content is not overwritten. Replacing the original is a different operation from sending an unrelated new article.
- [Matrix redactions](https://spec.matrix.org/latest/client-server-api/#redactions): use the redaction mechanism for withdrawal, and handle replacement events explicitly. Redaction is not a guarantee of erasing downloaded copies.
- [GPT Image 2 model](https://developers.openai.com/api/docs/models/gpt-image-2): verified the exact model identifier `gpt-image-2`, its Images API support, and snapshot `gpt-image-2-2026-04-21`. No model substitution is planned.
- Local pinned Makepad `47837267faf6970a6cc36acedf9f83846b277307`, `widgets/src/text_input.rs` and `widgets/src/html.rs`: rich display and text input are separate; the existing editor must acquire a real native editing layer.
- [Octoscript image-to-app flow](https://github.com/OctoSense-org/Octoscript-AppCard/tree/main/lab/image-to-appcard-flow): preserve exact generation prompts/bytes, measure observed atlas crops, author semantic mappings, retain independent native and visual acceptance.

## Existing v1 gaps verified in source

`src/article_app/model.rs` stores only title and Markdown, strips image elements from HTML, and persists one draft per account. `src/article_app/ui.rs` formats by inserting Markdown markers and sends a rich-text Matrix message, but does not retain the published event ID. It cannot currently manage multiple articles, images, themes, cover art, publication revisions or withdrawal. The v1 renderer is native Makepad; it does not instantiate a WebView.
