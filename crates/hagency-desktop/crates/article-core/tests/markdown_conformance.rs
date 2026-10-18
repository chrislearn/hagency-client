use article_core::{
    markdown_render::{self, Dialect},
    render::Renderer,
};
use serde::Deserialize;

#[test]
fn double_dollar_math_keeps_inline_context_in_editor_and_reader() {
    #[derive(Default)]
    struct Math(Vec<bool>);
    impl Renderer for Math {
        fn math(&mut self, formula: &article_core::math::Formula) -> String {
            self.0.push(formula.display);
            "FORMULA".into()
        }
    }
    let source = "$$E=mc^2$$\n\n行内的公式$$E=mc^2$$行内的公式，行内的$$E=mc^2$$公式。\n\n$$x > y$$\n\n```math\nx^2\n```\n\n> $$x^2$$\n\n- $$y^2$$\n\nText **$$z^2$$** after\n";
    let mut reader = Math::default();
    markdown_render::render(source, &mut reader);
    assert_eq!(reader.0, [true, false, false, true, true, true, true, false]);
    let document = article_core::document::Document::from_markdown("Math", source).unwrap();
    let mut editor = Math::default();
    markdown_render::render_editor(&document, &mut editor);
    assert_eq!(editor.0, reader.0);
}

#[test]
fn editor_blocks_keep_whole_document_context_and_update_after_edits() {
    use article_core::document::Document;
    struct Plain;
    impl Renderer for Plain {}
    let source = "---\ntitle: Metadata\n---\n\n[TOC]\n\n# Same\n\n# Same\n\n[reference][target]\n\n| A | B |\n| - | - |\n| one | two |\n\nFootnote[^n]\n\n[^n]: note\n\n[target]: https://example.org\n";
    let mut document = Document::from_markdown("Editor", source).unwrap();
    let rendered = markdown_render::render_editor(&document, &mut Plain);
    assert_eq!(rendered.len(), document.blocks.len());
    assert!(rendered[0].html.trim().is_empty(), "Front matter is metadata, not article text");
    let toc = document.blocks.iter().position(|b| b.text.trim() == "[TOC]").unwrap();
    assert!(rendered[toc].html.contains("href=\"#same-1\""));
    let reference = document.blocks.iter().position(|b| b.text.contains("reference")).unwrap();
    assert!(rendered[reference].html.contains("href=\"https://example.org\""));
    let table = document.blocks.iter().position(|b| b.text.contains("| A | B |")).unwrap();
    assert!(rendered[table].html.contains("<table>"));
    assert!(rendered.iter().any(|b| b.html.contains("id=\"fn-n\"")));
    assert_eq!(document.markdown(), source, "Rendering must not rewrite imported source");
    document.blocks[table].text = document.blocks[table].text.replace("one", "edited");
    let edited = markdown_render::render_editor(&document, &mut Plain);
    assert!(edited[table].html.contains("edited"));
    assert!(edited[toc].html.contains("href=\"#same-1\""));
}

