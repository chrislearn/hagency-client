//! Run with an explicit local Markdown file; no network or GUI is involved.
use article_core::{
    document::{Document, MAX_BODY},
    markdown,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Pass a Markdown file path")?;
    let source = std::fs::read_to_string(&path)?;
    let result = Document::from_markdown("Markdown compatibility", &source);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "path": path,
            "bytes": source.len(),
            "size_limit": MAX_BODY,
            "source_blake3": blake3::hash(source.as_bytes()).to_hex().to_string(),
            "import_error": result.as_ref().err(),
            "block_count": result.as_ref().ok().map(|document| document.blocks.len()),
            "exact_source_roundtrip": result.as_ref().ok().map(|document| document.markdown() == source),
            "issues": markdown::inspect(&source),
        }))?
    );
    result?;
    Ok(())
}
