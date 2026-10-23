use super::*;

impl ProjectBoard {
    pub(super) fn remember_comment(&mut self, cx: &Cx) {
        let Some(task) = &self.selected else {
            return;
        };
        let text = self.view.text_input(cx, ids!(comment_editor.source)).text();
        let key = (self.project.clone(), task.id.clone());
        if text.is_empty() {
            self.comment_drafts.remove(&key);
        } else {
            self.comment_drafts.insert(key, text);
        }
    }
    pub(super) fn restore_comment(&mut self, cx: &mut Cx, task: Option<&str>) {
        let text = task
            .and_then(|id| self.comment_drafts.get(&(self.project.clone(), id.into())))
            .cloned()
            .unwrap_or_default();
        self.view
            .text_input(cx, ids!(comment_editor.source))
            .set_text(cx, &text);
        self.view.label(cx, ids!(comment_draft_hint)).set_text(
            cx,
            if text.is_empty() {
                ""
            } else {
                tr("Unsent draft · kept for this task during this session")
            },
        );
    }
    pub(super) fn current_mxid(&self) -> Option<String> {
        if let Some(client) = crate::sliding_sync::get_client() {
            return client.user_id().map(ToString::to_string);
        }
        #[cfg(debug_assertions)]
        if let Some(account) = self
            .identity
            .strip_prefix("development:")
            .and_then(|s| s.split(':').next())
        {
            return self
                .members
                .iter()
                .find(|id| member_label(id) == account)
                .cloned();
        }
        None
    }
    pub(super) fn handle_collaboration(&mut self, cx: &mut Cx, actions: &Actions) {
        self.collaboration_actions(cx, actions);
        if self
            .view
            .text_input(cx, ids!(comment_editor.source))
            .changed(actions)
            .is_some()
        {
            self.remember_comment(cx);
            let present = !self
                .view
                .text_input(cx, ids!(comment_editor.source))
                .text()
                .is_empty();
            self.view.label(cx, ids!(comment_draft_hint)).set_text(
                cx,
                if present {
                    tr("Unsent draft · kept for this task during this session")
                } else {
                    ""
                },
            );
        }
        if self.view.button(cx, ids!(assign_me)).clicked(actions)
            && let Some(user) = self.current_mxid()
            && let Some(index) = self.members.iter().position(|id| id == &user)
        {
            self.view
                .drop_down(cx, ids!(assignee))
                .set_selected_item(cx, index + 1);
            self.dirty = true;
            self.sync_controls(cx);
        }
        if self.view.button(cx, ids!(my_tasks)).clicked(actions)
            && let Some(user) = self.current_mxid()
        {
            self.query.assignee = Some(user);
            self.reset_viewports(cx);
            self.sync_members(cx, self.members.clone());
            self.load(cx);
        }
    }
}

