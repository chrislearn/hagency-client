//! Lossless source blocks for Markdown/HTML outside the rich text model.
//! Parsing never grants resource loading or active content to a renderer.
use std::ops::Range;
use pulldown_cmark::{Event, Options, Parser};
use crate::document::{Block, BlockKind, Document, MAX_BODY};

fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS | Options::ENABLE_MATH
        | Options::ENABLE_FOOTNOTES | Options::ENABLE_GFM | Options::ENABLE_DEFINITION_LIST
}

/// Keep simple paragraphs as rich text. Preserve each extended top-level
/// structure as its own editable source block, including its original syntax.
pub fn import_markdown(title: &str, source: &str) -> Result<Document, String> {
    if source.len() > MAX_BODY { return Err("Article exceeds its size limits.".into()); }
    let parser = Parser::new_ext(source, options());
    let mut definitions: Vec<Range<usize>> = parser.reference_definitions().iter()
        .map(|(_, definition)| definition.span.clone()).collect();
    definitions.sort_by_key(|r| r.start);
    let references = definitions.iter().map(|r| &source[r.clone()]).collect::<Vec<_>>().join("\n");
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    start = range.start;
                    if matches!(tag, pulldown_cmark::Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Indented)) {
                        start = source[..start].rfind('\n').map_or(0, |n| n + 1);
                    }
                }
                depth += 1;
            }
            Event::End(_) => {
                depth -= 1;
                if depth == 0 { ranges.push(start..range.end); }
            }
            _ if depth == 0 => ranges.push(range),
            _ => (),
        }
    }
    let mut document = Document { title: title.into(), blocks: Vec::new(), reference_definitions: references, ..Document::default() };
    for range in ranges {
        let text = &source[range];
        // References must resolve against the whole document in previews.
        let resolved = format!("{text}\n\n{}", document.reference_definitions);
        let rich = if crate::markdown::inspect(&resolved).is_empty() {
            Document::from_visual_markdown(title, &resolved).ok().filter(|candidate| {
                // An unrelated visual edit must not change hard breaks, list
                // tightness/numbering, autolinks, or other Markdown semantics.
                use crate::markdown_render::{specification_html, Dialect};
                specification_html(&resolved,Dialect::Article)==specification_html(&candidate.markdown(),Dialect::Article)
            })
        } else { None };
        if let Some(rich) = rich {
            document.blocks.extend(rich.blocks);
        } else {
            document.blocks.push(Block::new(BlockKind::Markdown, text));
        }
    }
    if document.blocks.is_empty() {
        document.blocks.push(Block::new(BlockKind::Paragraph, ""));
    }
    document.retain_source(source);
    document.validate()?;
    Ok(document)
}

pub fn markdown_html(source: &str) -> String {
    crate::math::markdown_html(source, crate::math::fallback)
}

pub use makepad_markdown::markup::{html_fragment, html_fragment_with_renderer};

/// File names are supplied by a host picker; this API has no filesystem access.
pub fn import_file(name: &str, bytes: &[u8]) -> Result<Document, String> {
    if bytes.len() > MAX_BODY { return Err("Article exceeds its size limits.".into()); }
    let text = std::str::from_utf8(bytes).map_err(|_| "Choose a UTF-8 Markdown or HTML file.")?.trim_start_matches('\u{feff}');
    let path = std::path::Path::new(name);
    let title: String = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Imported article")
        .chars().filter(|c| !c.is_control()).take(120).collect();
    match path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "html" | "htm" => Document::from_html(&title, text),
        "md" | "markdown" | "txt" => Document::from_markdown(&title, text),
        _ => Err("Choose a Markdown or HTML file.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indented_code_keeps_first_line_indentation_and_literal_html() {
        let source = "Code:\n\n    <?php\n        echo \"hello\";\n    ?>\n\n    | A | B |\n";
        let doc = Document::from_markdown("Code", source).unwrap();
        assert!(doc.html().contains("<pre><code>&lt;?php"));
        assert!(doc.html().contains("        echo") || doc.html().contains("    echo"));
        assert_eq!(doc.markdown(), source);
    }

    #[test]
    fn extended_markdown_roundtrips_and_renders_structures() {
        let source = "---\ntitle: source\n---\n\n# 标题\n\n正文 **粗体** `代码` ~~删除~~\n\n- [x] 完成\n  - 嵌套\n\n| 项目 | 值 |\n| --- | ---: |\n| 中文 | 3 |\n\n```html\n<script>example()</script>\n```\n\n<a href=\"https://example.org\">链接</a>\n\n![图](https://example.org/image.png)\n";
        let doc = Document::from_markdown("Test", source).unwrap();
        assert_eq!(doc.markdown(), source);
        let rendered = doc.html();
        for tag in ["<h1>标题", "<code>代码", "<del>删除", "☑", "<table>", "<ul>", "&lt;script&gt;", "href=\"https://example.org\""] { assert!(rendered.contains(tag), "{tag}: {rendered}"); }
        assert!(!rendered.contains("<input"));
        assert!(!rendered.contains("<script"));
        assert!(!rendered.contains("<img"));
        assert!(!rendered.contains("title: source"));
        let restored: Document = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(restored.markdown(), source);
    }

    #[test]
    fn edits_do_not_restore_stale_source_and_references_survive() {
        let mut doc = Document::from_markdown("Test", "[链接][target]\n\n[target]: https://example.org\n").unwrap();
        assert!(doc.html().contains("href=\"https://example.org\""));
        doc.blocks[0].text = "修改 [链接][target]".into();
        assert!(doc.markdown().contains("修改"));
        let reparsed = Document::from_markdown("Test", &doc.markdown()).unwrap();
        assert!(reparsed.html().contains("href=\"https://example.org\""));
    }

    #[test]
    fn html_file_import_preserves_source_but_renders_inert_content() {
        let source = "<!doctype html><html><head><style>p {color:red}</style></head><body><h1>中文</h1><p onclick='run()'>hello <b>world</b></p><script>attack()</script><img src='file:///etc/passwd'><a href='jav&#97;script:run()'>bad</a><table><tr><td>cell</td></tr></table></body></html>";
        let doc = import_file("中文.HTML", source.as_bytes()).unwrap();
        assert_eq!(doc.title, "中文");
        assert_eq!(doc.markdown(), source);
        assert!(doc.is_html_source());
        let html = doc.html();
        for unsafe_text in ["<script", "attack()", "onclick", "file:", "javascript:", "<style", "color:red"] { assert!(!html.contains(unsafe_text), "{html}"); }
        assert!(html.contains("<b>world</b>"));
        assert!(html.contains("<td>cell</td>"));
        assert!(import_file("bad.html", &[0xff]).is_err());
        assert!(import_file("large.md", &vec![b'x'; MAX_BODY+1]).is_err());
        assert!(import_file("program.exe", b"hello").is_err());
    }
}
