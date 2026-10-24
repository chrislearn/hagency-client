//! Task-scoped native media: uploads never submit the surrounding Markdown draft.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    Attachment,
    Description,
    Comment,
}
impl Target {
    fn path(self) -> &'static [LiveId] {
        match self {
            Self::Description => ids!(description_editor),
            Self::Comment => ids!(comment_editor),
            Self::Attachment => ids!(attachments_section),
        }
    }
}
#[derive(Clone, Debug)]
struct Insert {
    target: Target,
    source: String,
    range: std::ops::Range<usize>,
}
#[derive(Default)]
pub(super) struct MediaState {
    scope: u64,
    preparing: u64,
    queue: VecDeque<(AttachmentUpload, Insert)>,
    current: Option<(String, Insert)>,
    requested: HashSet<String>,
    delete_targets: HashMap<WidgetUid, String>,
    deleting: Option<String>,
    hovering: Option<Target>,
}
#[derive(Debug)]
struct Prepared {
    scope: u64,
    token: u64,
    insert: Insert,
    result: Result<Vec<AttachmentUpload>, String>,
}
#[derive(Default)]
struct ImageCache {
    entries: HashMap<String, Option<Texture>>,
}

fn image_cache(cx: &mut Cx) -> &mut ImageCache {
    cx.set_global(ImageCache::default());
    cx.get_global::<ImageCache>()
}

#[derive(Script, Widget)]
pub struct BoardInlineImage {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    #[rust]
    key: String,
    #[rust]
    loaded: Option<TextureId>,
}
impl ScriptHook for BoardInlineImage {
    fn on_after_new_scoped(&mut self, _vm: &mut ScriptVm, scope: &mut Scope) {
        if let Some(doc) = scope.props.get::<makepad_html::HtmlDoc>() {
            let mut walker = doc.new_walker_with_index(scope.index + 1);
            while let Some((name, value)) = walker.while_attr_lc() {
                if name == id!(src) {
                    self.key = value.into();
                }
            }
        }
    }
}
impl Widget for BoardInlineImage {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
    fn set_text(&mut self, cx: &mut Cx, text: &str) {
        self.view.label(cx, ids!(caption)).set_text(cx, text);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let entry = image_cache(cx).entries.get(&self.key).cloned();
        let texture = entry.clone().flatten();
        let texture_id = texture.as_ref().map(Texture::texture_id);
        if self.loaded != texture_id {
            self.loaded = texture_id;
            self.view
                .image(cx, ids!(picture))
                .set_texture(cx, texture.clone());
        }
        self.view
            .widget(cx, ids!(picture))
            .set_visible(cx, texture.is_some());
        self.view
            .label(cx, ids!(unavailable))
            .set_visible(cx, texture.is_none());
        self.view.label(cx, ids!(unavailable)).set_text(
            cx,
            if entry.is_none() {
                tr("Image unavailable · attachment removed or not part of this task")
            } else {
                tr("Loading image…")
            },
        );
        self.view.draw_walk(cx, scope, walk)
    }
}