use base64::{Engine, engine::general_purpose::STANDARD};
#[derive(Debug)]
struct AttachmentSaved {
    identity: String,
    project: String,
    result: Result<(), String>,
}
#[derive(Debug)]
struct PickedAttachment {
    identity: String,
    project: String,
    task: String,
    generation: u64,
    result: Result<AttachmentUpload, String>,
}
impl ProjectBoard {
    pub(super) fn refresh_notifications(&mut self, cx: &mut Cx) {
        if !self.reads.contains(&self.notification_request) {
            self.read(
                cx,
                Read::Notifications(self.notification_before),
                BoardAction::Notifications {
                    before: self.notification_before,
                },
            );
        }
    }
    pub(super) fn apply_collaboration(&mut self, cx: &mut Cx, kind: &Read, value: &Value) {
        match kind {
            Read::Notifications(_) => {
                self.notifications = value["items"].as_array().cloned().unwrap_or_default();
                if self.notification_before == 0 {
                    self.notification_latest = self
                        .notifications
                        .first()
                        .and_then(|v| v["sequence"].as_i64())
                        .unwrap_or(0);
                }
                let unread = value["unread"].as_i64().unwrap_or(0);
                self.view
                    .button(cx, ids!(notifications_button))
                    .set_text(cx, &format!("{} · {}", tr("Notifications"), unread));
                self.view
                    .button(cx, ids!(notifications_older))
                    .set_visible(cx, value["hasMore"] == true);
            }
            Read::NotificationAck => self.refresh_notifications(cx),
            Read::OpenTask(id) => {
                if let Ok(task) = serde_json::from_value::<BoardTask>(value["task"].clone())
                    && task.id == *id
                    && task.project_id == self.project
                {
                    self.select(cx, &task);
                }
            }
            Read::Attachment(task, download)
                if self.selected.as_ref().is_some_and(|t| t.id == *task) =>
            {
                let result = STANDARD.decode(value["data"].as_str().unwrap_or(""));
                let Ok(data) = result else {
                    self.notice(cx, error_message("invalid_server_response"));
                    return;
                };
                if data.len() > 2 * 1024 * 1024 {
                    return;
                }
                if *download {
                    let filename = value["filename"].as_str().unwrap_or("attachment");
                    let identity = self.identity.clone();
                    let project = self.project.clone();
                    if let Err(e) = robius_file_picker::FileDialog::new()
                        .set_file_name(filename)
                        .save_data(data, move |result| {
                            if !matches!(result, Ok(None)) {
                                Cx::post_action(AttachmentSaved {
                                    identity,
                                    project,
                                    result: result.map(|_| ()).map_err(|e| e.to_string()),
                                });
                            }
                        })
                    {
                        self.notice(cx, &e.to_string());
                    }
                } else if let Ok(reader) =
                    ::image::ImageReader::new(std::io::Cursor::new(&data)).with_guessed_format()
                {
                    let mut reader = reader;
                    let mut limits = ::image::Limits::default();
                    limits.max_image_width = Some(8192);
                    limits.max_image_height = Some(8192);
                    limits.max_alloc = Some(64 * 1024 * 1024);
                    reader.limits(limits);
                    if reader.decode().is_ok()
                        && self
                            .view
                            .image(cx, ids!(attachment_preview))
                            .load_image_from_data(cx, &data)
                            .is_ok()
                    {
                        self.view
                            .widget(cx, ids!(attachment_preview))
                            .set_visible(cx, true);
                    } else {
                        self.notice(
                            cx,
                            tr("This image cannot be previewed. Download the original file."),
                        );
                    }
                }
            }
            _ => (),
        }
        self.view.redraw(cx);
    }
    fn pick_attachment(&mut self, cx: &mut Cx) {
        let Some(task) = &self.selected else {
            return;
        };
        if self.pending.is_some() || !self.detail_loaded || task.archived_at.is_some() {
            return;
        }
        let (identity, project, task, generation) = (
            self.identity.clone(),
            self.project.clone(),
            task.id.clone(),
            self.detail_request,
        );
        self.notice(cx, tr("Choose a file to attach to this task."));
        let result = robius_file_picker::FileDialog::new().pick_file(move |picked| {
            if matches!(picked, Ok(None)) {
                return;
            }
            std::thread::spawn(move || {
                let result = (|| {
                    use std::io::Read;
                    let file = picked
                        .map_err(|e| e.to_string())?
                        .ok_or("No file")?
                        .into_local_file()
                        .map_err(|e| e.to_string())?;
                    let filename = file
                        .path()
                        .file_name()
                        .ok_or("Invalid filename")?
                        .to_string_lossy()
                        .into_owned();
                    let mut bytes = Vec::new();
                    std::fs::File::open(file.path())
                        .map_err(|e| e.to_string())?
                        .take(2 * 1024 * 1024 + 1)
                        .read_to_end(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
                        return Err(tr("Choose a non-empty file smaller than 2 MB.").to_owned());
                    }
                    let content_type = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
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
                        content_type: content_type.into(),
                        data: STANDARD.encode(bytes),
                    })
                })();
                Cx::post_action(PickedAttachment {
                    identity,
                    project,
                    task,
                    generation,
                    result,
                });
            });
        });
        if let Err(e) = result {
            self.notice(cx, &e.to_string());
        }
    }
    fn collaboration_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.view.button(cx, ids!(detail_tab)).clicked(actions)
            || self.view.button(cx, ids!(discussion_tab)).clicked(actions)
        {
            self.discussion = self.view.button(cx, ids!(discussion_tab)).clicked(actions);
            self.view
                .view(cx, ids!(editor_scroll))
                .set_scroll_pos(cx, dvec2(0.0, 0.0));
            self.sync_controls(cx);
        }
        if self
            .view
            .button(cx, ids!(notifications_button))
            .clicked(actions)
        {
            self.view
                .view(cx, ids!(notifications_panel))
                .set_visible(cx, true);
            self.view
                .portal_list(cx, ids!(notifications_list))
                .set_first_id_and_scroll(0, 0.0);
            self.notification_before = 0;
            self.refresh_notifications(cx);
        }
        if self
            .view
            .button(cx, ids!(notifications_close))
            .clicked(actions)
        {
            self.view
                .view(cx, ids!(notifications_panel))
                .set_visible(cx, false);
        }
        if self
            .view
            .button(cx, ids!(notifications_read))
            .clicked(actions)
            && self.notification_latest > 0
        {
            self.read(
                cx,
                Read::NotificationAck,
                BoardAction::ReadNotification {
                    input: NotificationRead {
                        sequence: self.notification_latest,
                        all_before: true,
                    },
                },
            );
        }
        if self
            .view
            .button(cx, ids!(notifications_older))
            .clicked(actions)
            && let Some(before) = self
                .notifications
                .last()
                .and_then(|v| v["sequence"].as_i64())
        {
            self.view
                .portal_list(cx, ids!(notifications_list))
                .set_first_id_and_scroll(0, 0.0);
            self.notification_before = before;
            self.read(
                cx,
                Read::Notifications(before),
                BoardAction::Notifications { before },
            );
        }
        if self
            .view
            .button(cx, ids!(upload_attachment))
            .clicked(actions)
        {
            self.pick_attachment(cx);
        }
        for action in actions {
            if let Some(saved) = action.downcast_ref::<AttachmentSaved>()
                && saved.identity == self.identity
                && saved.project == self.project
            {
                match &saved.result {
                    Ok(()) => self.notice(cx, tr("Attachment downloaded")),
                    Err(error) => self.notice(cx, error),
                }
            }
            if let Some(picked) = action.downcast_ref::<PickedAttachment>()
                && picked.identity == self.identity
                && picked.project == self.project
                && picked.generation == self.detail_request
                && self.selected.as_ref().is_some_and(|t| t.id == picked.task)
                && self.pending.is_none()
            {
                match &picked.result {
                    Ok(input) => self.write(
                        cx,
                        BoardAction::UploadAttachment {
                            task: picked.task.clone(),
                            input: input.clone(),
                        },
                    ),
                    Err(error) => self.notice(cx, error),
                }
            }
            if let Some(widget) = action.as_widget_action()
                && matches!(widget.cast::<ButtonAction>(), ButtonAction::Clicked(_))
            {
                if let Some(item) = self.notification_targets.get(&widget.widget_uid).cloned() {
                    if self.pending.is_some() || self.dirty || self.columns_draft.is_some() {
                        self.notice(
                            cx,
                            tr("Save or close your current draft before opening another task."),
                        );
                        continue;
                    }
                    if let (Some(id), Some(seq)) =
                        (item["taskId"].as_str(), item["sequence"].as_i64())
                    {
                        self.read(
                            cx,
                            Read::OpenTask(id.into()),
                            BoardAction::Detail { task: id.into() },
                        );
                        self.read(
                            cx,
                            Read::NotificationAck,
                            BoardAction::ReadNotification {
                                input: NotificationRead {
                                    sequence: seq,
                                    all_before: false,
                                },
                            },
                        );
                        self.view
                            .view(cx, ids!(notifications_panel))
                            .set_visible(cx, false);
                    }
                }
                if let Some((id, download)) =
                    self.attachment_targets.get(&widget.widget_uid).cloned()
                    && let Some(task) = &self.selected
                {
                    self.read(
                        cx,
                        Read::Attachment(task.id.clone(), download),
                        BoardAction::Attachment {
                            task: task.id.clone(),
                            attachment: id,
                        },
                    );
                }
            }
        }
    }
    pub(super) fn draw_collaboration(
        &mut self,
        cx: &mut Cx2d,
        scope: &mut Scope,
        step: &WidgetRef,
    ) -> bool {
        let notifications = step.widget_uid()
            == self
                .view
                .portal_list(cx, ids!(notifications_list))
                .widget_uid();
        let attachments = step.widget_uid()
            == self
                .view
                .portal_list(cx, ids!(attachment_list))
                .widget_uid();
        if !notifications && !attachments {
            return false;
        }
        let Some(mut list) = step.borrow_mut::<PortalList>() else {
            return true;
        };
        let rows = if notifications {
            &self.notifications
        } else {
            &self.attachments
        };
        if notifications {
            self.notification_targets.clear();
        } else {
            self.attachment_targets.clear();
        }
        list.set_item_range(
            cx,
            0,
            if notifications {
                rows.len().max(1)
            } else {
                rows.len()
            },
        );
        while let Some(index) = list.next_visible_item(cx) {
            if let Some(value) = rows.get(index) {
                let row = list.item(cx, index, id!(Item));
                if notifications {
                    row.button(cx, ids!(open)).set_text(
                        cx,
                        &format!(
                            "{} {}",
                            if value["read"] == true { "·" } else { "●" },
                            value["title"].as_str().unwrap_or("")
                        ),
                    );
                    row.label(cx, ids!(detail))
                        .set_text(cx, &activity_text(value));
                    self.notification_targets
                        .insert(row.button(cx, ids!(open)).widget_uid(), value.clone());
                } else {
                    let name = value["filename"].as_str().unwrap_or("");
                    let mime = value["contentType"].as_str().unwrap_or("");
                    row.button(cx, ids!(open)).set_text(
                        cx,
                        &format!(
                            "{} · {} KB",
                            name,
                            value["size"].as_u64().unwrap_or(0).div_ceil(1024)
                        ),
                    );
                    if let Some(id) = value["id"].as_str() {
                        self.attachment_targets.insert(
                            row.button(cx, ids!(open)).widget_uid(),
                            (id.into(), !mime.starts_with("image/")),
                        );
                        self.attachment_targets.insert(
                            row.button(cx, ids!(download)).widget_uid(),
                            (id.into(), true),
                        );
                    }
                }
                row.draw_all(cx, scope);
            } else {
                list.item(
                    cx,
                    index,
                    if notifications && rows.is_empty() && index == 0 {
                        id!(Empty)
                    } else {
                        id!(Filler)
                    },
                )
                .draw_all(cx, scope);
            }
        }
        true
    }
}
pub(super) fn activity_text(value: &Value) -> String {
    let kind = tr(match value["kind"].as_str().unwrap_or("") {
        "created" => "Created task",
        "edited" => "Edited task",
        "moved" => "Moved task",
        "archived" => "Archived task",
        "restored" => "Restored task",
        "commented" => "Added a comment",
        "attachment_added" => "Added an attachment",
        "attachment_removed" => "Removed an attachment",
        _ => "Updated task",
    });
    let mut lines = vec![format!(
        "{} · {} · {}",
        member_label(value["actor"].as_str().unwrap_or("")),
        timestamp(&value["occurredAt"]),
        kind
    )];
    if let Some(fields) = value["details"]["fields"].as_array() {
        for field in fields {
            let key = field["field"].as_str().unwrap_or("");
            let label = tr(match key {
                "title" => "Title",
                "state" => "Status",
                "priority" => "Priority",
                "assigneeMxid" => "Assignee",
                "dueDate" => "Due date",
                "labels" => "Labels",
                "description" => "Description",
                "checklist" => "Checklist",
                "parentId" => "Parent task",
                _ => key,
            });
            if field.get("before").is_some() {
                let text = |v: &Value| {
                    if v.is_null() {
                        "—".into()
                    } else if let Some(s) = v.as_str() {
                        member_label(s).to_owned()
                    } else {
                        v.to_string()
                    }
                };
                lines.push(format!(
                    "{}: {} → {}",
                    label,
                    text(&field["before"]),
                    text(&field["after"])
                ));
            } else {
                lines.push(format!("{} · {}", label, tr("Updated")));
            }
        }
    }
    if let Some(name) = value["details"]["filename"].as_str() {
        lines.push(name.to_owned());
    }
    if let Some(excerpt) = value["details"]["excerpt"].as_str() {
        lines.push(excerpt.to_owned());
    }
    lines.join("\n")
}

pub(super) fn member_choice(id: &str, members: &[String]) -> String {
    let label = member_label(id);
    if members.iter().filter(|m| member_label(m) == label).count() > 1 {
        id.into()
    } else {
        label.into()
    }
}
