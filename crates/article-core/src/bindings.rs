//! Optional, statically admitted Octoscript L0 field bindings.
use crate::editing::EditorState;
use octoscript_ui_l0::{InstanceStore, NodeValue, RealizeLimits, UiNode};
pub const SOURCE: &str = include_str!("../resources/app.card");
/// A deliberately small native kit adapter. Every Field and event comes from
/// the admitted L0 tree. Authority and publication controls belong to the embedding host.
pub struct EditorField {
    pub text: String,
    pub placeholder: String,
    pub event: String,
}
pub fn realize_editor(draft: &EditorState, chinese: bool) -> Result<Vec<EditorField>, String> {
    let data = serde_json::json!({"title":draft.title,"markdown":draft.markdown,"env":{"locale":{"lang":if chinese {"zh"} else {"en"}}}});
    let report = octoscript_ui_l0::realize(
        SOURCE,
        &data,
        RealizeLimits {
            max_nodes: 16,
            max_depth: 8,
            max_collection: 2,
            max_work: 64,
        },
    );
    let root = report.complete_root()?;
    fn fields(node: &UiNode, out: &mut Vec<EditorField>) -> Result<(), String> {
        match node.kind.as_str() {
            "Surface" | "Col" => (),
            "Field" => {
                let arg = |name: &str| node.args.iter().find(|(k, _)| k == name).map(|(_, v)| v);
                let (
                    Some(NodeValue::Text(text)),
                    Some(NodeValue::Text(placeholder)),
                    Some(NodeValue::Event(event)),
                ) = (arg("text"), arg("placeholder"), arg("on_change"))
                else {
                    return Err("Invalid article field binding".into());
                };
                out.push(EditorField {
                    text: text.clone(),
                    placeholder: placeholder.clone(),
                    event: event.clone(),
                });
            }
            _ => return Err("Unsupported article kit role".into()),
        }
        for child in &node.children {
            fields(child, out)?;
        }
        Ok(())
    }
    let mut out = Vec::new();
    fields(root, &mut out)?;
    if out.len() != 2 || out[0].event != "title_changed" || out[1].event != "markdown_changed" {
        return Err("Unsupported article field mapping".into());
    }
    Ok(out)
}
pub fn apply_input(draft: &mut EditorState, event: &str, value: &str) -> Result<(), String> {
    let mut store = InstanceStore::default();
    store.set_cell(
        octoscript_ui_l0::CARD_STATE_KEY,
        "title",
        serde_json::json!(draft.title),
    );
    store.set_cell(
        octoscript_ui_l0::CARD_STATE_KEY,
        "markdown",
        serde_json::json!(draft.markdown),
    );
    if matches!(event, "title_changed") && value == draft.title
        || matches!(event, "markdown_changed") && value == draft.markdown
    {
        return Ok(());
    }
    // Top-level state dispatch uses the root instance; only these declared
    // setter events exist. User text is a JSON string, never script source.
    if !octoscript_ui_l0::dispatch_with(
        SOURCE,
        &mut store,
        "root",
        event,
        Some(&serde_json::Value::String(value.into())),
    ) {
        return Err("Unknown editor event".into());
    }
    let field = match event {
        "title_changed" => "title",
        "markdown_changed" => "markdown",
        _ => return Err("Unknown editor event".into()),
    };
    let text = store
        .get(octoscript_ui_l0::CARD_STATE_KEY, field)
        .and_then(|v| v.as_str())
        .ok_or("Missing editor state")?;
    let mut next = draft.clone();
    if field == "title" {
        next.title = text.into()
    } else {
        next.markdown = text.into()
    };
    next.validate()?;
    *draft = next;
    Ok(())
}