fn upload(filename: String, bytes: Vec<u8>) -> Result<AttachmentUpload, String> {
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return Err(tr("Choose a non-empty file smaller than 2 MB.").into());
    }
    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if bytes.starts_with(b"%PDF-") {
        "application/pdf"
    } else {
        "application/octet-stream"
    };
    Ok(AttachmentUpload {
        idempotency_key: uuid::Uuid::new_v4().to_string(),
        filename,
        content_type: mime.into(),
        data: STANDARD.encode(bytes),
    })
}
fn file_upload(path: &std::path::Path) -> Result<AttachmentUpload, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    upload(
        path.file_name()
            .ok_or("Invalid filename")?
            .to_string_lossy()
            .into_owned(),
        bytes,
    )
}
fn image_reference(name: &str, id: &str) -> String {
    // Alt text is prose, never Markdown syntax supplied by a filename.
    let name = name
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace(['\r', '\n'], " ");
    format!("\n\n![{name}](attachment:{id})\n\n")
}
fn insertion_range(insert: &Insert, current: &str) -> std::ops::Range<usize> {
    if current == insert.source {
        insert.range.clone()
    } else {
        current.len()..current.len()
    }
}
impl ProjectBoard {
    pub(super) fn cancel_media_uploads(&mut self) {
        self.media.current = None;
        self.media.queue.clear();
    }
    pub(super) fn media_scope(&self) -> u64 {
        self.media.scope
    }
    pub(super) fn clear_media(&mut self, cx: &mut Cx) {
        let prefix = format!("{}:", self.media.scope);
        image_cache(cx)
            .entries
            .retain(|key, _| !key.starts_with(&prefix));
        self.media = MediaState {
            scope: nonce(),
            ..Default::default()
        };
        self.view
            .view(cx, ids!(attachment_delete))
            .set_visible(cx, false);
    }
    fn media_ready(&self) -> bool {
        self.detail_loaded
            && self.pending.is_none()
            && self.media.preparing == 0
            && self.media.queue.is_empty()
            && self
                .selected
                .as_ref()
                .is_some_and(|t| t.archived_at.is_none())
    }
    fn begin_media(&mut self, cx: &mut Cx, target: Target) -> Option<(u64, u64, Insert)> {
        if !self.media_ready() {
            self.notice(
                cx,
                tr("Wait for the current upload, or save this task before adding images."),
            );
            return None;
        }
        if self.media.scope == 0 {
            self.media.scope = nonce();
        }
        let input = self
            .view
            .view(cx, target.path())
            .text_input(cx, ids!(source));
        let source = input.text();
        let selection = input.selection();
        let insert = Insert {
            target,
            source,
            range: selection.anchor.index.min(selection.cursor.index)
                ..selection.anchor.index.max(selection.cursor.index),
        };
        self.media.preparing = nonce();
        Some((self.media.scope, self.media.preparing, insert))
    }
    pub(super) fn pick_media(&mut self, cx: &mut Cx, target: Target) {
        let Some((scope, token, insert)) = self.begin_media(cx, target) else {
            return;
        };
        let result = robius_file_picker::FileDialog::new().pick_file(move |picked| {
            std::thread::spawn(move || {
                let result = (|| {
                    let Some(file) = picked.map_err(|e| e.to_string())? else {
                        return Ok(Vec::new());
                    };
                    let file = file.into_local_file().map_err(|e| e.to_string())?;
                    Ok(vec![file_upload(file.path())?])
                })();
                Cx::post_action(Prepared {
                    scope,
                    token,
                    insert,
                    result,
                });
            });
        });
        if let Err(e) = result {
            self.media.preparing = 0;
            self.notice(cx, &e.to_string());
        }
    }
    fn paste_media(&mut self, cx: &mut Cx, target: Target, explicit: bool) {
        // Snapshot the pasteboard at the gesture, not after a background task starts.
        // Otherwise a later clipboard change could turn a text paste into an upload.
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        let image = {
            let mut clipboard = match arboard::Clipboard::new() {
                Ok(c) => c,
                Err(e) => {
                    if explicit {
                        self.notice(cx, &e.to_string());
                    }
                    return;
                }
            };
            if !explicit && clipboard.get_text().is_ok_and(|t| !t.is_empty()) {
                return;
            }
            match clipboard.get_image() {
                Ok(image)
                    if image.width <= 8192
                        && image.height <= 8192
                        && image.bytes.len() <= 64 * 1024 * 1024 =>
                {
                    (image.width, image.height, image.bytes.into_owned())
                }
                Ok(_) => {
                    self.notice(cx, tr("This image is too large to preview."));
                    return;
                }
                Err(_) => {
                    if explicit {
                        self.notice(cx, tr("Copy an image or screenshot first."));
                    }
                    return;
                }
            }
        };
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            let _ = (target, explicit);
            self.notice(cx, tr("Clipboard images are only available on desktop."));
        }
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        {
            let Some((scope, token, insert)) = self.begin_media(cx, target) else {
                return;
            };
            std::thread::spawn(move || {
                let result = (|| {
                    let rgba =
                        ::image::RgbaImage::from_raw(image.0 as u32, image.1 as u32, image.2)
                            .ok_or("Invalid clipboard image")?;
                    let mut png = std::io::Cursor::new(Vec::new());
                    rgba.write_to(&mut png, ::image::ImageFormat::Png)
                        .map_err(|e| e.to_string())?;
                    Ok(vec![upload(
                        format!("screenshot-{}.png", uuid::Uuid::new_v4().simple()),
                        png.into_inner(),
                    )?])
                })();
                Cx::post_action(Prepared {
                    scope,
                    token,
                    insert,
                    result,
                });
            });
        }
    }
    fn next_media(&mut self, cx: &mut Cx) {
        if self.pending.is_some() {
            return;
        }
        if let Some((input, insert)) = self.media.queue.pop_front()
            && let Some(task) = &self.selected
        {
            self.media.current = Some((input.idempotency_key.clone(), insert));
            self.write(
                cx,
                BoardAction::UploadAttachment {
                    task: task.id.clone(),
                    input,
                },
            );
        }
    }
    pub(super) fn media_saved(&mut self, cx: &mut Cx, action: &Option<BoardAction>, value: &Value) {
        self.media.requested.clear();
        let prefix = format!("{}:", self.media.scope);
        image_cache(cx)
            .entries
            .retain(|key, value| !key.starts_with(&prefix) || value.is_some());
        match action {
            Some(BoardAction::UploadAttachment { input, .. }) => {
                if let Some((key, insert)) = self.media.current.take()
                    && key == input.idempotency_key
                    && insert.target != Target::Attachment
                {
                    if let Some(id) = value["attachment"].as_str().filter(|id| valid_id(id)) {
                        let editor = self.view.view(cx, insert.target.path());
                        let field = editor.text_input(cx, ids!(source));
                        let current = field.text();
                        let range = insertion_range(&insert, &current);
                        // Upload acknowledgement has released the mutation lock; restore
                        // editability before using the native undo-aware edit operation.
                        field.set_is_read_only(cx, false);
                        let inserted = field.replace_range(
                            cx,
                            range,
                            &image_reference(&input.filename, id),
                            makepad_widgets::text_input::UndoGroup::New,
                        );
                        if inserted.is_err() {
                            self.notice(cx,tr("Image uploaded, but could not be inserted. Please reopen the editor."));
                            return;
                        }
                        if insert.target == Target::Description {
                            self.dirty = true;
                            self.discard_armed = false;
                        } else {
                            self.remember_comment(cx);
                        }
                    } else {
                        self.notice(cx,tr("Attachment uploaded. This server must be updated to insert image references."));
                    }
                }
            }
            Some(BoardAction::RemoveAttachment { input, .. }) => {
                image_cache(cx)
                    .entries
                    .remove(&format!("{}:{}", self.media.scope, input.attachment));
                self.media.deleting = None;
                self.view
                    .view(cx, ids!(attachment_delete))
                    .set_visible(cx, false);
                self.view
                    .widget(cx, ids!(attachment_preview))
                    .set_visible(cx, false);
            }
            _ => (),
        }
        self.sync_markdown_views(cx);
    }
    pub(super) fn media_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        for target in [Target::Description, Target::Comment] {
            let editor = self.view.view(cx, target.path());
            if editor.button(cx, ids!(insert_image)).clicked(actions) {
                self.pick_media(cx, target);
            }
            if editor.button(cx, ids!(paste_image)).clicked(actions) {
                self.paste_media(cx, target, true);
            }
        }
        if self.view.button(cx, ids!(delete_cancel)).clicked(actions) {
            self.media.deleting = None;
            self.view
                .view(cx, ids!(attachment_delete))
                .set_visible(cx, false);
        }
        if self.view.button(cx, ids!(delete_confirm)).clicked(actions)
            && self.pending.is_none()
            && let (Some(id), Some(task)) = (self.media.deleting.clone(), self.selected.as_ref())
        {
            self.write(
                cx,
                BoardAction::RemoveAttachment {
                    task: task.id.clone(),
                    input: AttachmentRemove {
                        idempotency_key: uuid::Uuid::new_v4().to_string(),
                        attachment: id,
                    },
                },
            );
        }
        for action in actions {
            if let Some(prepared) = action.downcast_ref::<Prepared>()
                && prepared.scope == self.media.scope
                && prepared.token == self.media.preparing
            {
                self.media.preparing = 0;
                match &prepared.result {
                    Ok(inputs) if self.pending.is_none() => {
                        if inputs.len() + self.attachments.len() > 20 {
                            self.notice(cx, tr("This task has reached its attachment limit."));
                            continue;
                        }
                        if prepared.insert.target != Target::Attachment
                            && inputs.iter().any(|i| !i.content_type.starts_with("image/"))
                        {
                            self.notice(
                                cx,
                                tr("Insert a PNG or JPEG image. Add other files in Attachments."),
                            );
                            continue;
                        }
                        self.media
                            .queue
                            .extend(inputs.iter().cloned().map(|i| (i, prepared.insert.clone())));
                        self.next_media(cx);
                    }
                    Ok(_) => self.notice(
                        cx,
                        tr("Task changed while preparing the image. Please try again."),
                    ),
                    Err(e) => self.notice(cx, e),
                }
            }
            if let Some(widget) = action.as_widget_action()
                && matches!(widget.cast::<ButtonAction>(), ButtonAction::Clicked(_))
                && let Some(id) = self.media.delete_targets.get(&widget.widget_uid).cloned()
                && self.pending.is_none()
            {
                let name = self
                    .attachments
                    .iter()
                    .find(|v| v["id"] == id)
                    .and_then(|v| v["filename"].as_str())
                    .unwrap_or("");
                self.view
                    .label(cx, ids!(delete_filename))
                    .set_text(cx, name);
                self.media.deleting = Some(id);
                self.view
                    .view(cx, ids!(attachment_delete))
                    .set_visible(cx, true);
            }
        }
        self.next_media(cx);
    }
    pub(super) fn media_row(&mut self, cx: &mut Cx, row: &WidgetRef, id: &str) {
        self.media
            .delete_targets
            .insert(row.button(cx, ids!(remove)).widget_uid(), id.into());
        let button = row.button(cx, ids!(remove));
        let ready = self.media_ready();
        let was_enabled = button.borrow().is_some_and(|b| b.enabled());
        if was_enabled != ready {
            button.set_board_enabled(cx, ready);
        }
    }
    pub(super) fn media_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::KeyDown(key) = event
            && key.key_code == KeyCode::KeyV
            && key.modifiers.is_primary()
            && !key.is_repeat
        {
            for target in [Target::Description, Target::Comment] {
                let input = self
                    .view
                    .view(cx, target.path())
                    .text_input(cx, ids!(source));
                if cx.has_key_focus(input.area()) && !input.is_read_only() {
                    self.paste_media(cx, target, false);
                    break;
                }
            }
        }
        if self.selected.is_none()
            || !matches!(event, Event::Drag(_) | Event::Drop(_) | Event::DragEnd)
        {
            return;
        }
        if matches!(event, Event::DragEnd) {
            self.media.hovering = None;
        }
        for target in [Target::Description, Target::Comment, Target::Attachment] {
            if (target == Target::Comment) != self.discussion {
                continue;
            }
            let pane = self.view.view(cx, target.path());
            if !pane.visible() {
                continue;
            }
            match event.drag_hits(cx, pane.area()) {
                DragHit::Drag(hit)
                    if hit.state != DragState::Out
                        && self.media_ready()
                        && hit.items.iter().any(|i| {
                            matches!(
                                i,
                                DragItem::FilePath {
                                    internal_id: None,
                                    ..
                                }
                            )
                        }) =>
                {
                    *hit.response.lock().unwrap() = DragResponse::Copy;
                    self.media.hovering = Some(target);
                    self.view.label(cx, ids!(media_drop_hint)).set_text(
                        cx,
                        if target == Target::Attachment {
                            tr("Release to add attachments")
                        } else {
                            tr("Release to insert images into this draft")
                        },
                    );
                }
                DragHit::Drop(hit) => {
                    self.media.hovering = None;
                    let paths: Vec<_> = hit
                        .items
                        .iter()
                        .filter_map(|item| {
                            if let DragItem::FilePath {
                                path,
                                internal_id: None,
                            } = item
                            {
                                Some(std::path::PathBuf::from(path))
                            } else {
                                None
                            }
                        })
                        .collect();
                    if paths.is_empty() {
                        continue;
                    }
                    if paths.len() > 20 {
                        self.notice(cx, tr("This task has reached its attachment limit."));
                        continue;
                    }
                    if let Some((scope, token, insert)) = self.begin_media(cx, target) {
                        std::thread::spawn(move || {
                            Cx::post_action(Prepared {
                                scope,
                                token,
                                insert,
                                result: paths.iter().map(|p| file_upload(p)).collect(),
                            })
                        });
                    }
                }
                DragHit::DragEnd => self.media.hovering = None,
                DragHit::Drag(hit)
                    if hit.state == DragState::Out && self.media.hovering == Some(target) =>
                {
                    self.media.hovering = None;
                }
                _ => (),
            }
        }
        self.view
            .label(cx, ids!(media_drop_hint))
            .set_visible(cx, self.media.hovering.is_some());
        if matches!(event, Event::Drag(_) | Event::Drop(_) | Event::DragEnd) {
            for target in [Target::Description, Target::Comment, Target::Attachment] {
                let mut pane = self.view.view(cx, target.path());
                if self.media.hovering == Some(target) {
                    script_apply_eval!(cx,pane,{draw_bg.border_color:mod.widgets.RINX_ACCENT draw_bg.border_size:2.0});
                } else {
                    script_apply_eval!(cx,pane,{draw_bg.border_color:mod.widgets.RINX_BORDER draw_bg.border_size:1.0});
                }
            }
            self.view.redraw(cx);
        }
    }
    pub(super) fn media_html(&self, source: &str) -> String {
        markdown::render_markdown_with_media(source, self.media.scope)
    }
    pub(super) fn sync_inline_images(&mut self, cx: &mut Cx) {
        self.sync_markdown_views(cx);
        if self.media.scope == 0 {
            self.media.scope = nonce();
        }
        let Some(task) = self.selected.as_ref().map(|t| t.id.clone()) else {
            return;
        };
        let prefix = format!("{}:", self.media.scope);
        let active: HashSet<_> = self
            .attachments
            .iter()
            .filter_map(|v| v["id"].as_str().map(str::to_owned))
            .collect();
        image_cache(cx)
            .entries
            .retain(|key, _| !key.starts_with(&prefix) || active.contains(&key[prefix.len()..]));
        let mut sources = vec![
            self.view
                .text_input(cx, ids!(description_editor.source))
                .text(),
            self.view.text_input(cx, ids!(comment_editor.source)).text(),
        ];
        sources.extend(
            self.comments
                .iter()
                .filter_map(|v| v["body"].as_str().map(str::to_owned)),
        );
        for id in sources
            .iter()
            .flat_map(|s| references(s))
            .collect::<HashSet<_>>()
        {
            let key = format!("{prefix}{id}");
            if active.contains(&id)
                && !image_cache(cx).entries.contains_key(&key)
                && self.media.requested.insert(id.clone())
            {
                image_cache(cx).entries.insert(key, None);
                self.read(
                    cx,
                    Read::InlineImage(task.clone(), id.clone(), self.media.scope),
                    BoardAction::Attachment {
                        task: task.clone(),
                        attachment: id,
                    },
                );
            }
        }
        self.sync_markdown_views(cx);
    }
    pub(super) fn inline_image(
        &mut self,
        cx: &mut Cx,
        task: &str,
        id: &str,
        scope: u64,
        value: &Value,
    ) {
        if scope != self.media.scope
            || !self.selected.as_ref().is_some_and(|t| t.id == task)
            || !self.attachments.iter().any(|a| a["id"] == id)
        {
            return;
        }
        let result = (|| {
            let bytes = STANDARD.decode(value["data"].as_str()?).ok()?;
            if bytes.len() > 2 * 1024 * 1024 {
                return None;
            }
            let mut reader = ::image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()
                .ok()?;
            let mut limits = ::image::Limits::default();
            limits.max_image_width = Some(8192);
            limits.max_image_height = Some(8192);
            limits.max_alloc = Some(64 * 1024 * 1024);
            reader.limits(limits);
            let image = reader.decode().ok()?.thumbnail(1024, 1024).into_rgba8();
            ImageBuffer::new(
                image.as_raw(),
                image.width() as usize,
                image.height() as usize,
            )
            .ok()
            .map(|i| i.into_new_texture(cx))
        })();
        let key = format!("{scope}:{id}");
        if let Some(texture) = result {
            image_cache(cx).entries.insert(key, Some(texture));
        } else {
            image_cache(cx).entries.remove(&key);
        }
        self.view.redraw(cx);
    }
}
pub(super) fn valid_id(id: &str) -> bool {
    id.starts_with("att_")
        && id.len() <= 80
        && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
fn references(source: &str) -> Vec<String> {
    pulldown_cmark::Parser::new(source)
        .filter_map(|event| match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }) => dest_url
                .strip_prefix("attachment:")
                .filter(|id| valid_id(id))
                .map(str::to_owned),
            _ => None,
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_file_drag_accepts_comment_target_and_clears_highlight_on_cancel() {
        use std::sync::{Arc, Mutex};
        let (mut cx, mut board) = super::super::tests::panel();
        board.selected = Some(super::super::tests::task("t1", "todo"));
        board.detail_loaded = true;
        board.discussion = true;
        board
            .view
            .view(&cx, ids!(inspector))
            .set_visible(&mut cx, true);
        board.sync_controls(&mut cx);
        super::super::tests::draw(&mut cx, &mut board, 1500.0, 1600.0);
        let rect = board
            .view
            .view(&cx, ids!(comment_editor))
            .area()
            .clipped_rect(&cx);
        assert!(rect.size.x > 0.0 && rect.size.y > 0.0);
        let response = Arc::new(Mutex::new(DragResponse::None));
        let handled = Arc::new(Mutex::new(false));
        let event = Event::Drag(DragEvent {
            modifiers: Default::default(),
            handled: handled.clone(),
            abs: rect.pos + rect.size * 0.5,
            items: Arc::new(vec![DragItem::FilePath {
                path: "/tmp/截图.png".into(),
                internal_id: None,
            }]),
            response: response.clone(),
        });
        board.media_event(&mut cx, &event);
        assert_eq!(*response.lock().unwrap(), DragResponse::Copy);
        assert!(*handled.lock().unwrap());
        assert_eq!(board.media.hovering, Some(Target::Comment));
        board.media_event(&mut cx, &Event::DragEnd);
        assert_eq!(board.media.hovering, None);
        assert!(!board.view.widget(&cx, ids!(media_drop_hint)).visible());
    }
    #[test]
    fn attachment_references_are_scoped_and_filenames_cannot_inject_markdown() {
        let text = image_reference("a] (bad) [截图.png", "att_123");
        assert_eq!(references(&text), vec!["att_123"]);
        assert!(references("![x](attachment:../other) ![y](https://remote/image)").is_empty());
    }
    #[test]
    fn late_upload_preserves_edits_made_while_preparing() {
        let insert = Insert {
            target: Target::Comment,
            source: "中文 draft".into(),
            range: 7..12,
        };
        assert_eq!(insertion_range(&insert, "中文 draft"), 7..12);
        assert_eq!(insertion_range(&insert, "new text"), 8..8);
    }
    #[test]
    fn upload_ack_inserts_into_temporarily_locked_editor_and_preserves_draft() {
        let (mut cx, mut board) = super::super::tests::panel();
        board.selected = Some(super::super::tests::task("t1", "todo"));
        board.project = "p1".into();
        board.clear_media(&mut cx);
        let input = upload("截图.png".into(), b"\x89PNG\r\n\x1a\nfixture".to_vec()).unwrap();
        let field = board.view.text_input(&cx, ids!(comment_editor.source));
        field.set_text(&mut cx, "中文草稿");
        field.set_is_read_only(&mut cx, true);
        board.media.current = Some((
            input.idempotency_key.clone(),
            Insert {
                target: Target::Comment,
                source: "中文草稿".into(),
                range: 12..12,
            },
        ));
        board.media_saved(
            &mut cx,
            &Some(BoardAction::UploadAttachment {
                task: "t1".into(),
                input,
            }),
            &serde_json::json!({"attachment":"att_123"}),
        );
        assert!(
            field
                .text()
                .starts_with("中文草稿\n\n![截图.png](attachment:att_123)")
        );
        assert_eq!(
            board.comment_drafts.get(&("p1".into(), "t1".into())),
            Some(&field.text())
        );
        let scope = board.media.scope;
        image_cache(&mut cx)
            .entries
            .insert(format!("{scope}:att_123"), None);
        board.clear_media(&mut cx);
        assert!(
            !image_cache(&mut cx)
                .entries
                .contains_key(&format!("{scope}:att_123"))
        );
        assert_ne!(scope, board.media.scope);
    }
    #[test]
    fn markdown_emits_only_valid_task_attachment_images() {
        let html = markdown::render_markdown_with_media(
            "前文 ![截图](attachment:att_123) 后文 ![remote](https://example.test/a.png) <boardimage src='secret'>",
            42,
        );
        assert!(html.contains("<boardimage src=\"42:att_123\">截图</boardimage>"));
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;boardimage"));
        assert!(!html.contains("<boardimage src='secret'"));
    }
}
