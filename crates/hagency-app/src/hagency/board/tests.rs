use super::*;
use makepad_widgets::event::{ScrollEvent, ScrollPhase};
use serde_json::json;
use makepad_widgets::makepad_draw::cx_draw::CxDraw;
fn task(id: &str, state: &str) -> BoardTask {
    serde_json::from_value(json!({"id":id,"projectId":"p1","number":1,"title":"Design a clear, shared project board","description":"Acceptance criteria and context.","state":state,"rank":1024,"priority":"high","assigneeMxid":"@alice:test","labels":["design","desktop"],"dueDate":"2026-11-01","version":1,"createdBy":"@alice:test","createdAt":1,"updatedAt":1,"archivedAt":null})).unwrap()
}
fn panel() -> (Cx, ProjectBoard) {
    let mut cx = Cx::new(Box::new(|_, _| {}));
    let board = cx.with_vm(|vm| {
        makepad_widgets::script_mod(vm);
        crate::app::register_widgets(vm);
        vm.bx.captured_errors = Some(Vec::new());
        let value = script_eval!(vm,{mod.widgets.ProjectBoard {}});
        let board = ProjectBoard::script_from_value(vm, value);
        let errors = vm.take_errors();
        assert!(errors.is_empty(), "{errors:?}");
        board
    });
    (cx, board)
}
fn populate(board: &mut ProjectBoard) {
    board.identity = "account".into();
    board.project = "p1".into();
    board.epoch = 1;
    board.members = vec!["@alice:test".into(), "@bob:test".into()];
    board.snapshot = Some(Snapshot {
        project_id: "p1".into(),
        sequence: 1,
        capability: CAPABILITY.into(),
        members: board.members.clone(),
        lanes: STATES
            .iter()
            .map(|state| ::hagency::project_board::Lane {
                state: (*state).into(),
                total: 1,
                tasks: vec![task(&format!("task_{state}"), state)],
                next_cursor: None,
                sequence: 1,
            })
            .collect(),
    });
}
fn draw(cx: &mut Cx, board: &mut ProjectBoard, width: f64, height: f64) {
    cx.redraw_all();
    let pass = DrawPass::new(cx);
    pass.set_size(cx, dvec2(width, height));
    let mut dl = DrawList2d::new(cx);
    let overlay = Overlay {
        draw_list: DrawList::new(cx),
    };
    let event = DrawEvent::default();
    let mut draw = CxDraw::new(cx, &event);
    let mut cx2d = Cx2d::new(&mut draw);
    cx2d.begin_pass(&pass, None);
    dl.begin_always(&mut cx2d);
    overlay.begin(&mut cx2d);
    cx2d.begin_root_turtle(dvec2(width, height), Layout::flow_overlay());
    board.draw_walk_all(&mut cx2d, &mut Scope::empty(), Walk::fixed(width, height));
    overlay.end(&mut cx2d);
    cx2d.end_pass_sized_turtle();
    dl.end(&mut cx2d);
    cx2d.end_pass(&pass);
}
#[test]
fn kanban_script_layout_and_narrow_inspector_use_the_available_viewport() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    board.sync_members(&mut cx, board.members.clone());
    for width in [600.0, 1100.0, 1500.0] {
        board
            .view
            .view(&cx, ids!(inspector))
            .set_visible(&mut cx, false);
        draw(&mut cx, &mut board, width, 760.0);
        let rect = board.view.view(&cx, ids!(board_scroll)).area().rect(&cx);
        assert!(rect.size.y > 500.0, "{rect:?}");
        for index in 0..2 {
            let column = board.view.view(&cx, column_path(index));
            let rect = column.area().rect(&cx);
            assert!((rect.size.x - 260.0).abs() < 1.0, "{rect:?}");
            assert!(rect.size.y > 500.0);
        }
        let card = board
            .view
            .view(&cx, ids!(todo))
            .portal_list(&cx, ids!(cards))
            .item(&mut cx, 0, id!(Card));
        assert!(card.area().rect(&cx).size.y > 90.0);
        board
            .view
            .view(&cx, ids!(inspector))
            .set_visible(&mut cx, true);
        board.fill_editor(&mut cx, task("task_todo", "todo").fields());
        draw(&mut cx, &mut board, width, 760.0);
        let panel = board.view.view(&cx, ids!(inspector)).area().rect(&cx);
        assert!(panel.pos.x + panel.size.x <= width + 1.0, "{panel:?}");
        if width < 850.0 {
            assert!(!board.view.view(&cx, ids!(board_scroll)).visible());
            assert!(panel.size.x > width - 30.0);
        } else {
            assert!((panel.size.x - 340.0).abs() < 1.0);
        }
    }
}
#[test]
fn kanban_old_account_project_and_request_replies_cannot_replace_current_content() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    board.reads.insert(11);
    let done = Completed {
        identity: "old-account".into(),
        project: "p1".into(),
        epoch: 1,
        request: 11,
        kind: Read::Snapshot,
        detail_generation: 0,
        result: Err("project_membership_required".into()),
    };
    board.apply_read(&mut cx, &done);
    assert!(board.snapshot.is_some());
    assert!(board.reads.contains(&11));
    let done = Completed {
        identity: "account".into(),
        project: "another-project".into(),
        ..done
    };
    board.apply_read(&mut cx, &done);
    assert!(board.snapshot.is_some());
    let done = Completed {
        project: "p1".into(),
        epoch: 0,
        ..done
    };
    board.apply_read(&mut cx, &done);
    assert!(board.snapshot.is_some());
    let done = Completed { epoch: 1, ..done };
    board.apply_read(&mut cx, &done);
    assert!(board.snapshot.is_none());
    assert!(board.members.is_empty());
}
#[test]
fn kanban_conflicts_preserve_draft_and_unknown_saves_preserve_the_original_intent() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    board.selected = Some(task("task_one", "todo"));
    board.fill_editor(
        &mut cx,
        Fields {
            title: "My unsaved edit".into(),
            ..Default::default()
        },
    );
    board.dirty = true;
    let action = BoardAction::Update {
        task: "task_one".into(),
        input: Update {
            idempotency_key: "original-intent".into(),
            expected_version: 1,
            fields: board.fields(&mut cx),
        },
    };
    board.pending = Some(action.clone());
    board.save_request = 12;
    let done = Saved {
        identity: "account".into(),
        project: "p1".into(),
        epoch: 1,
        request: 12,
        result: Err("server_unavailable".into()),
    };
    board.apply_saved(&mut cx, &done);
    assert_eq!(
        serde_json::to_value(&board.pending).unwrap(),
        serde_json::to_value(Some(&action)).unwrap()
    );
    assert!(board.dirty);
    let conflict = Saved {
        result: Err("board_version_conflict".into()),
        ..done
    };
    board.apply_saved(&mut cx, &conflict);
    assert!(board.pending.is_none());
    assert!(board.conflict);
    assert_eq!(
        board.view.text_input(&cx, ids!(title)).text(),
        "My unsaved edit"
    );
}
#[test]
fn kanban_large_lanes_virtualize_cards_and_resolve_real_drop_neighbours() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    for (index, lane) in board
        .snapshot
        .as_mut()
        .unwrap()
        .lanes
        .iter_mut()
        .enumerate()
    {
        lane.tasks = (0..200)
            .map(|i| {
                let mut value = task(&format!("task_{index}_{i}"), STATES[index]);
                value.rank = (i + 1) * 1024;
                value.number = i + 1;
                value
            })
            .collect();
        lane.total = 200;
    }
    draw(&mut cx, &mut board, 1500.0, 760.0);
    assert!(
        board.card_targets.len() < 70,
        "only visible cards should exist, got {}",
        board.card_targets.len()
    );
    assert!(!board.card_areas.contains_key("task_0_199"));
    let second = board.card_areas["task_0_1"].rect(&cx);
    let neighbours = board
        .drop_neighbours(
            &mut cx,
            0,
            dvec2(second.pos.x + 20.0, second.pos.y + 5.0),
            "task_1_0",
        )
        .unwrap();
    assert_eq!(
        neighbours,
        (Some("task_0_1".into()), Some("task_0_0".into()))
    );
}

