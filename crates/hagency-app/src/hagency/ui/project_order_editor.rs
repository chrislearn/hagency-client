//! Project settings owns two independent lists. A drop carries its original
//! project and visible IDs so an intervening navigation or refresh cannot
//! apply an old pointer gesture to a new directory.
use super::*;
use crate::hagency::project_order::{OrderKind, ProjectOrder};

#[derive(Clone, Debug, Default)]
enum OrderAction {
    Dropped {
        identity: String,
        project: String,
        original: Vec<String>,
        ordered: Vec<String>,
    },
    #[default]
    None,
}

struct Drag {
    from: usize,
    start: Vec2d,
    pointer: Option<Vec2d>,
    slot: Option<usize>,
}

#[derive(Script, ScriptHook, Widget)]
pub struct ProjectOrderList {
    #[deref]
    list: FlatList,
    #[rust]
    rows: Vec<(String, String)>,
    #[rust]
    identity: String,
    #[rust]
    project: String,
    #[rust]
    editable: bool,
    #[rust]
    drag: Option<Drag>,
}

impl ProjectOrderList {
    fn bind(&mut self, identity: &str, project: &str, rows: Vec<(String, String)>, editable: bool) {
        if self.rows != rows || self.project != project || self.identity != identity || !editable {
            self.drag = None;
        }
        self.rows = rows;
        self.identity = identity.to_owned();
        self.project = project.to_owned();
        self.editable = editable;
    }

    fn insertion_slot(&self, cx: &Cx, position: Vec2d) -> Option<usize> {
        for (index, (id, _)) in self.rows.iter().enumerate() {
            let item = self.list.items.get(&LiveId::from_str(id))?;
            let area = item.widget.area();
            let rect = area.rect(cx);
            if area.clipped_rect(cx).contains(position) {
                return Some(index + usize::from(position.y >= rect.pos.y + rect.size.y * 0.5));
            }
        }
        None
    }
}

impl Widget for ProjectOrderList {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if matches!(
            event,
            Event::WindowLostFocus(_)
                | Event::KeyDown(KeyEvent {
                    key_code: KeyCode::Escape,
                    ..
                })
        ) {
            if self.drag.take().is_some() {
                self.redraw(cx);
            }
        }
        if self.editable && self.rows.len() > 1 {
            for index in 0..self.rows.len() {
                let Some(item) = self.list.items.get(&LiveId::from_str(&self.rows[index].0)) else {
                    continue;
                };
                let area = if matches!(event, Event::TouchUpdate(_)) {
                    item.widget.view(cx, ids!(handle)).area()
                } else {
                    item.widget.area()
                };
                match event.hits(cx, area) {
                    Hit::FingerHoverIn(_) => cx.set_cursor(MouseCursor::Move),
                    Hit::FingerDown(e) if e.is_primary_hit() => {
                        self.drag = Some(Drag {
                            from: index,
                            start: e.abs,
                            pointer: None,
                            slot: None,
                        });
                    }
                    Hit::FingerMove(e) if self.drag.is_some() => {
                        let slot = self.insertion_slot(cx, e.abs);
                        let drag = self.drag.as_mut().unwrap();
                        if (e.abs - drag.start).length() >= 4.0 {
                            drag.pointer = Some(e.abs);
                        }
                        if drag.pointer.is_some() && slot != drag.slot {
                            drag.slot = slot;
                            self.redraw(cx);
                        }
                    }
                    Hit::FingerUp(e) => {
                        if let Some(drag) = self.drag.take() {
                            if (e.abs - drag.start).length() >= 4.0
                                && let Some(slot) = self.insertion_slot(cx, e.abs)
                            {
                                let to = slot - usize::from(slot > drag.from);
                                if to != drag.from {
                                    let original: Vec<_> =
                                        self.rows.iter().map(|(id, _)| id.clone()).collect();
                                    let mut ordered = original.clone();
                                    let moved = ordered.remove(drag.from);
                                    ordered.insert(to, moved);
                                    cx.widget_action(
                                        self.widget_uid(),
                                        OrderAction::Dropped {
                                            identity: self.identity.clone(),
                                            project: self.project.clone(),
                                            original,
                                            ordered,
                                        },
                                    );
                                }
                            }
                            self.redraw(cx);
                        }
                    }
                    _ => {}
                }
            }
        }
        self.list.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while self.list.draw_walk(cx, scope, walk).is_step() {
            let slot = self.drag.as_ref().and_then(|drag| drag.slot);
            for (index, (id, name)) in self.rows.iter().enumerate() {
                let Some(row) = self.list.item(cx, LiveId::from_str(id), id!(Item)) else {
                    continue;
                };
                row.label(cx, ids!(name)).set_text(cx, name);
                row.view(cx, ids!(handle))
                    .set_visible(cx, self.editable && self.rows.len() > 1);
                row.view(cx, ids!(before))
                    .set_visible(cx, slot == Some(index));
                row.view(cx, ids!(after)).set_visible(
                    cx,
                    index + 1 == self.rows.len() && slot == Some(self.rows.len()),
                );
                row.draw_all(cx, scope);
            }
        }
        DrawStep::done()
    }
}

