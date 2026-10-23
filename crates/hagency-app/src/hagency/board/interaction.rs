use super::*;

impl Widget for ProjectBoard {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if !self.active {
            // Read-only design previews must still permit viewport interaction.
            self.view.handle_event(cx, event, scope);
            return;
        }
        if !self
            .connection()
            .is_ok_and(|s| s.identity() == self.identity)
        {
            self.clear_content(cx);
            self.pending = None;
            self.retained.clear();
            self.active = false;
            self.epoch = nonce();
            cx.stop_timer(self.timer);
            return;
        }
        match event {
            Event::WindowLostFocus(_) => {
                self.foreground = false;
                cx.stop_timer(self.timer);
            }
            Event::WindowGotFocus(_) => {
                self.foreground = true;
                if !self.loading {
                    self.load(cx);
                }
            }
            Event::KeyDown(e) if e.key_code == KeyCode::Escape => {
                self.drag = None;
                self.view
                    .label(cx, ids!(drag_notice))
                    .set_visible(cx, false);
                self.view.redraw(cx);
            }
            _ => (),
        }
        if self.timer.is_event(event).is_some() {
            self.timer = Timer::default();
            if !self.loading && !self.saving && self.drag.is_none() {
                if let Some(snapshot) = &self.snapshot {
                    self.read(
                        cx,
                        Read::Changes,
                        BoardAction::Changes {
                            after: snapshot.sequence,
                        },
                    );
                } else {
                    self.load(cx);
                }
            } else {
                self.schedule(cx);
            }
        }
        if self.drag_frame.is_event(event).is_some()
            && let Some(drag) = self.drag.clone()
            && drag.moved
        {
            let viewport = self.view.view(cx, ids!(board_scroll)).area().rect(cx);
            let step = if drag.at.x < viewport.pos.x + 36.0 {
                -12.0
            } else if drag.at.x > viewport.pos.x + viewport.size.x - 36.0 {
                12.0
            } else {
                0.0
            };
            if step != 0.0 {
                let visible: Vec<_> = (0..5)
                    .filter(|i| self.view.view(cx, column_path(*i)).visible())
                    .collect();
                let actual = visible
                    .first()
                    .map(|i| {
                        viewport.pos.x - self.view.view(cx, column_path(*i)).area().rect(cx).pos.x
                    })
                    .unwrap_or(0.0);
                let extent = (visible.len() as f64 * 272.0 - 12.0 - viewport.size.x).max(0.0);
                self.scroll_x = (actual + step).clamp(0.0, extent);
                self.view
                    .view(cx, ids!(board_scroll))
                    .set_scroll_pos(cx, dvec2(self.scroll_x, 0.0));
            }
            for index in 0..5 {
                let column = self.view.view(cx, column_path(index));
                let list = column.portal_list(cx, ids!(cards));
                let rect = list.area().rect(cx);
                if rect.contains(drag.at) {
                    let step = if drag.at.y < rect.pos.y + 40.0 {
                        10.0
                    } else if drag.at.y > rect.pos.y + rect.size.y - 40.0 {
                        -10.0
                    } else {
                        0.0
                    };
                    if step != 0.0
                        && let Some(mut inner) = list.borrow_mut()
                    {
                        let first = inner.first_id();
                        let scroll = inner.first_scroll();
                        inner.set_first_id_and_scroll(first, scroll + step);
                        inner.redraw(cx);
                    }
                }
            }
            self.drag_frame = cx.new_next_frame();
            self.view.redraw(cx);
        }
        self.view.handle_event(cx, event, scope);
        let Event::Actions(actions) = event else {
            return;
        };
        for action in actions {
            if let Some(done) = action.downcast_ref::<Completed>() {
                self.apply_read(cx, done);
            }
            if let Some(done) = action.downcast_ref::<Saved>() {
                self.apply_saved(cx, done);
            }
            if let Some(action) = action.as_widget_action() {
                if let Some(task) = self.card_targets.get(&action.widget_uid).cloned() {
                    match action.cast::<CardAction>() {
                        CardAction::Open => self.select(cx, &task),
                        CardAction::Start(abs)
                            if self.pending.is_none() && !self.dirty && !self.query.archived =>
                        {
                            self.drag = Some(Drag {
                                task,
                                start: abs,
                                at: abs,
                                moved: false,
                            });
                        }
                        CardAction::Drag(abs) => {
                            if let Some(drag) = &mut self.drag {
                                drag.at = abs;
                                drag.moved |= (abs - drag.start).length() > 6.0;
                                if drag.moved {
                                    self.drag_frame = cx.new_next_frame();
                                    self.view.label(cx, ids!(drag_notice)).set_visible(cx, true);
                                    self.view.redraw(cx);
                                }
                            }
                        }
                        CardAction::Drop(abs) => {
                            if let Some(drag) = self.drag.take()
                                && drag.moved
                            {
                                if let Some(index) = (0..5).find(|i| {
                                    self.view
                                        .view(cx, column_path(*i))
                                        .area()
                                        .rect(cx)
                                        .contains(abs)
                                }) {
                                    let filtered = self.query.query.is_some()
                                        || self.query.label.is_some()
                                        || self.query.assignee.is_some()
                                        || self.query.overdue;
                                    let complete = self
                                        .snapshot
                                        .as_ref()
                                        .is_some_and(|s| s.lanes[index].next_cursor.is_none());
                                    let target =
                                        self.drop_neighbours(cx, index, abs, &drag.task.id);
                                    if !filtered
                                        && complete
                                        && let Some((before, after)) = target
                                    {
                                        self.move_task(
                                            cx,
                                            drag.task,
                                            STATES[index],
                                            Position::Between,
                                            before,
                                            after,
                                        );
                                    } else {
                                        self.move_task(
                                            cx,
                                            drag.task,
                                            STATES[index],
                                            Position::Append,
                                            None,
                                            None,
                                        );
                                    }
                                }
                            }
                            self.view
                                .label(cx, ids!(drag_notice))
                                .set_visible(cx, false);
                            self.view.redraw(cx);
                        }
                        _ => (),
                    }
                }
                if self.more_targets.contains_key(&action.widget_uid) {
                    if let Some(index) = self.more_targets.get(&action.widget_uid).copied()
                        && matches!(action.cast::<ButtonAction>(), ButtonAction::Clicked(..))
                    {
                        self.load_more(cx, index);
                    }
                }
            }
        }
        for id in [ids!(title), ids!(description), ids!(labels), ids!(due_date)] {
            if self.view.text_input(cx, id).changed(actions).is_some() {
                self.dirty = true;
                self.discard_armed = false;
            }
        }
        if self
            .view
            .drop_down(cx, ids!(priority))
            .changed(actions)
            .is_some()
            || self
                .view
                .drop_down(cx, ids!(assignee))
                .changed(actions)
                .is_some()
        {
            self.dirty = true;
            self.discard_armed = false;
        }
        if self
            .view
            .drop_down(cx, ids!(new_status))
            .changed(actions)
            .is_some()
            && self.creating
        {
            self.dirty = true;
            self.discard_armed = false;
        }
        for index in 0..5 {
            if self
                .view
                .view(cx, column_path(index))
                .button(cx, ids!(add))
                .clicked(actions)
            {
                self.new_task(cx);
                if self.creating && !self.dirty {
                    self.view
                        .drop_down(cx, ids!(new_status))
                        .set_selected_item(cx, index);
                }
            }
        }
        if self.view.button(cx, ids!(clear_filters)).clicked(actions) && !self.loading {
            self.query = Query::default();
            self.view.text_input(cx, ids!(search)).set_text(cx, "");
            self.view
                .text_input(cx, ids!(filter_label))
                .set_text(cx, "");
            self.view
                .drop_down(cx, ids!(filter_state))
                .set_selected_item(cx, 0);
            self.view
                .drop_down(cx, ids!(filter_assignee))
                .set_selected_item(cx, 0);
            self.view
                .check_box(cx, ids!(overdue))
                .set_active(cx, false, Animate::No);
            self.view
                .check_box(cx, ids!(archived))
                .set_active(cx, false, Animate::No);
            self.reset_viewports(cx);
            self.load(cx);
        }
        if self.view.button(cx, ids!(refresh)).clicked(actions) {
            self.failed = false;
            self.load(cx);
        }
        if self.view.button(cx, ids!(retry)).clicked(actions) {
            self.retry_write(cx);
        }
        if self.view.button(cx, ids!(new_task)).clicked(actions) {
            self.new_task(cx);
        }
        if self.view.button(cx, ids!(apply_filters)).clicked(actions)
            || self
                .view
                .text_input(cx, ids!(search))
                .returned(actions)
                .is_some()
            || self
                .view
                .check_box(cx, ids!(overdue))
                .changed(actions)
                .is_some()
            || self
                .view
                .check_box(cx, ids!(archived))
                .changed(actions)
                .is_some()
        {
            if !self.loading {
                let search = self.view.text_input(cx, ids!(search)).text();
                let label = self.view.text_input(cx, ids!(filter_label)).text();
                let member = self
                    .view
                    .drop_down(cx, ids!(filter_assignee))
                    .selected_item();
                self.query = Query {
                    state: self
                        .view
                        .drop_down(cx, ids!(filter_state))
                        .selected_item()
                        .checked_sub(1)
                        .and_then(|i| STATES.get(i))
                        .map(|s| (*s).into()),
                    query: if search.trim().is_empty() {
                        None
                    } else {
                        Some(search.trim().into())
                    },
                    label: if label.trim().is_empty() {
                        None
                    } else {
                        Some(label.trim().into())
                    },
                    assignee: member
                        .checked_sub(1)
                        .and_then(|i| self.members.get(i).cloned()),
                    overdue: self.view.check_box(cx, ids!(overdue)).active(cx),
                    archived: self.view.check_box(cx, ids!(archived)).active(cx),
                    ..Default::default()
                };
                if self.query.validate().is_ok() {
                    self.reset_viewports(cx);
                    self.load(cx);
                } else {
                    self.notice(
                        cx,
                        tr("Keep searches under 120 characters and labels under 32 characters."),
                    );
                }
            }
        }
        if self.view.button(cx, ids!(close_detail)).clicked(actions) {
            if self.pending.is_some() {
                self.notice(cx,tr("A previous save is unconfirmed. Retry its original request to avoid duplicates."));
            } else if self.dirty && !self.discard_armed {
                self.discard_armed = true;
                self.notice(
                    cx,
                    tr("You have unsaved changes. Save them, or close again to discard the draft."),
                );
            } else {
                self.selected = None;
                self.creating = false;
                self.dirty = false;
                self.discard_armed = false;
                self.detail_request = nonce();
                self.view.view(cx, ids!(inspector)).set_visible(cx, false);
                self.notice(cx, "");
            }
        }
        if self.view.button(cx, ids!(reload_latest)).clicked(actions)
            && let Some(task) = self.selected.clone()
        {
            self.dirty = false;
            self.conflict = false;
            self.select(cx, &task);
        }
        if self.view.button(cx, ids!(save)).clicked(actions) && self.pending.is_none() {
            let mut fields = self.fields(cx);
            if let Err(e) = fields.validate() {
                self.notice(cx, error_message(e.0));
            } else if self.creating {
                self.write(
                    cx,
                    BoardAction::Create {
                        input: Create {
                            idempotency_key: uuid::Uuid::new_v4().to_string(),
                            fields,
                            state: STATES[self
                                .view
                                .drop_down(cx, ids!(new_status))
                                .selected_item()
                                .min(4)]
                            .into(),
                        },
                    },
                );
            } else if let Some(task) = &self.selected {
                self.write(
                    cx,
                    BoardAction::Update {
                        task: task.id.clone(),
                        input: Update {
                            idempotency_key: uuid::Uuid::new_v4().to_string(),
                            expected_version: task.version,
                            fields,
                        },
                    },
                );
            }
        }
        if self.view.button(cx, ids!(move_top)).clicked(actions)
            || self.view.button(cx, ids!(move_bottom)).clicked(actions)
        {
            if self.dirty {
                self.notice(
                    cx,
                    tr("Save or close your current draft before opening another task."),
                );
            } else if let Some(task) = self.selected.clone() {
                let state = STATES[self
                    .view
                    .drop_down(cx, ids!(move_state))
                    .selected_item()
                    .min(4)];
                let position = if self.view.button(cx, ids!(move_top)).clicked(actions) {
                    Position::Prepend
                } else {
                    Position::Append
                };
                self.move_task(cx, task, state, position, None, None);
            }
        }
        if self.view.button(cx, ids!(archive)).clicked(actions)
            && let Some(task) = &self.selected
        {
            let input = Versioned {
                idempotency_key: uuid::Uuid::new_v4().to_string(),
                expected_version: task.version,
            };
            let action = if task.archived_at.is_some() {
                BoardAction::Restore {
                    task: task.id.clone(),
                    input,
                }
            } else {
                BoardAction::Archive {
                    task: task.id.clone(),
                    input,
                }
            };
            self.write(cx, action);
        }
        if self.view.button(cx, ids!(add_comment)).clicked(actions)
            && let Some(task) = &self.selected
        {
            let body = self.view.text_input(cx, ids!(comment)).text();
            if !body.trim().is_empty() && body.len() <= 8192 {
                self.write(
                    cx,
                    BoardAction::Comment {
                        task: task.id.clone(),
                        input: Comment {
                            idempotency_key: uuid::Uuid::new_v4().to_string(),
                            body,
                        },
                    },
                );
            } else {
                self.notice(cx, tr("Comments must contain 1–8192 bytes."));
            }
        }
        if self.view.button(cx, ids!(more_comments)).clicked(actions)
            && let Some(task) = &self.selected
        {
            let id = task.id.clone();
            self.read(
                cx,
                Read::Comments(id.clone(), true),
                BoardAction::Comments {
                    task: id,
                    after: self.comments_after,
                },
            );
        }
        if self.view.button(cx, ids!(more_activity)).clicked(actions)
            && let Some(task) = &self.selected
        {
            let id = task.id.clone();
            self.read(
                cx,
                Read::Activity(id.clone(), true),
                BoardAction::Activity {
                    task: id,
                    after: self.activity_after,
                },
            );
        }
        if let Some(task) = &self.selected {
            let id = task.id.clone();
            if self.view.button(cx, ids!(first_comments)).clicked(actions) {
                self.read(
                    cx,
                    Read::Comments(id.clone(), false),
                    BoardAction::Comments {
                        task: id.clone(),
                        after: 0,
                    },
                );
            }
            if self.view.button(cx, ids!(first_activity)).clicked(actions) {
                self.read(
                    cx,
                    Read::Activity(id.clone(), false),
                    BoardAction::Activity { task: id, after: 0 },
                );
            }
        }
        self.sync_controls(cx);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.card_targets.clear();
        self.card_areas.clear();
        self.more_targets.clear();
        if let Some(drag) = &self.drag
            && drag.moved
        {
            let size = cx.current_pass_size();
            let at = dvec2(
                (drag.at.x + 16.0).clamp(8.0, (size.x - 270.0).max(8.0)),
                (drag.at.y + 18.0).clamp(8.0, (size.y - 90.0).max(8.0)),
            );
            self.view.tooltip(cx, ids!(drag_preview)).show_with_options(
                cx,
                at,
                &format!("#{} · {}", drag.task.number, drag.task.title),
            );
        } else {
            self.view.tooltip(cx, ids!(drag_preview)).hide(cx);
        }

