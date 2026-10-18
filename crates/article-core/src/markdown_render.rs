//! Article block mapping over Makepad’s whole-document Markdown renderer.
pub use makepad_markdown::markdown_render::*;
use crate::render::Renderer;

/// One rendered fragment per editable document block. Parsing each block in
/// isolation would break TOCs, reference links, footnotes and heading anchors.
pub fn render_editor(document: &crate::document::Document, renderer: &mut dyn Renderer) -> Vec<RenderedBlock> {
    let (source, starts) = document.markdown_with_block_lines();
    let mut blocks = vec![RenderedBlock::default(); starts.len()];
    for rendered in render(&source, renderer) {
        let index = starts.partition_point(|line| *line <= rendered.source_line).saturating_sub(1);
        if let Some(block) = blocks.get_mut(index) {
            block.html.push_str(&rendered.html);
            if !block.text.is_empty() { block.text.push('\n'); }
            block.text.push_str(&rendered.text);
            block.anchors.extend(rendered.anchors);
            block.source_line = starts[index];
        }
    }
    blocks
}
