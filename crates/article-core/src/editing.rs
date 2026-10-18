//! Transport-independent source binding and document history.
use serde::{Serialize, Deserialize};
use crate::document::{Document, MAX_BODY};
const MAX_MARKDOWN: usize = MAX_BODY;
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorState {
    pub title: String,
    pub markdown: String,
}
impl EditorState {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.chars().count() > 120
            || self.title.chars().any(char::is_control)
            || self.markdown.len() > MAX_MARKDOWN
        {
            return Err("Use a title up to 120 characters and Markdown up to 24 KB."
            .into());
        }
        Ok(())
    }
}
/// Document-level history shared by embedded and standalone editors.
#[derive(Default)]
pub struct EditHistory {
    undo: Vec<Document>,
    redo: Vec<Document>,
}
impl EditHistory {
    pub fn checkpoint(&mut self, document: &Document) {
        self.undo.push(document.clone());
        if self.undo.len() > 100 { self.undo.remove(0); }
        self.redo.clear();
    }
    pub fn undo(&mut self, document: &mut Document) -> bool {
        let Some(previous) = self.undo.pop() else { return false; };
        self.redo.push(std::mem::replace(document, previous)); true
    }
    pub fn redo(&mut self, document: &mut Document) -> bool {
        let Some(next) = self.redo.pop() else { return false; };
        self.undo.push(std::mem::replace(document, next)); true
    }
    pub fn clear(&mut self) { self.undo.clear(); self.redo.clear(); }
}
