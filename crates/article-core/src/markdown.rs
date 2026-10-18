//! Explain which structures need editable source blocks or remain literal.
//!
//! The first pass deliberately uses the same CommonMark parser as
//! rich text converter. The second recognizes extensions preserved by the
//! source-block importer, including features with no dedicated renderer.
use crate::document::{valid_id, validate_link};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Imported intact as an editable source block, rendered in previews.
    SourceBlock,
    /// Applying otherwise-valid source succeeds with this syntax left literal.
    Literal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Html,
    InlineCode,
    CodeBlock,
    ExternalImage,
    NonHttpsLink,
    NestedList,
    NestedQuote,
    HeadingLevel,
    OrderedListStart,
    Table,
    TaskList,
    Strikethrough,
    FrontMatter,
    Math,
    TableOfContents,
    PageBreak,
    EmojiShortcode,
    LinkedImage,
    Footnote,
    Alert,
    DescriptionList,
}

/// One entry per feature, located at its first occurrence (one-based line).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MarkdownIssue {
    pub feature: Feature,
    pub effect: Effect,
    pub line: usize,
}

/// Audit import fidelity. This is a diagnostic, not validation: normal document
/// size, title and image-count limits still belong to `Document::validate`.
pub fn inspect(source: &str) -> Vec<MarkdownIssue> {
    let mut issues = Vec::new();
    let mut record = |feature, effect, offset: usize| {
        if !issues
            .iter()
            .any(|issue: &MarkdownIssue| issue.feature == feature)
        {
            issues.push(MarkdownIssue {
                feature,
                effect,
                line: source.as_bytes()[..offset]
                    .iter()
                    .filter(|byte| **byte == b'\n')
                    .count()
                    + 1,
            });
        }
    };
    let mut lists = 0;
    let mut quotes = 0;
    let mut links = 0;
    let mut in_code = false;
    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Html(_) | Event::InlineHtml(_) => {
                record(Feature::Html, Effect::SourceBlock, range.start)
            }
            Event::Code(_) => record(Feature::InlineCode, Effect::SourceBlock, range.start),
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                record(Feature::CodeBlock, Effect::SourceBlock, range.start);
            }
            Event::End(TagEnd::CodeBlock) => in_code = false,
            Event::Start(Tag::Image { dest_url, .. }) => {
                if !dest_url.strip_prefix("asset:").is_some_and(valid_id) {
                    record(Feature::ExternalImage, Effect::SourceBlock, range.start);
                }
                if links > 0 {
                    record(Feature::LinkedImage, Effect::SourceBlock, range.start);
                }
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                links += 1;
                if validate_link(&dest_url).is_err() {
                    record(Feature::NonHttpsLink, Effect::SourceBlock, range.start);
                }
            }
            Event::End(TagEnd::Link) => links -= 1,
            Event::Start(Tag::List(start)) => {
                if lists > 0 {
                    record(Feature::NestedList, Effect::SourceBlock, range.start);
                }
                if start.is_some_and(|number| number != 1) {
                    record(Feature::OrderedListStart, Effect::SourceBlock, range.start);
                }
                lists += 1;
            }
            Event::End(TagEnd::List(_)) => lists -= 1,
            Event::Start(Tag::BlockQuote(_)) => {
                if quotes > 0 {
                    record(Feature::NestedQuote, Effect::SourceBlock, range.start);
                }
                quotes += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => quotes -= 1,
            Event::Start(Tag::Heading { level, .. }) if ![2, 3].contains(&(level as u8)) => {
                record(Feature::HeadingLevel, Effect::SourceBlock, range.start)
            }
            Event::Start(Tag::Paragraph) => match source[range.clone()].trim() {
                "[TOC]" | "[TOCM]" => {
                    record(Feature::TableOfContents, Effect::SourceBlock, range.start)
                }
                "[========]" => record(Feature::PageBreak, Effect::Literal, range.start),
                text if text.contains("[^") => record(Feature::Footnote, Effect::SourceBlock, range.start),
                _ => (),
            },
            Event::Text(text) if !in_code => {
                if text.contains("[^") || source[range.clone()].contains("[^") {
                    record(Feature::Footnote, Effect::SourceBlock, range.start);
                }
                if text.contains(":smiley:")
                    || text.contains(":star:")
                    || text.contains(":fa-")
                    || text.contains(":editormd-")
                {
                    record(Feature::EmojiShortcode, Effect::SourceBlock, range.start);
                }
            }
            _ => (),
        }
    }
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_MATH | Options::ENABLE_FOOTNOTES | Options::ENABLE_GFM | Options::ENABLE_DEFINITION_LIST;
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        match event {
            Event::FootnoteReference(_) | Event::Start(Tag::FootnoteDefinition(_)) => record(Feature::Footnote, Effect::SourceBlock, range.start),
            Event::Start(Tag::BlockQuote(Some(_))) => record(Feature::Alert, Effect::SourceBlock, range.start),
            Event::Start(Tag::DefinitionList) => record(Feature::DescriptionList, Effect::SourceBlock, range.start),
            Event::Start(Tag::Table(_)) => record(Feature::Table, Effect::SourceBlock, range.start),
            Event::TaskListMarker(_) => record(Feature::TaskList, Effect::SourceBlock, range.start),
            Event::Start(Tag::Strikethrough) => {
                record(Feature::Strikethrough, Effect::SourceBlock, range.start)
            }
            Event::Start(Tag::MetadataBlock(_)) => {
                record(Feature::FrontMatter, Effect::SourceBlock, range.start)
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                record(Feature::Math, Effect::SourceBlock, range.start)
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language)))
                if matches!(language.as_ref(), "math" | "katex" | "latex") =>
            {
                record(Feature::Math, Effect::SourceBlock, range.start)
            }
            _ => (),
        }
    }
    issues.sort_by_key(|issue| issue.line);
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn supported_visual_subset_has_no_issues() {
        let source = "普通 **粗体** 和 *斜体*\n\n## 二级\n\n### 三级\n\n> 引用\n\n- 项目\n\n1. 项目\n\n[链接](https://example.org)\n\n![照片](asset:image_1)\n\n---";
        assert!(inspect(source).is_empty());
        Document::from_markdown("文章", source).unwrap();
    }

    #[test]
    fn reports_source_blocks_with_unicode_line_numbers() {
        let source = "中文👩‍💻\n\n<a href=\"https://example.org\">链接</a>\n\n![图](https://example.org/image.png)\n\n[邮箱](mailto:test@example.org)\n\n`内联代码`\n\n```rust\nfn main() {}\n```\n\n- 外层\n  - 内层";
        let issues = inspect(source);
        for (feature, line) in [
            (Feature::Html, 3),
            (Feature::ExternalImage, 5),
            (Feature::NonHttpsLink, 7),
            (Feature::InlineCode, 9),
            (Feature::CodeBlock, 11),
            (Feature::NestedList, 16),
        ] {
            assert!(
                issues.contains(&MarkdownIssue {
                    feature,
                    effect: Effect::SourceBlock,
                    line
                }),
                "{issues:?}"
            );
        }
        assert_eq!(issues.len(), 6);
        assert_eq!(Document::from_markdown("文章", source).unwrap().markdown(), source);
    }

    #[test]
    fn distinguishes_literal_extensions_from_source_blocks() {
        let source = "---\ntitle: Metadata\n---\n\n[TOC]\n\n~~删除线~~\n\n- [x] 完成\n\n| 项目 | 值 |\n| --- | --- |\n| 茶 | 2 |\n\n$$E=mc^2$$\n\n[========]\n\n:smiley:";
        let issues = inspect(source);
        assert!(issues.iter().any(|issue| issue.feature == Feature::Table && issue.effect == Effect::SourceBlock));
        assert!(issues.iter().any(|issue| issue.feature == Feature::Math && issue.effect == Effect::SourceBlock));
        for feature in [
            Feature::FrontMatter,
            Feature::TableOfContents,
            Feature::Strikethrough,
            Feature::TaskList,
            Feature::Table,
            Feature::Math,
            Feature::PageBreak,
            Feature::EmojiShortcode,
        ] {
            assert!(
                issues.iter().any(|issue| issue.feature == feature),
                "{issues:?}"
            );
        }
        Document::from_markdown("文章", source).unwrap();
    }

    #[test]
    fn code_examples_do_not_report_their_contents_as_rendered_features() {
        let source = "```text\n<table>\n[TOC]\n:smiley:\n| a | b |\n| --- | --- |\n```";
        assert_eq!(
            inspect(source),
            vec![MarkdownIssue {
                feature: Feature::CodeBlock,
                effect: Effect::SourceBlock,
                line: 1
            }]
        );
    }
}
