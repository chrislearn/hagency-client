//! Sidebar preferences and a shared, captured column resize gesture.
use makepad_widgets::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SidebarKind { #[default] Navigation, Context, Information }

impl SidebarKind {
    pub(crate) fn bounds(self) -> (f64, f64) {
        match self {Self::Navigation => (160.0, 320.0), Self::Context => (180.0, 420.0), Self::Information => (240.0, 480.0)}
    }
    fn valid(self, width: Option<f64>) -> Option<f64> {
        let (min, max) = self.bounds();
        width.filter(|w| w.is_finite() && (min..=max).contains(w))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SidebarWidths {
    #[serde(default, deserialize_with = "navigation_width")]
    pub navigation: Option<f64>,
    #[serde(default, deserialize_with = "context_width")]
    pub context: Option<f64>,
    #[serde(default, deserialize_with = "information_width")]
    pub information: Option<f64>,
}

impl SidebarWidths {
    pub(crate) fn set(&mut self, kind: SidebarKind, width: Option<f64>) {
        let slot = match kind {SidebarKind::Navigation => &mut self.navigation, SidebarKind::Context => &mut self.context, SidebarKind::Information => &mut self.information};
        *slot = kind.valid(width);
    }
}
fn read_width<'de, D: serde::Deserializer<'de>>(d: D, kind: SidebarKind) -> Result<Option<f64>, D::Error> {
    Ok(kind.valid(crate::utils::deserialize_or_default(d)?))
}
fn navigation_width<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {read_width(d, SidebarKind::Navigation)}
fn context_width<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {read_width(d, SidebarKind::Context)}
fn information_width<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {read_width(d, SidebarKind::Information)}

/// Clamp the display only; a temporary narrow viewport never overwrites preferences.
pub(crate) fn left_widths(available: f64, collapsed: bool, context: bool, preferred: &SidebarWidths) -> (f64, f64, f64, f64) {
    let nav_min = (if collapsed {64.0_f64} else {160.0_f64}).min(available.max(0.0));
    let context_min = if context {180.0} else {0.0};
    let budget = (available - 320.0).max(nav_min + context_min).min(available.max(0.0));
    let nav_max = (budget - context_min).max(nav_min).min(320.0);
    let nav = if collapsed {nav_min} else {SidebarKind::Navigation.valid(preferred.navigation).unwrap_or(if available < 960.0 {160.0} else {200.0}).clamp(nav_min, nav_max)};
    let context_max = (budget - nav).max(context_min).min(420.0);
    let context_width = if context {SidebarKind::Context.valid(preferred.context).unwrap_or(if available < 960.0 {180.0} else {240.0}).clamp(context_min, context_max)} else {0.0};
    (nav, context_width, nav_max, context_max)
}

#[derive(Clone, Debug)]
pub(crate) struct SidebarWidthSaved(pub SidebarKind, pub Option<f64>);

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    mod.widgets.SidebarResizeHandle = #(SidebarResizeHandle::register_widget(vm)) {
        width: 6 height: Fill cursor: MouseCursor.ColResize show_bg: true
        draw_bg +: {
            hover: instance(0.0)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.rect(self.rect_size.x * 0.5 - 0.5, 0.0, 1.0, self.rect_size.y)
                sdf.fill(#0000.mix(mod.widgets.RINX_ACCENT, self.hover))
                return sdf.result
            }
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct SidebarResizeHandle {
    #[deref] view: View,
    #[rust] kind: SidebarKind,
    #[rust] current: f64,
    #[rust] min: f64,
    #[rust] max: f64,
    #[rust] reverse: bool,
    #[rust] drag: Option<(f64, f64)>,
    #[rust] hovered: bool,
}

impl Widget for SidebarResizeHandle {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if !self.view.visible {self.drag = None; return;}
        if let Event::KeyDown(key) = event && key.key_code == KeyCode::Escape && self.drag.take().is_some() {
            cx.redraw_all();
        }
        match event.hits(cx, self.view.area()) {
            Hit::FingerHoverIn(_) | Hit::FingerHoverOver(_) => {
                self.hovered = true;
                script_apply_eval!(cx, self.view, {draw_bg +: {hover: 1.0}});
                cx.set_cursor(MouseCursor::ColResize);
            }
            Hit::FingerHoverOut(_) if self.drag.is_none() => {self.hovered = false; script_apply_eval!(cx, self.view, {draw_bg +: {hover: 0.0}});}
            Hit::FingerDown(fe) if fe.is_primary_hit() => {
                if fe.tap_count == 2 {self.drag = None; cx.action(SidebarWidthSaved(self.kind, None));}
                else {self.drag = Some((self.current, self.current));}
                cx.set_cursor(MouseCursor::ColResize);
            }
            Hit::FingerMove(fe) => if let Some((start, width)) = self.drag.as_mut() {
                let delta = (fe.abs.x - fe.abs_start.x) * if self.reverse {-1.0} else {1.0};
                *width = (*start + delta).clamp(self.min, self.max);
                cx.set_cursor(MouseCursor::ColResize);
                cx.redraw_all();
            },
            Hit::FingerUp(_) => if let Some((_, width)) = self.drag.take() {
                cx.action(SidebarWidthSaved(self.kind, Some(width.clamp(self.min, self.max))));
                script_apply_eval!(cx, self.view, {draw_bg +: {hover: 0.0}});
            },
            _ => {}
        }
        self.view.handle_event(cx, event, scope);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {self.view.draw_walk(cx, scope, walk)}
}

impl SidebarResizeHandleRef {
    pub(crate) fn preview(&self) -> Option<f64> {self.borrow().and_then(|h| h.drag.map(|(_, width)| width))}
    /// RoomScreen routes overlays separately; keep captured motion/up events
    /// with the separator even after the pointer leaves the information card.
    pub(crate) fn route_pointer_event(&self, cx: &mut Cx, event: &Event, scope: &mut Scope) -> bool {
        let own = self.borrow().is_some_and(|h| {
            if !h.view.visible {return false;}
            let point = match event {Event::MouseDown(e) => Some(e.abs), Event::MouseMove(e) => Some(e.abs), Event::MouseUp(e) => Some(e.abs), _ => None};
            point.is_some_and(|point| h.drag.is_some() || (h.hovered && matches!(event, Event::MouseMove(_))) || h.view.area().rect(cx).contains(point))
                || matches!(event, Event::KeyDown(e) if e.key_code == KeyCode::Escape && h.drag.is_some())
        });
        if own {self.handle_event(cx, event, scope);}
        own
    }
    pub(crate) fn configure(&self, cx: &mut Cx, kind: SidebarKind, current: f64, max: f64, reverse: bool, enabled: bool) {
        if let Some(mut h) = self.borrow_mut() {
            h.kind = kind; h.current = current; h.min = kind.bounds().0; h.max = max.max(h.min); h.reverse = reverse;
            h.view.visible = enabled;
            if !enabled && (h.drag.is_some() || h.hovered) {
                h.drag = None; h.hovered = false;
                script_apply_eval!(cx, h.view, {draw_bg +: {hover: 0.0}});
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::app_preferences::AppPreferences;
    use makepad_widgets::makepad_draw::cx_draw::CxDraw;

    #[test]
    fn widths_restore_independently_and_reject_corrupt_values() {
        let prefs = AppPreferences { sidebar_widths: SidebarWidths {navigation: Some(260.0), context: Some(350.0), information: Some(390.0)}, ..Default::default() };
        let encoded = serde_json::to_value(&prefs).unwrap();
        assert_eq!(serde_json::from_value::<AppPreferences>(encoded.clone()).unwrap(), prefs);
        let mut legacy = encoded.clone(); legacy.as_object_mut().unwrap().remove("sidebar_widths");
        assert_eq!(serde_json::from_value::<AppPreferences>(legacy).unwrap().sidebar_widths, SidebarWidths::default());
        for key in ["navigation", "context", "information"] {
            for bad in [serde_json::json!(-1), serde_json::json!(0), serde_json::json!(999999), serde_json::json!("bad"), serde_json::json!({})] {
                let mut corrupt = encoded.clone(); corrupt["sidebar_widths"][key] = bad;
                let restored = serde_json::from_value::<AppPreferences>(corrupt).unwrap();
                let mut expected = prefs.sidebar_widths.clone();
                expected.set(match key {"navigation" => SidebarKind::Navigation, "context" => SidebarKind::Context, _ => SidebarKind::Information}, None);
                assert_eq!(restored.sidebar_widths, expected);
            }
        }
        let mut widths = prefs.sidebar_widths.clone(); widths.set(SidebarKind::Information, Some(f64::NAN));
        assert_eq!(widths.information, None);
        assert_eq!(widths.navigation, prefs.sidebar_widths.navigation);
    }

    #[test]
    fn narrow_windows_keep_chat_space_without_destroying_saved_widths() {
        let preferred = SidebarWidths {navigation: Some(320.0), context: Some(420.0), information: Some(480.0)};
        for width in [760.0, 860.0, 1000.0, 1600.0] {
            let (nav, context, _, _) = left_widths(width, false, true, &preferred);
            assert!(nav >= 160.0 && context >= 180.0);
            assert!(width - nav - context >= 320.0);
        }
        assert_eq!(left_widths(1600.0, false, true, &preferred), (320.0, 420.0, 320.0, 420.0));
        assert_eq!(left_widths(1600.0, true, true, &preferred).0, 64.0);
        assert_eq!(preferred.navigation, Some(320.0));
        assert_eq!(preferred.context, Some(420.0));
    }

    fn handle(kind: SidebarKind, reverse: bool) -> (Cx, SidebarResizeHandle) {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut handle = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm); crate::app::register_widgets(vm);
            vm.bx.captured_errors = Some(Vec::new());
            let value = script_eval!(vm, {mod.widgets.SidebarResizeHandle {}});
            let handle = SidebarResizeHandle::script_from_value(vm, value);
            assert!(vm.take_errors().is_empty()); handle
        });
        handle.kind = kind; handle.reverse = reverse; handle.current = 280.0;
        (handle.min, handle.max) = kind.bounds();
        let pass = DrawPass::new(&mut cx); pass.set_size(&mut cx, dvec2(100.0, 500.0));
        let mut list = DrawList2d::new(&mut cx);
        let event = DrawEvent::default(); let mut draw = CxDraw::new(&mut cx, &event); let mut cx2d = Cx2d::new(&mut draw);
        cx2d.begin_pass(&pass, None); list.begin_always(&mut cx2d);
        cx2d.begin_root_turtle(dvec2(100.0, 500.0), Layout::flow_overlay());
        handle.draw_walk_all(&mut cx2d, &mut Scope::empty(), Walk {width: Size::Fixed(6.0), height: Size::Fixed(500.0), ..Walk::default()});
        cx2d.end_pass_sized_turtle(); list.end(&mut cx2d); cx2d.end_pass(&pass); drop(cx2d); drop(draw);
        // Match the platform's mouse-down bookkeeping before dispatching events.
        cx.fingers.first_mouse_button = Some((MouseButton::PRIMARY, WindowId(0,0)));
        (cx, handle)
    }
    fn down() -> Event {Event::MouseDown(MouseDownEvent {abs: dvec2(3.0, 200.0), button: MouseButton::PRIMARY, window_id: WindowId(0,0), modifiers: KeyModifiers::default(), handled: std::cell::Cell::new(Area::Empty), time: 1.0})}
    fn movement(x: f64) -> Event {Event::MouseMove(MouseMoveEvent {abs: dvec2(x, 200.0), lock_delta: DVec2::default(), window_id: WindowId(0,0), modifiers: KeyModifiers::default(), handled: std::cell::Cell::new(Area::Empty), time: 1.1})}
    #[test]
    fn captured_drag_saves_once_on_release_and_reverses_for_the_right_edge() {
        for (kind, reverse, expected) in [(SidebarKind::Navigation, false, 320.0), (SidebarKind::Information, true, 240.0)] {
            let (mut cx, mut h) = handle(kind, reverse);
            h.handle_event(&mut cx, &down(), &mut Scope::empty());
            assert!(h.drag.is_some());
            let actions = cx.capture_actions(|cx| h.handle_event(cx, &movement(1003.0), &mut Scope::empty()));
            assert!(!actions.iter().any(|a| a.downcast_ref::<SidebarWidthSaved>().is_some()), "moving must not persist every frame");
            assert_eq!(h.drag.unwrap().1, expected, "capture continues outside the six-pixel handle");
            let up = Event::MouseUp(MouseUpEvent {abs: dvec2(1003.0, 200.0), button: MouseButton::PRIMARY, window_id: WindowId(0,0), modifiers: KeyModifiers::default(), time: 1.2});
            let actions = cx.capture_actions(|cx| h.handle_event(cx, &up, &mut Scope::empty()));
            let saved: Vec<_> = actions.iter().filter_map(|a| a.downcast_ref::<SidebarWidthSaved>()).collect();
            assert_eq!(saved.len(), 1); assert_eq!(saved[0].0, kind); assert_eq!(saved[0].1, Some(expected));
            assert!(h.drag.is_none());
        }
    }
    #[test]
    fn escape_cancels_without_saving() {
        let (mut cx, mut h) = handle(SidebarKind::Context, false);
        h.handle_event(&mut cx, &down(), &mut Scope::empty());
        h.handle_event(&mut cx, &movement(80.0), &mut Scope::empty());
        let escape = Event::KeyDown(KeyEvent {key_code: KeyCode::Escape, is_repeat: false, modifiers: KeyModifiers::default(), time: 1.2});
        let actions = cx.capture_actions(|cx| h.handle_event(cx, &escape, &mut Scope::empty()));
        assert!(h.drag.is_none());
        assert!(!actions.iter().any(|a| a.downcast_ref::<SidebarWidthSaved>().is_some()));
    }
}