impl HagencyPanel {
    pub(super) fn scroll_project_order_drag(&mut self, cx: &mut Cx, event: &Event) {
        if !self.view.view(cx, ids!(project_settings_section)).visible()
            || !(matches!(event, Event::MouseMove(_) | Event::TouchUpdate(_))
                || self.order_scroll_frame.is_event(event).is_some())
        {
            return;
        }
        for path in [ids!(board_order_list), ids!(room_order_list)] {
            let widget = self.view.widget(cx, path);
            let Some(mut list) = widget.borrow_mut::<ProjectOrderList>() else {
                continue;
            };
            let Some(position) = list.drag.as_ref().and_then(|drag| drag.pointer) else {
                continue;
            };
            let viewport = self.view.view(cx, ids!(details_scroll));
            let window = viewport.area().clipped_rect(cx);
            if !window.contains(position) {
                return;
            }
            let delta = if position.y < window.pos.y + 32.0 {
                -12.0
            } else if position.y > window.pos.y + window.size.y - 32.0 {
                12.0
            } else {
                return;
            };
            let rows = list.area().rect(cx);
            if !rows.contains(position) {
                return;
            }
            let page = self.view.view(cx, ids!(projects_page)).area().rect(cx);
            let current = (window.pos.y - page.pos.y).max(0.0);
            let target = (current + delta).clamp(0.0, (page.size.y - window.size.y).max(0.0));
            if (target - current).abs() < 0.5 {
                return;
            }
            let slot = list.insertion_slot(cx, position);
            list.drag.as_mut().unwrap().slot = slot;
            viewport.set_scroll_pos(cx, dvec2(0.0, target));
            self.order_scroll_frame = cx.new_next_frame();
            return;
        }
    }

    fn order_rows(&self, kind: OrderKind) -> Vec<(String, String)> {
        let (catalog, key) = match kind {
            OrderKind::Boards => (&self.board_catalog, "id"),
            OrderKind::Rooms => (&self.rooms, "roomId"),
        };
        catalog
            .iter()
            .filter(|item| item["active"] != false)
            .filter_map(|item| {
                let id = item[key].as_str().filter(|id| !id.is_empty())?;
                let name = item["name"]
                    .as_str()
                    .filter(|name| !name.is_empty())
                    .or_else(|| item["displayName"].as_str().filter(|name| !name.is_empty()))
                    .unwrap_or(id);
                let name = if kind == OrderKind::Boards && name == "Main Board" {
                    crate::i18n::tr(name)
                } else {
                    name
                };
                Some((id.to_owned(), name.to_owned()))
            })
            .collect()
    }

    fn can_manage_order(&self) -> bool {
        self.active
            && self.page == Page::Projects
            && !self.display_blocked
            && self.project_home["canManagePolicy"] == true
            && self.project_home["archived"] != true
    }

