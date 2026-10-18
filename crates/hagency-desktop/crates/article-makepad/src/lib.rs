//! Native article components. The embedding application supplies account,
//! persistence, image selection, navigation and publication services.
pub use article_core::document;
pub mod rich_input;
pub mod body_selection;
pub use makepad_markdown_widgets::{math, content, content_view, emoji, diagram, diagram_view};
pub mod math_view;
mod rich_layout;

mod aliases {
use makepad_widgets::*;
script_mod! {
    use mod.prelude.widgets_internal.*
    mod.widgets.ArticleMath = mod.widgets.MarkdownMath
    mod.widgets.ArticleDiagram = mod.widgets.MarkdownDiagram
    mod.widgets.ArticleImage = mod.widgets.MarkdownImage
    mod.widgets.ArticleEmoji = mod.widgets.MarkdownEmoji
    mod.widgets.ArticleCode = mod.widgets.MarkdownCode
    mod.widgets.ArticleCell = mod.widgets.MarkdownCell
}
}

pub fn script_mod(vm: &mut makepad_widgets::ScriptVm) {
    rich_input::script_mod(vm);
    makepad_markdown_widgets::script_mod(vm);
    aliases::script_mod(vm);
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod apple_fonts;
pub mod presentation;
