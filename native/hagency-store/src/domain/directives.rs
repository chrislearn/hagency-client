//! `/thread` session directives (TS parity: `backend-v2.js:2131-2310`).
//!
//! An operator addresses an agent with `/thread model <name|default>` or
//! `/thread mode <plan|auto>` in a room. It is a directive, never chat input:
//! it updates the thread session's `model_override`/`mode_override` and is
//! answered in-thread with a confirmation notice. A non-operator directive is
//! consumed with a refusal notice, not applied; a malformed one is consumed
//! with a usage notice. Delivery never fails because of a directive
//! (`backend-v2.js:2261-2266`): the bridge consumes it and answers instead of
//! posting a delivery-failure warning into the room.
//!
//! This module is the PURE half — the parser and the confirmation text — so the
//! TS-visible words and the decision logic are asserted directly, exactly as the
//! retained `parseThreadSessionDirective` / `threadSessionDirectiveConfirmation`
//! are. The store applies the resulting override and the host answers through
//! the command-notice path; neither is here.

use serde::{Deserialize, Serialize};

/// The retained usage line (`backend-v2.js:2143`), verbatim.
pub const THREAD_DIRECTIVE_USAGE: &str =
    "usage: /thread model <name|default> | /thread mode <plan|auto>";

/// The non-operator refusal (`backend-v2.js:2266`), verbatim.
pub const THREAD_DIRECTIVE_OPERATOR_REFUSAL: &str =
    "/thread directives require operator trust; the message was not delivered as chat.";

/// A malformed model's refusal (`backend-v2.js:2152`), verbatim.
pub const THREAD_DIRECTIVE_MODEL_REFUSAL: &str =
    "model must be a plain model name or alias (max 64 chars)";

/// A session's write mode override (`router/src/store.ts:82`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreadMode {
    Plan,
    Auto,
}

/// What a valid directive asks to set. `None` is the `default|reset|clear`
/// verb, which clears the override (`backend-v2.js:2150,2158`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadDirective {
    Model(Option<String>),
    Mode(Option<ThreadMode>),
}

/// The parse outcome. `None` means the body is not a `/thread` directive at
/// all; `Err(body)` is a malformed directive whose `body` is the in-thread
/// notice (usage or model refusal), never a delivery failure.
pub fn parse(body: &str) -> Option<Result<ThreadDirective, String>> {
    let body = strip_address(body);
    if !body.starts_with("/thread") {
        return None;
    }
    // `/thread\s+(\S+)(?:\s+(\S+))?\s*$` — the whole trimmed body, two tokens max.
    let rest = body["/thread".len()..].trim();
    if rest.is_empty() {
        return Some(Err(THREAD_DIRECTIVE_USAGE.into()));
    }
    let mut tokens = rest.split_whitespace();
    let key = tokens.next().unwrap_or_default().to_ascii_lowercase();
    let value = tokens.next();
    if tokens.next().is_some() {
        // A third token means the line did not match the two-token shape.
        return Some(Err(THREAD_DIRECTIVE_USAGE.into()));
    }
    match key.as_str() {
        "model" => Some(match value {
            None => Err(THREAD_DIRECTIVE_USAGE.into()),
            Some(v) if is_clear_word(v) => Ok(ThreadDirective::Model(None)),
            Some(v) if is_model(v) => Ok(ThreadDirective::Model(Some(v.to_owned()))),
            Some(_) => Err(THREAD_DIRECTIVE_MODEL_REFUSAL.into()),
        }),
        "mode" => Some(match value {
            None => Err(THREAD_DIRECTIVE_USAGE.into()),
            Some(v) if is_clear_word(v) => Ok(ThreadDirective::Mode(None)),
            Some(v) if v.eq_ignore_ascii_case("plan") => Ok(ThreadDirective::Mode(Some(ThreadMode::Plan))),
            Some(v) if v.eq_ignore_ascii_case("auto") => Ok(ThreadDirective::Mode(Some(ThreadMode::Auto))),
            Some(_) => Err(THREAD_DIRECTIVE_USAGE.into()),
        }),
        _ => Some(Err(THREAD_DIRECTIVE_USAGE.into())),
    }
}

/// The confirmation notice (`threadSessionDirectiveConfirmation`,
/// `backend-v2.js:2161-2169`), verbatim: the resolved model and mode, plus the
/// write warning only when mode is `auto`.
pub fn confirmation(model_override: Option<&str>, mode_override: Option<ThreadMode>) -> String {
    let model = model_override.map_or("model=default".to_owned(), |m| format!("model={m}"));
    let mode = match mode_override {
        Some(ThreadMode::Auto) => "mode=auto".to_owned(),
        Some(ThreadMode::Plan) => "mode=plan".to_owned(),
        None => "mode=default (read-only)".to_owned(),
    };
    let write_warning = if mode_override == Some(ThreadMode::Auto) {
        " Runners in this thread may now write to the agent workspace (writes stay serialized by workspace lease)."
    } else {
        ""
    };
    format!("Thread session updated: {model}, {mode}.{write_warning}")
}

/// `default|reset|clear` clears the override (`backend-v2.js:2150,2158`).
fn is_clear_word(value: &str) -> bool {
    value.eq_ignore_ascii_case("default")
        || value.eq_ignore_ascii_case("reset")
        || value.eq_ignore_ascii_case("clear")
}

/// `THREAD_DIRECTIVE_MODEL_PATTERN` (`backend-v2.js:2129`):
/// `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$` — a plain model name/alias ≤64 chars.
fn is_model(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else { return false };
    if !first.is_ascii_alphanumeric() || value.len() > 64 {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Strip mention pills and a leading `@name` address so the directive can lead
/// the visible text (`backend-v2.js:2133-2137`): markdown links first, then any
/// leading `@local[:server]` tokens.
fn strip_address(body: &str) -> String {
    let mut stripped = body;
    // `\[[^\]]*\]\([^)]*\)` → ' ' (markdown link pills).
    let mut out = String::with_capacity(stripped.len());
    let mut rest = stripped;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        out.push(' ');
        match rest[open + 1..].find("](").map(|i| open + 1 + i) {
            Some(close_open) => match rest[close_open + 2..].find(')') {
                Some(end) => {
                    let skip = close_open + 2 + end + 1;
                    rest = &rest[skip..];
                }
                None => {
                    out.push_str(&rest[open..]);
                    rest = "";
                }
            },
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    stripped = out.trim();
    // `^(?:@[\w.-]+[:,]?\s+)+` — leading @name address tokens.
    loop {
        let s = stripped.trim_start();
        if !s.starts_with('@') {
            break;
        }
        let localpart: String = s[1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':'))
            .collect();
        if localpart.is_empty() {
            break;
        }
        let mut after = &s[1 + localpart.len()..];
        if let Some(r) = after.strip_prefix(':') {
            // skip the trailing `:` colon separator, then whitespace
            after = r;
        } else if let Some(r) = after.strip_prefix(',') {
            after = r;
        }
        if !after.trim_start().is_empty() && !after.starts_with(char::is_whitespace) {
            break;
        }
        stripped = after.trim_start();
    }
    stripped.to_owned()
}