    pub(super) fn sync_project_order_editor(&self, cx: &mut Cx) {
        let admin = self.can_manage_order() && self.view.view(cx, ids!(project_detail)).visible();
        let settings_tab = self.view.button(cx, ids!(project_settings_tab));
        if settings_tab.visible() != admin {
            settings_tab.set_visible(cx, admin);
        }
        if !self.view.view(cx, ids!(project_settings_section)).visible() {
            return;
        }
        let Some(project) = self.project(cx) else {
            return;
        };
        let editable = admin && !self.busy && ProjectOrder::can_edit(cx, &self.identity, &project);
        let failed = ProjectOrder::load_failed(cx, &self.identity, &project);
        self.view
            .view(cx, ids!(order_error))
            .set_visible(cx, failed);
        let hint = self.view.label(cx, ids!(order_hint));
        let show_hint = editable && (self.board_catalog.len() > 1 || self.rooms.len() > 1);
        if hint.visible() != show_hint {
            hint.set_visible(cx, show_hint);
        }
        for (kind, list, empty) in [
            (OrderKind::Boards, ids!(board_order_list), ids!(no_boards)),
            (OrderKind::Rooms, ids!(room_order_list), ids!(no_rooms)),
        ] {
            let rows = self.order_rows(kind);
            let placeholder = self.view.label(cx, empty);
            let show_empty = rows.is_empty() && !self.busy;
            if placeholder.visible() != show_empty {
                placeholder.set_visible(cx, show_empty);
            }
            if let Some(mut inner) = self.view.widget(cx, list).borrow_mut::<ProjectOrderList>() {
                inner.bind(&self.identity, &project, rows, editable);
            }
        }
    }