#[test]
fn editor_full_sample_matches_reader_semantics() {
    use article_core::document::Document;
    struct Plain;
    impl Renderer for Plain {}
    let source = include_str!("../../../lab/article-editor/render-comparison/source.md");
    let document = Document::from_markdown("Editor.md", source).unwrap();
    let editor = markdown_render::render_editor(&document, &mut Plain);
    let preview = markdown_render::render(source, &mut Plain);
    // The visual paragraph model may normalize whitespace between blocks, but
    // all rendered tags, attributes and non-whitespace text must be identical.
    let normalize = |blocks: Vec<markdown_render::RenderedBlock>| {
        canonical(&blocks.iter().map(|b| b.html.as_str()).collect::<String>())
            .into_iter().map(|token| if token.starts_with("text:") {
                token.split_whitespace().collect::<Vec<_>>().join(" ")
            } else { token }).filter(|token| token != "text:").collect::<Vec<_>>()
    };
    assert_eq!(normalize(editor), normalize(preview));
    assert_eq!(document.markdown(), source);
}
#[derive(Deserialize)]
struct Example {
    example: usize,
    section: String,
    markdown: String,
    html: String,
}
fn run(data: &str, dialect: Dialect) {
    let examples: Vec<Example> = serde_json::from_str::<Vec<Example>>(data)
        .unwrap()
        .into_iter()
        .filter(|e| !matches!(dialect, Dialect::Gfm) || e.section.contains("(extension)"))
        .collect();
    let mut failures = Vec::new();
    for e in &examples {
        let got = markdown_render::specification_html(&e.markdown, dialect);
        if canonical(&got) != canonical(&e.html) {
            failures.push(format!(
                "{} / {}\nsource: {:?}\nexpected: {:?}\nactual: {:?}",
                e.example, e.section, e.markdown, e.html, got
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} failed:\n{}",
        failures.len(),
        examples.len(),
        failures.join("\n")
    );
}
#[test]
fn commonmark_0312_specification() {
    run(
        include_str!("fixtures/commonmark-0.31.2.json"),
        Dialect::CommonMark,
    );
}
#[test]
fn gfm_029_specification() {
    run(include_str!("fixtures/gfm-0.29.json"), Dialect::Gfm);
}

struct Plain;
impl Renderer for Plain {}
#[test]
fn native_document_resolves_global_structure_and_keeps_source() {
    let source = "# 重复标题\n\n[TOC]\n\n3. three\n4. four\n\n# 重复标题\n\nVisit www.example.com and a@example.com.\n\nNote[^a] and again[^a].\n\n> [!WARNING]\n> Watch **out**.\n\n[^a]: 中文 footnote.\n\n[go](#重复标题-1)\n";
    let doc = article_core::document::Document::from_markdown("Native", source).unwrap();
    let blocks = markdown_render::render(&doc.markdown(), &mut Plain);
    let html = blocks.iter().map(|b| b.html.as_str()).collect::<String>();
    assert!(html.contains("<ol start=\"3\">"), "{html}");
    assert!(html.contains("href=\"http://www.example.com\""), "{html}");
    assert!(html.contains("href=\"mailto:a@example.com\""), "{html}");
    assert!(html.contains("href=\"#重复标题-1\""), "{html}");
    assert!(
        blocks
            .iter()
            .flat_map(|b| &b.anchors)
            .any(|a| a == "重复标题-1"),
        "{html}"
    );
    assert!(
        html.contains("中文 footnote") && html.contains("Warning"),
        "{html}"
    );
    assert!(html.contains("href=\"#fn-a\""), "{html}");
    assert_eq!(doc.markdown(), source);
}

#[test]
fn native_document_does_not_activate_source_code_or_drop_hard_breaks() {
    let source = "a  \nb\n\nsoft\nbreak\n\n- [x] complete\n- [ ] incomplete\n\n```js\nconst text = ':smiley:';\n```\n\n<script>attack()</script>\n\n[bad](javascript:attack)";
    let blocks = markdown_render::render(source, &mut Plain);
    let html = blocks.iter().map(|b| b.html.as_str()).collect::<String>();
    assert!(html.contains("a<br>\nb"), "{html}");
    assert!(html.contains("soft\nbreak"), "{html}");
    assert!(html.contains("☑") && html.contains("☐"), "{html}");
    assert!(html.contains(":smiley:"));
    assert!(
        !html.contains("javascript:") && !html.contains("<script"),
        "{html}"
    );
}

#[test]
fn unrelated_visual_edits_preserve_extended_markdown_semantics() {
    let source = "Editable paragraph.\n\n1. first\n2. second\n\nLine  \nbreak\n\nNote[^a].\n\n> [!NOTE]\n> Alert body.\n\n[^a]: Footnote body.\n";
    let mut doc = article_core::document::Document::from_markdown("Native", source).unwrap();
    doc.blocks[0].text = "Changed paragraph.".into();
    let html = markdown_render::render(&doc.markdown(), &mut Plain)
        .iter()
        .map(|b| b.html.as_str())
        .collect::<String>();
    assert!(
        html.contains("Changed paragraph.") && html.contains("<li>first</li>"),
        "{html}"
    );
    assert!(
        html.contains("Line<br>") && html.contains("href=\"#fn-a\""),
        "{html}"
    );
    assert!(
        html.contains("Footnote body.") && html.contains("Alert body."),
        "{html}"
    );
}

#[test]
fn large_documents_image_bindings_and_zero_start_are_supported() {
    let source = format!(
        "{}\n\n0. zero\n1. first\n\nLAST BLOCK",
        "Ordinary text 中文.\n\n".repeat(2000)
    );
    let mut doc = article_core::document::Document::from_markdown("Long", &source).unwrap();
    doc.resource_bindings
        .insert("images/a.png".into(), "asset1".into());
    doc.validate().unwrap();
    assert!(doc.asset_ids().iter().any(|id| id == "asset1"));
    let blocks = markdown_render::render(&doc.markdown(), &mut Plain);
    assert!(blocks.last().unwrap().text.contains("LAST BLOCK"));
    assert!(blocks.iter().any(|b| b.html.contains("<ol start=\"0\">")));
    assert_eq!(
        article_core::render::decode_fragment("a+b%20%E4%B8%AD"),
        "a+b 中"
    );
}

// HTML-equivalent attribute ordering and void-element slash spellings are not
// Markdown differences. Text, nesting and attribute values remain exact.
fn canonical(html: &str) -> Vec<String> {
    use html5ever::{
        buffer_queue::BufferQueue,
        tokenizer::{Token, TokenSink, TokenSinkResult, Tokenizer},
    };
    use std::cell::RefCell;
    struct Sink(RefCell<Vec<String>>);
    impl TokenSink for Sink {
        type Handle = ();
        fn process_token(&self, t: Token, _: u64) -> TokenSinkResult<()> {
            let mut out = self.0.borrow_mut();
            match t {
                Token::TagToken(t) => {
                    let mut a = t
                        .attrs
                        .iter()
                        .map(|a| format!("{}={}", a.name.local, a.value))
                        .collect::<Vec<_>>();
                    a.sort();
                    out.push(format!("{:?}:{}:{:?}", t.kind, t.name, a));
                }
                Token::CharacterTokens(t) => {
                    if let Some(last) = out.last_mut().filter(|s| s.starts_with("text:")) {
                        last.push_str(&t);
                    } else {
                        out.push(format!("text:{t}"));
                    }
                }
                Token::CommentToken(t) => out.push(format!("comment:{t}")),
                _ => {}
            }
            TokenSinkResult::Continue
        }
    }
    let q = BufferQueue::default();
    q.push_back(html.into());
    let t = Tokenizer::new(Sink(RefCell::new(Vec::new())), Default::default());
    let _ = t.feed(&q);
    t.end();
    t.sink.0.into_inner()
}