#[test]
fn kanban_project_switch_retains_only_original_unconfirmed_intents_and_resets_view_state() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    board.active = true;
    board.offline = true;
    board.failed = true;
    board.retry_delay = 60.0;
    board.pending = Some(BoardAction::Create {
        input: Create {
            idempotency_key: "unconfirmed".into(),
            fields: Fields {
                title: "Keep original".into(),
                ..Default::default()
            },
            state: "todo".into(),
        },
    });
    board.bind(&mut cx, "account", Some("p2"), true);
    assert!(board.active);
    assert!(board.snapshot.is_none());
    assert!(!board.offline && !board.failed);
    assert_eq!(board.retry_delay, 5.0);
    assert!(board.pending.is_none());
    assert!(board.retained.contains_key("p1"));
    board.bind(&mut cx, "account", Some("p1"), true);
    assert!(
        matches!(board.pending, Some(BoardAction::Create { ref input }) if input.idempotency_key == "unconfirmed")
    );
    board.bind(&mut cx, "different-account", Some("p1"), true);
    assert!(board.pending.is_none() && board.retained.is_empty());
}

#[test]
fn kanban_read_only_preview_still_scrolls_the_actual_native_viewports() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    let lane = &mut board.snapshot.as_mut().unwrap().lanes[0];
    lane.tasks = (0..30)
        .map(|i| task(&format!("task_{i}"), "todo"))
        .collect();
    lane.total = 30;
    board.active = false;
    draw(&mut cx, &mut board, 700.0, 760.0);
    let rect = board.view.view(&cx, ids!(todo)).area().rect(&cx);
    let list = board
        .view
        .view(&cx, ids!(todo))
        .portal_list(&cx, ids!(cards));
    let before = (
        list.borrow().unwrap().first_id(),
        list.borrow().unwrap().first_scroll(),
    );
    let scroll = |delta| {
        Event::Scroll(ScrollEvent {
            window_id: WindowId(0, 0),
            scroll: delta,
            abs: rect.pos + dvec2(100.0, 150.0),
            modifiers: KeyModifiers::default(),
            handled_x: std::cell::Cell::new(false),
            handled_y: std::cell::Cell::new(false),
            is_mouse: false,
            time: 1.0,
            phase: ScrollPhase::Changed,
        })
    };
    board.handle_event(&mut cx, &scroll(dvec2(0.0, 350.0)), &mut Scope::empty());
    draw(&mut cx, &mut board, 700.0, 760.0);
    let after = (
        list.borrow().unwrap().first_id(),
        list.borrow().unwrap().first_scroll(),
    );
    assert_ne!(
        before, after,
        "read-only means no mutations, not a frozen event tree"
    );
    board.handle_event(&mut cx, &scroll(dvec2(300.0, 0.0)), &mut Scope::empty());
    draw(&mut cx, &mut board, 700.0, 760.0);
    let after = board.view.view(&cx, ids!(todo)).area().rect(&cx);
    assert!(
        after.pos.x < rect.pos.x - 100.0,
        "horizontal viewport must scroll: {rect:?} -> {after:?}"
    );
}

