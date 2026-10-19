# Article editor v2: Markdown writing view

The v2 redesign replaces the block editor as the default editing surface with
one full-document Markdown source and a live rendered preview, closer to
[huasheng_editor](https://github.com/alchaincyf/huasheng_editor).

- **Desktop:** split view, with source on the left and a themed preview on a
  paper-width card on the right. A 编辑 | 双栏 | 预览 switch and a style
  (theme) panel sit in the header.
- **Phone width:** 编辑 | 预览, a title field above the source, and
  formatting at the bottom.

## Design atlases (image-to-appcard-flow)

| Atlas | Scenes | Artboard | Generation |
|---|---|---|---|
| `source/atlas-v2-mobile.png` | 8: article list, Markdown editing, live preview, theme sheet, insert image, cover and summary, choose chat, published | 406×776 | `gpt-image-2`, requested and actual 2048×2048 |
| `source/atlas-v2-desktop.png` | 4: split editor, theme gallery, paste images, publish sheet | 1280×800 | `gpt-image-2`, requested and actual 2560×1600 |

- **Records:** the exact prompts and receipts (model, sizes, SHA-256) are
  alongside the atlases. No credentials are recorded.
- **Mobile intake:** the upstream flow ran `intake` and `prepare`
  (`image-to-appcard-flow-v2-mobile.json`, output in
  `pipeline-output/intake-v2-mobile`).
- **Desktop intake (local extension):** upstream `flow.py` accepts only the
  406×776 artboard, and `atlas.py` requires 8–12 scenes.
  `intake_desktop.py` runs the unmodified upstream `atlas.py` with only the
  scene-count check relaxed. Crops, reversible transforms and hashes are
  upstream's. Output is in `pipeline-output/intake-v2-desktop`.

## Instrument verification

The same code path as the flow's Studio-less instrument route:
- **Instrument mode:** the real Rinx binary runs with hidden windows
  (`MAKEPAD_HIDE_WINDOWS=1`) and is driven through its remote-control port
  (`MAKEPAD_REMOTE`), using the signed-in demo fixture profile.
- **Comparison:** `compare_v2.py` compares window grabs with the intake
  references, using `compare_screens.py`'s side-by-side and diff-region
  functions. Output is in `evidence-v2/`. These are evidence about pixels,
  not a visual acceptance or a similarity score.

| Scene | Result after fixes |
|---|---|
| split-editor | Header (文章编辑器, centred title, style pill, 编辑/双栏/预览, 已保存, 发布), toolbar, source, live preview card, three-image gallery row, stats line |
| theme-gallery | 3×4 swatch grid; 金融时报 re-themes the preview and the pill |
| markdown-editing (phone) | Back, 编辑/预览, 发布; title field; source; 已保存 and bottom toolbar |
| live-preview (phone) | Title, quote, heading, list, three-image row |
| article-list (phone) | 我的文章 with + in the header; rows with thumbnail (or an "Aa" placeholder), title, "草稿 · 23 分钟前" or "已发布到 …", chevron |
| theme-sheet (phone) | "Aa 样式" button on the preview opens a bottom sheet: 12 swatches in a horizontal strip, 应用. Tapping a swatch re-themes the preview behind the sheet; × restores the previous style; 应用 keeps it as one undo step |
| insert-image (phone) | 图片 in the bottom toolbar opens 插入图片: 从设备选择, the 多张图片自动排版 switch, and a note |
| cover-summary (phone) | White page: cover with 更换封面, share card, summary, 下一步 |
| choose-chat (phone) | 发布到: search, chats with initial avatars and radio marks, 发布 |

Defects found by looking at the side-by-sides and fixed:
- CJK text rendered as boxes in the monospace source font.
- The title was repeated in the preview.
- The source pane background was grey.
- The header overflowed at phone width, hiding 预览 and 发布.
- Three images rendered stacked full-width instead of as one row.
- The theme list needed scrolling.

The toolbar was exercised through the instrument: **B** wraps or inserts
`**…**`, and **H** prefixes the line with `## `.

## Functional additions (after v2 screens)

- **Short image names:** the source shows `asset:` plus the first 8 hex
  digits of each image ID. It keeps the full ID where two images share that
  prefix. Storage, parsing and publishing always use full IDs
  (`shorten_assets` / `expand_assets`, unit-tested). Verified in the app.
- **Shortcuts:** Cmd/Ctrl+B, I and K wrap the selection in bold, italic or a
  link. Verified in the app (Cmd+B).
- **Pasting images (desktop):** Cmd/Ctrl+V with an image and no text on the
  clipboard imports the image at the cursor. Text pastes as usual. Verified
  in the app: two pastes in a row render as a two-image grid.
- **Picking several images (desktop):** 图片 opens a multi-select dialog
  (rfd), and the images are inserted in the order picked. Not verified in
  the app: a modal dialog cannot be driven over the bridge.
- **Publishing fix:** images referenced inside blocks kept as Markdown
  source (for example, several images in one paragraph) were not counted by
  `Document::asset_ids`. They were therefore not uploaded, and readers did not
  load them. They now count (article-core test). Readers compare the uploaded
  set with `asset_ids`, so older Rinx builds reject articles that contain such
  images, where before those images were silently missing.
- **Character count:** blocks kept as Markdown or HTML source are counted
  without markup, image references or tags (article-core test).
- **Other Matrix clients:** images inside blocks kept as source (image
  grids) are sent as their uploaded `mxc://` media in the plain-HTML
  fallback. In encrypted rooms they are sent as their description, as
  before for other images (backend test).
- **Preview follows the cursor (split view):** moving the cursor, clicking
  or typing scrolls the preview to the block being edited, unless that block
  is already in view. Verified in the app on a 30-section article. Scrolling
  the source with the wheel alone does not move the preview: the text input
  does not expose its scroll position.
- **Unappliable source:** if the writing view's source cannot be applied to
  the article, 发布 is blocked, and the reason is shown in the stats line
  ("暂不能发布：…", "仅保存为源码"). Before, the last applied version
  could be published.

## Known gaps (not yet matching the atlases, or not verified)

- **Source pane:** no line numbers or Markdown syntax colouring; it is a plain
  text input.
- **Accent colours:** the quote bar and headings use the ink colour rather
  than the theme accent.
- **Favourites:** the theme gallery has no favourite stars.
- **Dropping image files:** implemented, with an overlay and import at the
  cursor, but **not verified in the running app**: the remote bridge has no
  drag-and-drop route.
- **Images:** they are resized to 2048 px and stored as PNG (the existing
  importer), not JPEG-compressed. The "自动压缩" wording is
  aspirational until then.
- **Insert image (phone):** 拍照 and 粘贴剪贴板图片 from the atlas are left
  out, because Rinx cannot take photos or paste images there yet. The file
  picker picks one image at a time. With 多张图片自动排版 on, an image
  inserted straight after another joins its paragraph, so they render as a
  grid; off, each image gets its own paragraph (`image_insertion`,
  unit-tested). A picker cannot be driven through the bridge, so this was
  verified by the unit test, not in the running app.
- **Paste-images toast (desktop):** "已插入 N 张图片" appears for 2.5 s after
  images are inserted. It is not verified in the running app: it needs
  the file picker or a drop.
- **Theme sheet:** the atlas's favourite stars are not implemented. The
  selected style is not scrolled into view.
- **Published scene:** not verified visually, because the test homeserver
  is offline. The error path (the status is shown and the flow stays open)
  was verified.
- **Block editor:** it is no longer reachable from the UI (`block_mode`). The
  older `tools/wechat-ux/live/native_article_*.py` scripts that drive it need
  updating.
