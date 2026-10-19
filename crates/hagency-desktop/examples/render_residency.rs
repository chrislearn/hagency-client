//! Offline Windows regression for D3D11 instance-buffer retirement.
//! Run with --poll to service completion receipts; no account or network is used.
pub use makepad_widgets;
use makepad_widgets::*;
use rinx::render_maintenance::RenderMaintenance;

app_main!(App);

script_mod! {
    use mod.prelude.widgets.*
    mod.widgets.ResidencyLoad = set_type_default() do #(ResidencyLoad::register_widget(vm)) {
        width: Fill height: 160
        draw_bg.color: #308080
    }
    startup() do #(App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.title: "Hagency render residency check"
                window.inner_size: vec2(700, 350)
                body +: {
                    flow: Down padding: 20 spacing: 16
                    show_bg: true draw_bg.color: #203040
                    Label {text: "This panel must remain visible during repeated redraws."}
                    counter := Label {width: Fill text: "Starting"}
                    mod.widgets.ResidencyLoad {}
                    TextInput {width: Fill text: "Unsaved draft stays here"}
                }
            }
        }
    }
}

/// Independent sublists prevent shader batching from turning a complex scene
/// into one allocation. This exercises more than 64 replaced buffers per frame.
#[derive(Script, ScriptHook, Widget)]
struct ResidencyLoad {
    #[source]
    source: ScriptObjectRef,
    #[uid]
    uid: WidgetUid,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[redraw]
    #[live]
    draw_bg: DrawColor,
    #[rust]
    lists: Vec<DrawList>,
}

impl Widget for ResidencyLoad {
    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {}
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        if self.lists.is_empty() {
            self.lists = (0..256).map(|_| DrawList::new(cx)).collect();
        }
        let rect = cx.walk_turtle(walk);
        for (index, list) in self.lists.iter_mut().enumerate() {
            list.begin_always(cx);
            self.draw_bg.draw_abs(
                cx,
                Rect {
                    pos: rect.pos
                        + dvec2(
                            (index % 16) as f64 * rect.size.x / 16.0,
                            (index / 16) as f64 * 10.0,
                        ),
                    size: dvec2(rect.size.x / 16.0 - 2.0, 8.0),
                },
            );
            list.end(cx);
        }
        DrawStep::done()
    }
}

#[derive(Script, ScriptHook)]
struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    timer: Timer,
    #[rust]
    ticks: u64,
    #[rust]
    maintenance: RenderMaintenance,
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        makepad_widgets::script_mod(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if std::env::args().any(|arg| arg == "--poll") {
            self.maintenance.handle_event(cx, event);
        } else if std::env::args().any(|arg| arg == "--poll-only")
            && matches!(event, Event::Draw(_))
        {
            // The previous incomplete fix: receipts move, collector stays at 64.
            cx.frame_completion_serial();
        }
        if matches!(event, Event::Startup) {
            self.timer = cx.start_interval(0.016);
        }
        if self.timer.is_event(event).is_some() {
            self.ticks += 1;
            // Small envelope accelerates the same accounting exhaustion that
            // otherwise needs a long-running interactive session.
            cx.draw_lists
                .1
                .allocations
                .set_device_limit(4 * 1024 * 1024);
            self.ui.label(cx, ids!(counter)).set_text(
                cx,
                &format!(
                    "Frame {}: {}",
                    self.ticks,
                    "Rendering must remain complete. ".repeat(3)
                ),
            );
            self.ui.redraw(cx);
            if self.ticks % 60 == 0 {
                let budget = &cx.draw_lists.1.allocations;
                log!(
                    "residency tick={} submitted={} completed={} bytes={} records={} refusals={}",
                    self.ticks,
                    cx.frame_submission_serial(),
                    cx.frame_completed_serial(),
                    budget.bytes(),
                    budget.record_count(),
                    budget.refusals()
                );
            }
        }
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