        let width = cx.turtle().rect().size.x;
        let inspector = self.view.view(cx, ids!(inspector)).visible();
        self.view
            .view(cx, ids!(board_scroll))
            .set_visible(cx, !inspector || width >= 850.0);
        let mut panel = self.view.view(cx, ids!(inspector));
        let size = if width < 850.0 {
            Size::fill()
        } else {
            Size::Fixed(340.0)
        };
        script_apply_eval!(cx,panel,{width:#(size)});
        for (index, state) in STATES.iter().enumerate() {
            let column = self.view.view(cx, column_path(index));
            column.set_visible(
                cx,
                self.query
                    .state
                    .as_ref()
                    .is_none_or(|filter| filter == *state),
            );
            column
                .label(cx, ids!(heading))
                .set_text(cx, state_label(state));
            let mut marker = column.view(cx, ids!(marker));
            match index {
                1 => {
                    script_apply_eval!(cx,marker,{draw_bg +: {color:mod.widgets.RINX_ACCENT}});
                }
                2 => {
                    script_apply_eval!(cx,marker,{draw_bg +: {color:mod.widgets.RINX_WARNING_FG}});
                }
                3 => {
                    script_apply_eval!(cx,marker,{draw_bg +: {color:#9675bf}});
                }
                4 => {
                    script_apply_eval!(cx,marker,{draw_bg +: {color:mod.widgets.RINX_SUCCESS_FG}});
                }
                _ => {
                    script_apply_eval!(cx,marker,{draw_bg +: {color:mod.widgets.RINX_MUTED}});
                }
            }

            column.button(cx, ids!(add)).set_enabled(
                cx,
                self.snapshot.is_some()
                    && self.pending.is_none()
                    && !self.dirty
                    && !self.query.archived,
            );
            let lane = self.snapshot.as_ref().and_then(|s| s.lanes.get(index));
            column.label(cx, ids!(count)).set_text(
                cx,
                &lane
                    .map(|l| l.total.to_string())
                    .unwrap_or_else(|| "—".into()),
            );
            let mut column = column;
            let highlighted = self
                .drag
                .as_ref()
                .is_some_and(|d| d.moved && column.area().rect(cx).contains(d.at));
            script_apply_eval!(cx,column,{draw_bg +: {border_size:#(if highlighted{2.0}else{0.0}) border_color:mod.widgets.RINX_ACCENT}});
        }
        while let Some(step) = self.view.draw_walk(cx, scope, walk).step() {
            let index = (0..5).find(|i| {
                step.widget_uid()
                    == self
                        .view
                        .view(cx, column_path(*i))
                        .portal_list(cx, ids!(cards))
                        .widget_uid()
            });
            let Some(index) = index else {
                continue;
            };
            let Some(mut list) = step.borrow_mut::<PortalList>() else {
                continue;
            };
            let lane = self.snapshot.as_ref().and_then(|s| s.lanes.get(index));
            let total = lane.map_or(0, |l| l.tasks.len());
            let more = lane.is_some_and(|l| l.next_cursor.is_some());
            if total == 0 || list.first_id() >= total {
                list.set_first_id_and_scroll(0, 0.0);
            }
            list.set_item_range(cx, 0, total.max(1) + usize::from(more));
            while let Some(row_index) = list.next_visible_item(cx) {
                if let Some(task) = lane.and_then(|l| l.tasks.get(row_index)) {
                    let card = list.item(cx, row_index, id!(Card));
                    self.card_targets.insert(card.widget_uid(), task.clone());
                    card.label(cx, ids!(number))
                        .set_text(cx, &format!("#{}", task.number));
                    card.label(cx, ids!(title)).set_text(cx, &task.title);
                    card.label(cx, ids!(priority))
                        .set_text(cx, priority_label(&task.priority));
                    card.label(cx, ids!(tags))
                        .set_text(cx, &task.labels.join(" · "));
                    card.label(cx, ids!(tags))
                        .set_visible(cx, !task.labels.is_empty());
                    card.label(cx, ids!(assignee)).set_text(
                        cx,
                        task.assignee_mxid
                            .as_deref()
                            .map(member_label)
                            .unwrap_or_else(|| tr("Unassigned")),
                    );
                    card.label(cx, ids!(due))
                        .set_text(cx, task.due_date.as_deref().unwrap_or(""));
                    let overdue = task.due_date.as_deref().is_some_and(|date| {
                        date < chrono::Utc::now().format("%Y-%m-%d").to_string().as_str()
                    }) && task.state != "done"
                        && task.archived_at.is_none();
                    let mut due = card.label(cx, ids!(due));
                    if overdue {
                        script_apply_eval!(cx,due,{draw_text.color:mod.widgets.RINX_WARNING_FG});
                        due.set_text(
                            cx,
                            &format!(
                                "{} · {}",
                                tr("Overdue"),
                                task.due_date.as_deref().unwrap_or("")
                            ),
                        );
                    } else {
                        script_apply_eval!(cx,due,{draw_text.color:mod.widgets.RINX_MUTED});
                    }
                    let mut priority = card.label(cx, ids!(priority));
                    if task.priority == "urgent" {
                        script_apply_eval!(cx,priority,{draw_text.color:mod.widgets.RINX_WARNING_FG});
                    } else if task.priority == "high" {
                        script_apply_eval!(cx,priority,{draw_text.color:mod.widgets.RINX_ACCENT});
                    } else {
                        script_apply_eval!(cx,priority,{draw_text.color:mod.widgets.RINX_MUTED});
                    }
                    let mut card_bg = card.clone();
                    let selected = self.selected.as_ref().is_some_and(|t| t.id == task.id);
                    if selected {
                        script_apply_eval!(cx,card_bg,{draw_bg +: {border_color:mod.widgets.RINX_ACCENT}});
                    } else {
                        script_apply_eval!(cx,card_bg,{draw_bg +: {border_color:mod.widgets.RINX_BORDER}});
                    }
                    card.draw_all(cx, scope);
                    self.card_areas.insert(task.id.clone(), card.area());
                } else if more && row_index == total {
                    let row = list.item(cx, row_index, id!(More));
                    self.more_targets
                        .insert(row.button(cx, ids!(more)).widget_uid(), index);
                    row.button(cx, ids!(more)).set_enabled(cx, !self.loading);
                    row.draw_all(cx, scope);
                } else {
                    list.item(
                        cx,
                        row_index,
                        if row_index == 0 && self.snapshot.is_some() {
                            id!(Empty)
                        } else {
                            id!(Filler)
                        },
                    )
                    .draw_all(cx, scope);
                }
            }
        }
        DrawStep::done()
    }
}
impl ProjectBoard {
    pub(super) fn load_more(&mut self, cx: &mut Cx, index: usize) {
        if let Some(snapshot) = &self.snapshot
            && let Some(cursor) = snapshot.lanes[index].next_cursor.clone()
        {
            let mut query = self.query.clone();
            query.state = Some(STATES[index].into());
            query.sequence = Some(snapshot.sequence);
            query.cursor = Some(cursor);
            self.read(cx, Read::Page(index), BoardAction::Page { query });
        }
    }
    pub(super) fn drop_neighbours(
        &self,
        cx: &mut Cx,
        index: usize,
        abs: Vec2d,
        moving: &str,
    ) -> Option<(Option<String>, Option<String>)> {
        let lane = self.snapshot.as_ref()?.lanes.get(index)?;
        let tasks: Vec<_> = lane.tasks.iter().filter(|t| t.id != moving).collect();
        let next = tasks.iter().position(|t| {
            self.card_areas.get(&t.id).is_some_and(|area| {
                let rect = area.rect(cx);
                rect.size.y > 0.0 && abs.y < rect.pos.y + rect.size.y / 2.0
            })
        });
        match next {
            Some(i) => Some((
                Some(tasks[i].id.clone()),
                i.checked_sub(1).map(|j| tasks[j].id.clone()),
            )),
            None => {
                let viewport = self
                    .view
                    .view(cx, column_path(index))
                    .portal_list(cx, ids!(cards))
                    .area()
                    .rect(cx);
                let last = tasks
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| {
                        self.card_areas.get(&t.id).is_some_and(|area| {
                            let rect = area.rect(cx);
                            rect.size.y > 0.0
                                && rect.pos.y < viewport.pos.y + viewport.size.y
                                && rect.pos.y + rect.size.y > viewport.pos.y
                        })
                    })
                    .map(|(i, _)| i)
                    .max();
                last.map(|i| {
                    (
                        tasks.get(i + 1).map(|t| t.id.clone()),
                        Some(tasks[i].id.clone()),
                    )
                })
            }
        }
    }
}
