//! Capture the installed article import/render pipeline without an account.
//! Usage: article_render_comparison SOURCE.md CSS OUTPUT_DIR
use article_core::document::Document;
use makepad_html_renderer::{render_html, RenderOptions, ResourceMap};
use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};
use std::{fs, path::PathBuf};

fn body_with_math(
    document: &Document,
    images: &article_makepad::content::Images,
    out: &std::path::Path,
) -> Result<(String, ResourceMap, usize), String> {
    let mut resources = ResourceMap::default();
    let mut renderer = article_makepad::content::HtmlRenderer {
        images, size: 16.0, ink: 0x191919, error: None, math_count: 0,
        register: |id: &str, png: &[u8]| {
            resources.insert_image(id, png.to_vec()).map_err(|e| e.to_string())?;
            fs::write(out.join(id), png).map_err(|e| e.to_string())?;
            Ok(id.to_owned())
        },
    };
    let body = document.blocks.iter().map(|block| document.block_html_with_renderer(block, &mut renderer)).collect::<String>();
    let count = renderer.math_count;
    if let Some(error) = renderer.error { return Err(error); }
    Ok((body, resources, count))
}

fn raw_markdown(source: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    struct Browser;
    impl article_core::render::Renderer for Browser {
        fn image(&mut self, image: &article_core::render::ImageRequest<'_>) -> String {
            format!("<img src=\"{}\" width=\"{}\" alt=\"{}\">", article_core::document::escape(image.url), image.width.unwrap_or(180), article_core::document::escape(image.alt))
        }
    }
    let mut fence: Option<(String, String)> = None;
    let mut metadata = false;
    let events = Parser::new_ext(source, options).filter_map(|event| match event {
        Event::Start(Tag::MetadataBlock(_)) => {
            metadata = true;
            None
        }
        Event::End(TagEnd::MetadataBlock(_)) => {
            metadata = false;
            None
        }
        _ if metadata => None,
        Event::Start(Tag::CodeBlock(kind)) => {
            let language = match kind { pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(), _ => String::new() };
            fence = Some((language, String::new())); None
        }
        Event::Text(text) if fence.is_some() => { fence.as_mut().unwrap().1.push_str(&text); None }
        Event::End(TagEnd::CodeBlock) if fence.is_some() => {
            let (lang, source) = fence.take().unwrap();
            let rendered = if article_makepad::diagram::is_language(&lang) {
                article_makepad::diagram::svg(&lang, &source)
                    .map(|svg| format!("<div style=\"max-width:100%\">{svg}</div>"))
                    .unwrap_or_else(|error| article_makepad::diagram::fallback(&lang, &source, &error))
            } else { article_makepad::content::code_html(&lang, &source) };
            Some(Event::Html(rendered.into()))
        }
        Event::Text(text) => Some(Event::InlineHtml(article_core::render::emoji_text(&text, &mut Browser).into())),
        _ => Some(event),
    });
    let mut output = String::new();
    html::push_html(&mut output, events);
    output
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("Pass SOURCE.md CSS OUTPUT_DIR".into());
    }
    let source = fs::read_to_string(&args[1])?;
    let css = format!("{}\n{}", fs::read_to_string(&args[2])?, article_makepad::content::CSS);
    let output = PathBuf::from(&args[3]);
    fs::create_dir_all(&output)?;
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(output.join("resources.json"))?)?;
    let mut images = std::collections::BTreeMap::new();
    for item in manifest.as_array().ok_or("Expected image manifest")? {
        if let (Some(url), Some(file)) = (item["url"].as_str(), item["file"].as_str()) {
            let bytes = fs::read(output.join("assets").join(file))?;
            images.insert(url.to_owned(), article_makepad::content::prepare_image(&bytes)?);
        }
    }
    let images = std::sync::Arc::new(images);
    let sections = [
        ("01-opening", "Opening / line breaks", "---\ntitle:"),
        ("02-headings", "Heading levels", "# Heading 1\n"),
        ("03-text", "Text, quotes and links", "### 字符效果和横线等"),
        (
            "04-code",
            "Inline and fenced code",
            "### 多语言代码高亮 Codes",
        ),
        ("05-images", "Images and linked images", "### 图片 Images"),
        ("06-lists", "Nested lists and tasks", "### 列表 Lists"),
        ("07-tables", "Tables and alignment", "### 绘制表格 Tables"),
        (
            "08-entities",
            "Entities and emoji syntax",
            "#### 特殊符号 HTML Entities Codes",
        ),
        ("09-math", "Math / KaTeX source", "### 科学公式 TeX(KaTeX)"),
        (
            "10-diagrams",
            "Flowchart and sequence source",
            "### 分页符 Page break",
        ),
    ];
    let starts: Vec<_> = sections
        .iter()
        .map(|(_, _, marker)| source.find(marker).ok_or("Section missing"))
        .collect::<Result<_, _>>()?;
    let mut records = Vec::new();
    let full = Document::from_markdown("Editor.md", &source)?;
    for (index, (id, name, _)) in sections.iter().enumerate() {
        let start = starts[index];
        let end = starts.get(index + 1).copied().unwrap_or(source.len());
        let section = &source[start..end];
        let resolved = format!("{section}\n\n{}", full.reference_definitions);
        let document = Document::from_markdown("Editor.md", &resolved)?;
        let out = output.join(id);
        fs::create_dir_all(&out)?;
        let (body, resources, math_count) = body_with_math(&document, &images, &out)?;
        let wrap = |body: &str| {
            format!(
                "<!doctype html><html><head><meta charset=\"utf-8\"><title>Editor.md comparison</title><style>{css}</style></head><body><article>{body}</article></body></html>"
            )
        };
        let imported = wrap(&body);
        let reference = wrap(&raw_markdown(&resolved));
        fs::write(out.join("source.md"), section)?;
        fs::write(out.join("rinx.html"), &imported)?;
        fs::write(out.join("browser-reference.html"), reference)?;
        let rendered = render_html(
            &imported,
            RenderOptions {
                width_css: 440,
                viewport_height_css: 700,
                scale: 2.0,
                ..Default::default()
            },
            &resources,
        )?;
        image::save_buffer(
            out.join("blitz.png"),
            &rendered.rgba,
            rendered.width,
            rendered.height,
            image::ColorType::Rgba8,
        )?;
        records.push(serde_json::json!({
            "id":id,"name":name,"source_start_byte":start,"source_end_byte":end,
            "html_blake3":blake3::hash(imported.as_bytes()).to_hex().to_string(),
            "width_css":440,"viewport_height_css":700,"scale":2,
            "height_css":rendered.css_content_height,"width_px":rendered.width,
            "height_px":rendered.height,"clipped":rendered.clipped,
            "denied_resources":rendered.resources.denied,
            "math_formulas":math_count,
        }));
        println!(
            "Rendered {id}: {} px, clipped={}",
            rendered.height, rendered.clipped
        );
    }
    // Keep a whole-document capture too: it exposes the product's height limit.
    let (body, resources, math_count) = body_with_math(&full, &images, &output)?;
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{css}</style></head><body><article>{body}</article></body></html>"
    );
    fs::write(output.join("full-rinx.html"), &html)?;
    let rendered = render_html(
        &html,
        RenderOptions {
            width_css: 440,
            viewport_height_css: 700,
            scale: 2.0,
            ..Default::default()
        },
        &resources,
    )?;
    image::save_buffer(
        output.join("full-blitz.png"),
        &rendered.rgba,
        rendered.width,
        rendered.height,
        image::ColorType::Rgba8,
    )?;
    fs::write(
        output.join("renders.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "source_blake3":blake3::hash(source.as_bytes()).to_hex().to_string(),
            "source_bytes":source.len(),"import_blocks":full.blocks.len(),
            "math_formulas":math_count,
            "blitz_revision":makepad_html_renderer::BLITZ_REVISION,
            "makepad_html_revision":"b16565ae1e56dd7c3a87f00455ae327df599ca3b",
            "css":"Rinx Classic theme, 16 px, line-height 1.85, production table and highlighted-code styles",
            "image_urls":images.len(),
            "section_method":"Contiguous source sections; global reference definitions supplied to each",
            "full_document":{"height_css":rendered.css_content_height,"height_px":rendered.height,"clipped":rendered.clipped},
            "sections":records,
        }))?,
    )?;
    Ok(())
}