    pub(super) fn handle_project_order_actions(&mut self, cx: &mut Cx, actions: &Actions) -> bool {
        if !self.can_manage_order() {
            return false;
        }
        let Some(project_id) = self.project(cx) else {
            return false;
        };
        if self
            .view
            .button(cx, ids!(project_settings_tab))
            .clicked(actions)
        {
            if self.busy || !self.view.project_board(cx, ids!(board)).can_switch(cx) {
                return true;
            }
            self.wanted_project_section = None;
            self.wanted_open_rooms = false;
            self.wanted_access = false;
            self.wanted_new_room = false;
            self.project_section(cx, ids!(project_settings_section));
            self.run(cx, Target::Rooms, Command::ProjectRooms { project_id });
            return true;
        }
        if !self.view.view(cx, ids!(project_settings_section)).visible() {
            return false;
        }
        if self.view.button(cx, ids!(retry_order)).clicked(actions) {
            self.retry_project_order(cx);
            return true;
        }
        for (kind, path) in [
            (OrderKind::Boards, ids!(board_order_list)),
            (OrderKind::Rooms, ids!(room_order_list)),
        ] {
            let uid = self.view.widget(cx, path).widget_uid();
            if let OrderAction::Dropped {
                identity,
                project,
                original,
                ordered,
            } = actions.find_widget_action(uid).cast()
            {
                if identity == self.identity
                    && project == project_id
                    && !self.busy
                    && ProjectOrder::can_edit(cx, &identity, &project)
                    && self
                        .order_rows(kind)
                        .iter()
                        .map(|(id, _)| id)
                        .eq(original.iter())
                {
                    self.save_project_order(cx, kind, ordered);
                }
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use makepad_widgets::makepad_draw::cx_draw::CxDraw;

    fn list() -> (Cx, ProjectOrderList) {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut widget = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            vm.bx.captured_errors = Some(Vec::new());
            let value = script_eval!(vm, {mod.widgets.ProjectOrderList {}});
            let widget = ProjectOrderList::script_from_value(vm, value);
            assert!(vm.take_errors().is_empty());
            widget
        });
        widget.bind(
            "account",
            "project",
            ["a", "b", "c"]
                .into_iter()
                .map(|id| (id.into(), id.into()))
                .collect(),
            true,
        );
        let pass = DrawPass::new(&mut cx);
        pass.set_size(&mut cx, dvec2(280.0, 220.0));
        let mut list = DrawList2d::new(&mut cx);
        let event = DrawEvent::default();
        let mut draw = CxDraw::new(&mut cx, &event);
        let mut cx2d = Cx2d::new(&mut draw);
        cx2d.begin_pass(&pass, None);
        list.begin_always(&mut cx2d);
        cx2d.begin_root_turtle(dvec2(280.0, 220.0), Layout::flow_overlay());
        widget.draw_walk_all(&mut cx2d, &mut Scope::empty(), Walk::fixed(280.0, 220.0));
        cx2d.end_pass_sized_turtle();
        list.end(&mut cx2d);
        cx2d.end_pass(&pass);
        drop(cx2d);
        drop(draw);
        cx.fingers.first_mouse_button = Some((MouseButton::PRIMARY, WindowId(0, 0)));
        (cx, widget)
    }

    fn draw_panel(cx: &mut Cx, panel: &mut HagencyPanel, width: f64) {
        let size = dvec2(width, 640.0);
        cx.redraw_all();
        let pass = DrawPass::new(cx);
        pass.set_size(cx, size);
        let mut list = DrawList2d::new(cx);
        let overlay = Overlay {
            draw_list: DrawList::new(cx),
        };
        let event = DrawEvent {
            redraw_all: true,
            ..Default::default()
        };
        let mut draw = CxDraw::new(cx, &event);
        let mut cx2d = Cx2d::new(&mut draw);
        cx2d.begin_pass(&pass, None);
        list.begin_always(&mut cx2d);
        overlay.begin(&mut cx2d);
        cx2d.begin_root_turtle(size, Layout::flow_overlay());
        panel.draw_walk_all(&mut cx2d, &mut Scope::empty(), Walk::fixed(size.x, size.y));
        overlay.end(&mut cx2d);
        cx2d.end_pass_sized_turtle();
        list.end(&mut cx2d);
        cx2d.end_pass(&pass);
    }

    #[test]
    fn settings_tab_keeps_its_two_lists_after_directory_refresh_and_scrolls_long_lists() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut panel = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            let value = script_eval!(vm, {mod.widgets.HagencyPanel {}});
            HagencyPanel::script_from_value(vm, value)
        });
        panel.active = true;
        panel.identity = "test".into();
        panel.projects = vec![json!({"id":"p","name":"Project","spaceId":"!p:test"})];
        panel.labels(
            &mut cx,
            ids!(projects),
            "Select a Project",
            &panel.projects,
            "id",
        );
        panel.project_detail(&mut cx, 0);
        panel.project_home = json!({"projectId":"p","canManagePolicy":true,"archived":false});
        panel.board_catalog = (0..20).map(|i| json!({"id":format!("b{i}"),"name":format!("Planning board with a long name {i}")})).collect();
        panel.wanted_project_section = Some(("p".into(), ProjectDetailSection::Rooms));
        panel.wanted_open_rooms = true;
        panel.render_project_controls(&mut cx);
        panel
            .view
            .view(&cx, ids!(project_detail))
            .set_visible(&mut cx, true);
        draw_panel(&mut cx, &mut panel, 375.0);
        let tab = panel.view.button(&cx, ids!(project_settings_tab));
        assert!(tab.visible());
        let rect = tab.area().clipped_rect(&cx);
        assert!(
            rect.size.x > 40.0 && rect.size.y >= 25.0 && rect.pos.x + rect.size.x <= 375.0,
            "settings is reachable on a narrow wrapped toolbar: {rect:?}"
        );
        let click = cx.capture_actions(|cx| {
            cx.widget_action(tab.widget_uid(), ButtonAction::Clicked(Default::default()))
        });
        assert!(panel.handle_project_order_actions(&mut cx, &click));
        panel.apply(
            &mut cx,
            Target::Rooms,
            &json!({"rooms":[{"roomId":"!r:test","name":"Room","active":true}]}),
        );
        assert!(
            panel
                .view
                .view(&cx, ids!(project_settings_section))
                .visible()
        );
        assert!(!panel.view.view(&cx, ids!(rooms_section)).visible());
        assert!(!panel.wanted_open_rooms);
        draw_panel(&mut cx, &mut panel, 375.0);
        let room_list = panel.view.widget(&cx, ids!(room_order_list));
        assert_eq!(
            room_list.borrow::<ProjectOrderList>().unwrap().rows[0].0,
            "!r:test"
        );
        let viewport = panel.view.view(&cx, ids!(details_scroll));
        let rect = viewport.area().clipped_rect(&cx);
        let page = panel.view.view(&cx, ids!(projects_page));
        let before = page.area().rect(&cx);
        let point = dvec2(rect.pos.x + 64.0, rect.pos.y + rect.size.y - 6.0);
        let boards = panel.view.widget(&cx, ids!(board_order_list));
        {
            let mut list = boards.borrow_mut::<ProjectOrderList>().unwrap();
            let row = list.list.items[LiveId::from_str("b0")]
                .widget
                .area()
                .rect(&cx);
            assert!(row.size.x > 0.0 && row.pos.x + row.size.x <= 375.0);
            list.drag = Some(Drag {
                from: 0,
                start: row.pos,
                pointer: Some(point),
                slot: Some(1),
            });
        }
        panel.scroll_project_order_drag(
            &mut cx,
            &Event::MouseMove(MouseMoveEvent {
                abs: point,
                lock_delta: Default::default(),
                window_id: WindowId(0, 0),
                modifiers: Default::default(),
                handled: Cell::new(Area::Empty),
                time: 1.0,
            }),
        );
        draw_panel(&mut cx, &mut panel, 375.0);
        assert!(
            page.area().rect(&cx).pos.y < before.pos.y,
            "drag at the viewport edge scrolls toward offscreen items"
        );
    }

    #[test]
    fn pointer_drop_reorders_only_inside_the_original_project_list() {
        for (end_y, switch_project, expected) in [
            (119.0, false, true),
            (119.0, true, false),
            (180.0, false, false),
        ] {
            let (mut cx, mut widget) = list();
            let window_id = WindowId(0, 0);
            let start = widget.list.items[LiveId::from_str("a")]
                .widget
                .area()
                .rect(&cx)
                .pos
                + dvec2(100.0, 10.0);
            let end = dvec2(start.x, end_y);
            widget.handle_event(
                &mut cx,
                &Event::MouseDown(MouseDownEvent {
                    abs: start,
                    button: MouseButton::PRIMARY,
                    window_id,
                    modifiers: Default::default(),
                    handled: Cell::new(Area::Empty),
                    time: 1.0,
                }),
                &mut Scope::empty(),
            );
            assert!(
                widget.drag.is_some(),
                "a mouse press on the row body begins the gesture"
            );
            widget.handle_event(
                &mut cx,
                &Event::MouseMove(MouseMoveEvent {
                    abs: end,
                    lock_delta: Default::default(),
                    window_id,
                    modifiers: Default::default(),
                    handled: Cell::new(Area::Empty),
                    time: 1.1,
                }),
                &mut Scope::empty(),
            );
            if switch_project {
                widget.bind("account", "another-project", widget.rows.clone(), true);
            }
            let actions = cx.capture_actions(|cx| {
                widget.handle_event(
                    cx,
                    &Event::MouseUp(MouseUpEvent {
                        abs: end,
                        button: MouseButton::PRIMARY,
                        window_id,
                        modifiers: Default::default(),
                        time: 1.2,
                    }),
                    &mut Scope::empty(),
                )
            });
            match actions
                .find_widget_action(widget.widget_uid())
                .cast::<OrderAction>()
            {
                OrderAction::Dropped {
                    identity,
                    project,
                    original,
                    ordered,
                } => {
                    assert!(expected);
                    assert_eq!(
                        (identity.as_str(), project.as_str()),
                        ("account", "project")
                    );
                    assert_eq!(original, ["a", "b", "c"]);
                    assert_eq!(ordered, ["b", "c", "a"]);
                }
                OrderAction::None => assert!(!expected),
            }
            assert!(widget.drag.is_none());
        }
    }
}
