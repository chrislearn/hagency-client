//! Coordinates selection across paragraph widgets, independently of virtualization.
use article_core::{
    document::Document,
    editing::EditHistory,
    selection::{BodySelection, Position},
};
use makepad_widgets::{*, text::selection::Cursor};
use crate::rich_input::{ArticleRichInputRef, ArticleRichInputWidgetRefExt};
use unicode_segmentation::UnicodeSegmentation;
use makepad_widgets::makepad_platform::event::finger::TouchState;

#[derive(Default)]
pub struct ArticleSelection {
    pub selection: Option<BodySelection>,
    drag: Option<(Position, DVec2)>,
    pointer: Option<DVec2>,
    scroll_frame: Option<NextFrame>,
    pending_focus: Option<Position>,
    pending_input: Option<TextInputEvent>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionUpdate {
    Pass,
    Handled,
    Changed,
}

impl ArticleSelection {
    fn inputs(
        cx: &Cx,
        list: &PortalListRef,
        document: &Document,
    ) -> Vec<(usize, ArticleRichInputRef)> {
        (0..document.blocks.len())
            .filter_map(|index| {
                let (_, row) = list.get_item(index)?;
                let source_editor = row.view(cx, ids!(source_editor));
                if !source_editor.is_empty() && !source_editor.visible() { return None; }
                let input = row.article_rich_input(cx, ids!(rich));
                (!input.is_empty()).then_some((index, input))
            })
            .collect()
    }

    fn focused(
        cx: &Cx,
        list: &PortalListRef,
        document: &Document,
    ) -> Option<(usize, ArticleRichInputRef)> {
        Self::inputs(cx, list, document)
            .into_iter()
            .find(|(_, input)| input.has_focus(cx))
    }

    fn hit(
        cx: &Cx,
        list: &PortalListRef,
        document: &Document,
        abs: DVec2,
        nearest: bool,
    ) -> Option<Position> {
        let viewport = list.area().rect(cx);
        // A partially clipped paragraph still has its full layout rectangle.
        // Toolbar clicks must not hit that hidden portion and clear selection.
        // Drag tracking may leave the viewport so it can continue auto-scrolling.
        if !nearest && !viewport.contains(abs) {
            return None;
        }
        Self::inputs(cx, list, document)
            .into_iter()
            .filter_map(|(index, input)| {
                let area = input.area();
                if !area.is_valid(cx) {
                    return None;
                }
                let r = area.rect(cx);
                if r.size.y <= 0.0
                    || r.pos.y + r.size.y <= viewport.pos.y
                    || r.pos.y >= viewport.pos.y + viewport.size.y
                {
                    return None;
                }
                if !nearest && !r.contains(abs) {
                    return None;
                }
                let distance = (r.pos.y - abs.y).max(0.0) + (abs.y - r.pos.y - r.size.y).max(0.0);
                let cursor = input.cursor_at(cx, abs)?;
                Some((
                    distance,
                    Position {
                        block: index,
                        byte: cursor.index,
                    },
                ))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, position)| position)
    }