#[test]
fn kanban_mouse_drag_can_start_from_the_card_body() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    draw(&mut cx, &mut board, 1100.0, 760.0);
    let card = board
        .view
        .view(&cx, ids!(todo))
        .portal_list(&cx, ids!(cards))
        .item(&mut cx, 0, id!(Card));
    let rect = card.area().rect(&cx);
    let point = rect.pos + dvec2(40.0, rect.size.y * 0.5);
    let actions = cx.capture_actions(|cx| {
        card.handle_event(
            cx,
            &Event::MouseDown(MouseDownEvent {
                abs: point,
                button: MouseButton::PRIMARY,
                window_id: WindowId(0, 0),
                modifiers: KeyModifiers::default(),
                handled: std::cell::Cell::new(Area::Empty),
                time: 1.0,
            }),
            &mut Scope::empty(),
        )
    });
    assert!(
        actions
            .iter()
            .filter_map(|a| a.as_widget_action())
            .any(|a| matches!(a.cast::<CardAction>(), CardAction::Start(_))),
        "the card body must capture mouse dragging, not only the tiny handle"
    );
}

#[test]
fn kanban_empty_filtered_lane_resets_an_old_scroll_anchor() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    let lane = &mut board.snapshot.as_mut().unwrap().lanes[0];
    lane.tasks = (0..30)
        .map(|i| task(&format!("task_{i}"), "todo"))
        .collect();
    lane.total = 30;
    draw(&mut cx, &mut board, 1100.0, 760.0);
    let list = board
        .view
        .view(&cx, ids!(todo))
        .portal_list(&cx, ids!(cards));
    list.set_first_id_and_scroll(25, -40.0);
    let lane = &mut board.snapshot.as_mut().unwrap().lanes[0];
    lane.tasks.clear();
    lane.total = 0;
    draw(&mut cx, &mut board, 1100.0, 760.0);
    assert_eq!(list.first_id(), 0);
    let empty = list.item(&mut cx, 0, id!(Empty));
    assert!(
        empty.area().rect(&cx).size.y > 50.0,
        "an empty filtered lane must show its empty state rather than an old scroll offset"
    );
}

#[test]
fn kanban_older_history_pages_cannot_replace_the_latest_page() {
    let (mut cx, mut board) = panel();
    populate(&mut board);
    board.selected = Some(task("task_one", "todo"));
    board.detail_request = 7;
    board.comments_request = 12;
    board.reads.extend([11, 12]);
    let old = Completed {
        identity: "account".into(),
        project: "p1".into(),
        epoch: 1,
        request: 11,
        kind: Read::Comments("task_one".into(), true),
        detail_generation: 7,
        result: Ok(
            json!({"items":[{"author":"@alice:test","body":"old page","createdAt":1}],"next":1,"hasMore":false}),
        ),
    };
    board.apply_read(&mut cx, &old);
    assert!(board.comments.is_empty());
    let current = Completed {
        request: 12,
        result: Ok(
            json!({"items":[{"author":"@bob:test","body":"current page","createdAt":2}],"next":2,"hasMore":false}),
        ),
        ..old
    };
    board.apply_read(&mut cx, &current);
    assert_eq!(board.comments[0]["body"], "current page");
}
