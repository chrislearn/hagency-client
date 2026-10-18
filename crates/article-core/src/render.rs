//! Article resource collection over Makepad’s sanitized renderer.
pub use makepad_markdown::render::{Renderer, ImageRequest, EDITORMD_LOGO, valid_link, valid_image, image_link, decode_fragment};
// Rinx explicitly enables the Editor.md compatibility profile.
pub use makepad_markdown::render::editormd_emoji_text as emoji_text;

pub fn image_requests(document: &crate::document::Document) -> Vec<String> {
    image_references(document).into_iter().filter(|s|s.starts_with("https://") || s.starts_with("http://")).collect()
}

pub fn image_references(document: &crate::document::Document) -> Vec<String> {
    #[derive(Default)]
    struct Collector(Vec<String>);
    impl Renderer for Collector {
        fn image(&mut self, image: &ImageRequest<'_>) -> String {
            if valid_image(image.url)
                && !self.0.iter().any(|url| url == image.url)
                && self.0.len() < crate::document::MAX_IMAGES
            {
                self.0.push(image.url.into());
            }
            String::new()
        }
        fn text(&mut self, text: &str) -> String {
            emoji_text(text, self)
        }
    }
    let mut collector = Collector::default();
    if document.is_html_source() {
        for block in &document.blocks { document.block_html_with_renderer(block, &mut collector); }
    } else { crate::markdown_render::render(&document.markdown(), &mut collector); }
    collector.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resources_are_collected_from_markup_and_emoji_but_not_code_or_scripts() {
        let source = "![logo](https://example.org/a.png)\n\n<img src='https://example.org/b.svg'>\n\n:editormd-logo: :smiley: 😃\n\n```html\n<img src='https://example.org/code.png'>\n```\n\n<script><img src='https://example.org/script.png'></script>";
        let doc = crate::document::Document::from_markdown("Resources", source).unwrap();
        assert_eq!(
            image_requests(&doc),
            [
                "https://example.org/a.png",
                "https://example.org/b.svg",
                EDITORMD_LOGO
            ]
        );
        assert!(!doc.html().contains("<img"));
        assert_eq!(doc.markdown(), source);
    }
}