    fn clear_visible(&mut self, cx: &mut Cx, list: &PortalListRef, document: &Document) {
        if self.selection.take().is_some() {
            for (_, input) in Self::inputs(cx, list, document) {
                input.show_body_selection(cx, 0, 0);
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn apply_to_input(
        &self,
        cx: &mut Cx,
        index: usize,
        input: &ArticleRichInputRef,
        document: &Document,
    ) {
        if let Some(selection) = self.selection {
            let len = document.blocks[index].text.len();
            let range = selection.range(index, len).unwrap_or(0..0);
            if selection.anchor <= selection.cursor {
                input.show_body_selection(cx, range.start, range.end);
            } else {
                input.show_body_selection(cx, range.end, range.start);
            }
        } else {
            input.clear_body_selection(cx);
        }
    }

    pub fn after_draw(&mut self, cx: &mut Cx, index: usize, input: &ArticleRichInputRef) {
        if let Some(position) = self.pending_focus.filter(|p| p.block == index) {
            self.pending_focus = None;
            input.set_cursor(
                cx,
                Cursor {
                    index: position.byte,
                    prefer_next_row: false,
                },
                false,
            );
            input.take_key_focus(cx);
            if let Some(event) = self.pending_input.take() {
                input.handle_event(cx, &Event::TextInput(event), &mut Scope::empty());
            }
        }
    }

    fn sync(&self, cx: &mut Cx, list: &PortalListRef, document: &Document) {
        for (index, input) in Self::inputs(cx, list, document) {
            self.apply_to_input(cx, index, &input, document);
        }
        list.redraw(cx);
    }

    fn replace(
        &mut self,
        cx: &mut Cx,
        list: &PortalListRef,
        document: &mut Document,
        history: &mut EditHistory,
        text: &str,
    ) {
        let Some(selection) = self.selection else {
            return;
        };
        history.checkpoint(document);
        self.clear_visible(cx, list, document);
        if let Some(position) = selection.replace(document, text) {
            self.pending_focus = Some(position);
            list.set_first_id_and_scroll(position.block, 0.0);
        }
        self.drag = None;
        list.redraw(cx);
    }

    fn move_pointer(
        &mut self,
        cx: &mut Cx,
        list: &PortalListRef,
        document: &Document,
        abs: DVec2,
    ) -> bool {
        let Some((anchor, down)) = self.drag else {
            return false;
        };
        if (abs - down).length() < 3.0 && self.selection.is_none() {
            return false;
        }
        let Some(cursor) = Self::hit(cx, list, document, abs, true) else {
            return false;
        };
        let viewport = list.area().rect(cx);
        if anchor.block != cursor.block
            || self.selection.is_some()
            || abs.y < viewport.pos.y
            || abs.y > viewport.pos.y + viewport.size.y
        {
            self.selection = Some(BodySelection { anchor, cursor });
            self.sync(cx, list, document);
            self.pointer = Some(abs);
            let r = list.area().rect(cx);
            if abs.y < r.pos.y || abs.y > r.pos.y + r.size.y {
                let step = if abs.y < r.pos.y { 12.0 } else { -12.0 };
                if let Some(mut inner) = list.borrow_mut() {
                    let first = inner.first_id();
                    let scroll = inner.first_scroll();
                    inner.set_first_id_and_scroll(first, scroll + step);
                }
                self.scroll_frame = Some(cx.new_next_frame());
                list.redraw(cx);
            }
            true
        } else {
            false
        }
    }

    /// Call before dispatching to children. Pass events still go to the inputs,
    /// including pointer-up so native capture is always released normally.
    pub fn handle_event(
        &mut self,
        cx: &mut Cx,
        event: &Event,
        list: &PortalListRef,
        document: &mut Document,
        history: &mut EditHistory,
    ) -> SelectionUpdate {
        use SelectionUpdate::*;
        if self
            .selection
            .is_some_and(|selection| selection.ordered().1.block >= document.blocks.len())
        {
            self.clear_visible(cx, list, document);
        }
        self.selection = self.selection.and_then(|selection| selection.normalized(document));
        match event {
            Event::TouchUpdate(update) => {
                if update.touches.iter().any(|touch| touch.state == TouchState::Start
                    && Self::hit(cx, list, document, touch.abs, false).is_some()) {
                    self.clear_visible(cx, list, document);
                    self.drag = None;
                    self.pointer = None;
                    self.scroll_frame = None;
                }
            }
            Event::MouseDown(e) if e.button.is_primary() => {
                self.drag = None;
                if let Some(position) = Self::hit(cx, list, document, e.abs, false) {
                    let anchor = if e.modifiers.shift {
                        self.selection
                            .map(|s| s.anchor)
                            .or_else(|| {
                                Self::focused(cx, list, document).map(|(block, input)| Position {
                                    block,
                                    byte: input.selection().anchor.index,
                                })
                            })
                            .unwrap_or(position)
                    } else {
                        position
                    };
                    self.clear_visible(cx, list, document);
                    self.drag = Some((anchor, e.abs));
                    if e.modifiers.shift && anchor.block != position.block {
                        self.selection = Some(BodySelection {
                            anchor,
                            cursor: position,
                        });
                    }
                }
            }
            Event::MouseMove(e) => {
                if self.move_pointer(cx, list, document, e.abs) {
                    return Handled;
                }
            }
            Event::MouseUp(_) => {
                self.drag = None;
                self.pointer = None;
                self.scroll_frame = None;
            }
            Event::NextFrame(_) => {
                if self
                    .scroll_frame
                    .is_some_and(|frame| frame.is_event(event).is_some())
                {
                    self.scroll_frame = None;
                    if let Some(abs) = self.pointer {
                        self.move_pointer(cx, list, document, abs);
                    }
                }
            }
            Event::KeyDown(key) if Self::focused(cx, list, document).is_some() => {
                if key.key_code == KeyCode::KeyA && key.modifiers.is_primary() {
                    self.selection = BodySelection::all(document);
                    self.sync(cx, list, document);
                    return Handled;
                }
                if key.modifiers.shift
                    && matches!(key.key_code, KeyCode::ArrowLeft | KeyCode::ArrowRight)
                {
                    let (block, input) = Self::focused(cx, list, document).unwrap();
                    let local = input.selection();
                    let current = self.selection.unwrap_or(BodySelection {
                        anchor: Position {
                            block,
                            byte: local.anchor.index,
                        },
                        cursor: Position {
                            block,
                            byte: local.cursor.index,
                        },
                    });
                    let mut position = current.cursor;
                    let text = &document.blocks[position.block].text;
                    let forward = key.key_code == KeyCode::ArrowRight;
                    if self.selection.is_some()
                        || (!forward && position.byte == 0)
                        || (forward && position.byte == text.len())
                    {
                        if forward {
                            if position.byte < text.len() {
                                position.byte += text[position.byte..]
                                    .graphemes(true)
                                    .next()
                                    .map_or(0, str::len);
                            } else if position.block + 1 < document.blocks.len() {
                                position.block += 1;
                                position.byte = 0;
                            }
                        } else if position.byte > 0 {
                            position.byte = text[..position.byte]
                                .grapheme_indices(true)
                                .last()
                                .map_or(0, |(i, _)| i);
                        } else if position.block > 0 {
                            position.block -= 1;
                            position.byte = document.blocks[position.block].text.len();
                        }
                        self.selection = Some(BodySelection {
                            cursor: position,
                            ..current
                        });
                        self.sync(cx, list, document);
                        if current.anchor == position {
                            self.selection = None;
                        }
                        return Handled;
                    }
                }
                if key.key_code == KeyCode::KeyZ && key.modifiers.is_primary() {
                    let changed = if key.modifiers.shift {
                        history.redo(document)
                    } else {
                        history.undo(document)
                    };
                    if changed {
                        self.clear_visible(cx, list, document);
                        self.pending_focus = Some(Position::default());
                        list.set_first_id_and_scroll(0, 0.0);
                        list.redraw(cx);
                        return Changed;
                    }
                    return Handled;
                }
                if let Some(selection) = self.selection {
                    match key.key_code {
                        KeyCode::ReturnKey | KeyCode::NumpadEnter
                            if !key.modifiers.is_primary() =>
                        {
                            self.replace(cx, list, document, history, "\n");
                            return Changed;
                        }
                        KeyCode::Backspace | KeyCode::Delete => {
                            self.replace(cx, list, document, history, "");
                            return Changed;
                        }
                        KeyCode::Escape => {
                            self.clear_visible(cx, list, document);
                            return Handled;
                        }
                        KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::ArrowUp
                        | KeyCode::ArrowDown
                            if !key.modifiers.shift =>
                        {
                            let (start, end) = selection.ordered();
                            let position =
                                if matches!(key.key_code, KeyCode::ArrowLeft | KeyCode::ArrowUp) {
                                    start
                                } else {
                                    end
                                };
                            self.clear_visible(cx, list, document);
                            self.pending_focus = Some(position);
                            list.set_first_id_and_scroll(position.block, 0.0);
                            list.redraw(cx);
                            return Handled;
                        }
                        _ => {}
                    }
                }
            }
            Event::TextCopy(copy)
                if self.selection.is_some() && Self::focused(cx, list, document).is_some() =>
            {
                *copy.response.borrow_mut() = Some(self.selection.unwrap().text(document));
                return Handled;
            }
            Event::TextCut(copy)
                if self.selection.is_some() && Self::focused(cx, list, document).is_some() =>
            {
                *copy.response.borrow_mut() = Some(self.selection.unwrap().text(document));
                self.replace(cx, list, document, history, "");
                return Changed;
            }
            Event::TextInput(input)
                if self.selection.is_some() && Self::focused(cx, list, document).is_some() =>
            {
                let mut input = input.clone();
                if let Some(state) = input.full_state_sync.take() {
                    let old = Self::focused(cx, list, document)
                        .and_then(|(_, i)| i.content())
                        .map(|c| c.0)
                        .unwrap_or_default();
                    if old == state.text {
                        return Pass;
                    }
                    let prefix: usize = old
                        .chars()
                        .zip(state.text.chars())
                        .take_while(|(a, b)| a == b)
                        .map(|(c, _)| c.len_utf8())
                        .sum();
                    let suffix: usize = old[prefix..]
                        .chars()
                        .rev()
                        .zip(state.text[prefix..].chars().rev())
                        .take_while(|(a, b)| a == b)
                        .map(|(c, _)| c.len_utf8())
                        .sum();
                    input.input = state.text[prefix..state.text.len() - suffix].to_owned();
                }
                input.replace_range = None;
                input.replace_last = false;
                if input.composition.is_some() {
                    self.replace(cx, list, document, history, "");
                    self.pending_input = Some(input);
                } else {
                    self.replace(cx, list, document, history, &input.input);
                }
                return Changed;
            }
            _ => {}
        }
        Pass
    }

    pub fn after_event(&self, cx: &mut Cx, list: &PortalListRef, document: &Document) {
        if self.selection.is_some() {
            self.sync(cx, list, document);
        }
    }
}
