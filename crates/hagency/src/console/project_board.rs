//! One closed operation contract for native board requests; no arbitrary proxy.
use super::server_login::OwnerError;
use hagency_core::project_board::*;
use serde_json::Value;
fn invalid() -> OwnerError {
    OwnerError {
        status: 400,
        code: "invalid_board_payload".into(),
    }
}
fn key(value: &str) -> Result<(), OwnerError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(invalid());
    }
    Ok(())
}
fn query(query: Query) -> Result<String, OwnerError> {
    query.validate().map_err(|_| invalid())?;
    let value = serde_json::to_value(query).map_err(|_| invalid())?;
    let mut encoded = reqwest::Url::parse("http://local.invalid/").map_err(|_| invalid())?;
    for (key, value) in value.as_object().ok_or_else(invalid)? {
        if value.is_null() {
            continue;
        }
        let value = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        encoded.query_pairs_mut().append_pair(key, &value);
    }
    Ok(encoded.query().unwrap_or("").into())
}
pub(super) fn request(
    project: &str,
    action: BoardAction,
) -> Result<(reqwest::Method, String, Option<Value>), OwnerError> {
    use reqwest::Method;
    key(project)?;
    let root = format!("/hagency/api/v1/projects/{project}");
    let task = match &action {
        BoardAction::Detail { task }
        | BoardAction::Comments { task, .. }
        | BoardAction::Activity { task, .. }
        | BoardAction::Update { task, .. }
        | BoardAction::Move { task, .. }
        | BoardAction::Archive { task, .. }
        | BoardAction::Restore { task, .. }
        | BoardAction::Comment { task, .. } => {
            key(task)?;
            Some(task.clone())
        }
        _ => None,
    };
    let task_root = format!("{root}/tasks/{}", task.unwrap_or_default());
    let serialize = |input| serde_json::to_value(input).map_err(|_| invalid());
    Ok(match action {
        BoardAction::Snapshot { query: q } => {
            (Method::GET, format!("{root}/board?{}", query(q)?), None)
        }
        BoardAction::Page { query: q } => {
            (Method::GET, format!("{root}/tasks?{}", query(q)?), None)
        }
        BoardAction::Detail { .. } => (Method::GET, task_root, None),
        BoardAction::Changes { after } => {
            if after < 0 {
                return Err(invalid());
            }
            (
                Method::GET,
                format!("{root}/board/changes?after={after}&limit=100"),
                None,
            )
        }
        BoardAction::Comments { after, .. } => {
            if after < 0 {
                return Err(invalid());
            }
            (
                Method::GET,
                format!("{task_root}/comments?after={after}&limit=50"),
                None,
            )
        }
        BoardAction::Activity { after, .. } => {
            if after < 0 {
                return Err(invalid());
            }
            (
                Method::GET,
                format!("{task_root}/activity?after={after}&limit=50"),
                None,
            )
        }
        BoardAction::ConfigureColumns { input } => (
            Method::PATCH,
            format!("{root}/board/columns"),
            Some(serde_json::to_value(input).map_err(|_| invalid())?),
        ),
        BoardAction::Create { input } => (
            Method::POST,
            format!("{root}/tasks"),
            Some(serde_json::to_value(input).map_err(|_| invalid())?),
        ),
        BoardAction::Update { input, .. } => (
            Method::PATCH,
            task_root,
            Some(serde_json::to_value(input).map_err(|_| invalid())?),
        ),
        BoardAction::Move { input, .. } => (
            Method::POST,
            format!("{task_root}/move"),
            Some(serde_json::to_value(input).map_err(|_| invalid())?),
        ),
        BoardAction::Archive { input, .. } => (
            Method::POST,
            format!("{task_root}/archive"),
            Some(serialize(input)?),
        ),
        BoardAction::Restore { input, .. } => (
            Method::POST,
            format!("{task_root}/restore"),
            Some(serialize(input)?),
        ),
        BoardAction::Comment { input, .. } => (
            Method::POST,
            format!("{task_root}/comments"),
            Some(serde_json::to_value(input).map_err(|_| invalid())?),
        ),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn board_operations_are_closed_and_queries_are_encoded() {
        let (_, path, _) = request(
            "prj_one",
            BoardAction::Snapshot {
                query: Query {
                    query: Some("design & review/中文".into()),
                    ..Default::default()
                },
            },
        )
        .unwrap();
        assert!(path.contains("design+%26+review%2F"));
        for project in ["../other", "a?proxy=b", "https://elsewhere", "a/b"] {
            assert!(request(project, BoardAction::Changes { after: 0 }).is_err());
        }
        assert!(
            request(
                "p",
                BoardAction::Detail {
                    task: "../other".into()
                }
            )
            .is_err()
        );
        assert!(
            request(
                "p",
                BoardAction::Page {
                    query: Query {
                        limit: Some(101),
                        ..Default::default()
                    }
                }
            )
            .is_err()
        );
    }
}
