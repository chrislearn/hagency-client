//! #27 — the two console surfaces, asserted on what the operator observes.
//!
//! Every expectation is derived from the retained TypeScript, not guessed:
//!
//! * clear-dirty replies `{ok:true,result:{ok:true}}` (backend-v2.js:8899-8901)
//!   and maps a quarantine refusal to `inspection_required` at 409
//!   (`routerRefusalStatus`, backend-v2.js:1923-1932).
//! * execution-policy GET/PUT reply `{executionPolicy:{yolo},...}` /
//!   `{ok:true,executionPolicy,appliesTo:'next_dispatch'}` and 404 an unknown
//!   agent (backend-v2.js:10865-10885).
use super::*;
use std::time::Duration;

/// A lifecycle-scoped session, exchanged exactly as the lifecycle routes do.
async fn owner(service: &Service) -> String {
    // The issuer shares one rate budget across every scope (authority.rs
    // `issue_scope`): a second issue inside one second answers Busy. Clear the
    // budget; a 429 is a real refusal reported from its body, never retried.
    tokio::time::sleep(Duration::from_millis(1010)).await;
    let mut response = TestClient::post(format!(
        "{BASE}/api/native/v1/console/agent-lifecycle-access"
    ))
    .add_header("host", "127.0.0.1:13300", true)
    .bearer_auth(TOKEN)
    .send(service)
    .await;
    let status = response.status_code;
    let body = response.take_string().await.unwrap_or_default();
    assert_eq!(status, Some(StatusCode::OK), "lifecycle issue refused: {body}");
    let ticket = serde_json::from_str::<Value>(&body).unwrap()["ticket"]
        .as_str()
        .unwrap()
        .to_owned();
    let response = exchange(service, &ticket).await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

/// The dirty flag as the dispatch gate actually reads it.
fn dirty(f: &Fixture, id: &str) -> bool {
    let sql = rusqlite::Connection::open(f.root.path().join("state/domain.sqlite3")).unwrap();
    sql.query_row(
        "SELECT dirty FROM workspace_resources WHERE id=?1",
        [id],
        |r| r.get::<_, bool>(0),
    )
    .unwrap()
}

/// The operator releases a dirty workspace: the retained act, end to end.
#[tokio::test]
async fn native_console_clear_dirty_releases_the_dirty_workspace() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let cookie = owner(&service).await;
    // The gate holds the workspace dirty (the state a lost dispatch leaves).
    let sql = rusqlite::Connection::open(f.root.path().join("state/domain.sqlite3")).unwrap();
    sql.execute(
        "UPDATE workspace_resources SET dirty=1, dirty_generation=dirty_generation+1 WHERE id='private_workspace'",
        [],
    )
    .unwrap();
    drop(sql);
    assert!(dirty(&f, "private_workspace"), "precondition: dirty");

    let mut released = post("/console/api/resources/private_workspace/clear-dirty", &cookie)
        .send(&service)
        .await;
    assert_eq!(released.status_code, Some(StatusCode::OK));
    let body: Value = serde_json::from_str(&released.take_string().await.unwrap()).unwrap();
    assert_eq!(
        body,
        json!({"ok": true, "result": {"ok": true}}),
        "the retained clear-dirty reply (backend-v2.js:8899-8901)"
    );
    assert!(!dirty(&f, "private_workspace"), "the workspace is released");
}

/// A quarantined workspace is NOT releasable here: the retained product
/// requires the stopped-dispatch inspection flow and answers
/// `inspection_required` (backend-v2.js:1929).
#[tokio::test]
async fn native_console_clear_dirty_refuses_a_quarantined_workspace() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let cookie = owner(&service).await;
    let sql = rusqlite::Connection::open(f.root.path().join("state/domain.sqlite3")).unwrap();
    // The fixture's live dispatch settled unknown and is still unresolved.
    sql.execute(
        "UPDATE runner_dispatches SET state='outcome_unknown' WHERE id='private_dispatch'",
        [],
    )
    .unwrap();
    sql.execute(
        "UPDATE workspace_resources SET dirty=1 WHERE id='private_workspace'",
        [],
    )
    .unwrap();
    drop(sql);

    let mut refused = post("/console/api/resources/private_workspace/clear-dirty", &cookie)
        .send(&service)
        .await;
    assert_eq!(refused.status_code, Some(StatusCode::CONFLICT));
    let body: Value = serde_json::from_str(&refused.take_string().await.unwrap()).unwrap();
    assert_eq!(body["code"], "inspection_required");
    assert!(dirty(&f, "private_workspace"), "the quarantine is untouched");
}

/// A non-lifecycle session cannot reach either surface.
#[tokio::test]
async fn native_console_exec_policy_requires_the_lifecycle_scope() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let read_only = session(&service).await;
    for (method, path) in [
        ("get", format!("/console/api/agents/{}/execution-policy", f.engagement)),
        (
            "post",
            "/console/api/resources/private_workspace/clear-dirty".to_owned(),
        ),
    ] {
        let mut response = if method == "get" {
            get(&path, &read_only).send(&service).await
        } else {
            post(&path, &read_only).send(&service).await
        };
        assert_eq!(
            response.status_code,
            Some(StatusCode::FORBIDDEN),
            "{method} {path}"
        );
        let body: Value =
            serde_json::from_str(&response.take_string().await.unwrap_or_default()).unwrap();
        assert_eq!(body["code"], "agent_lifecycle_scope_required");
    }
}

/// The operator edits an agent's execution policy and reads it back — the
/// engagement's framework is Codex, so `yolo: true` is admissible
/// (`normalizeExecutionPolicy`, lib/execution-authorization.js:15-23).
#[tokio::test]
async fn native_console_execution_policy_round_trips() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let cookie = owner(&service).await;
    let path = format!("/console/api/agents/{}/execution-policy", f.engagement);

    // The default is TS's `{ yolo: false }` for an agent with no stored policy.
    let mut empty = get(&path, &cookie).send(&service).await;
    assert_eq!(empty.status_code, Some(StatusCode::OK));
    let body: Value = serde_json::from_str(&empty.take_string().await.unwrap()).unwrap();
    assert_eq!(body["executionPolicy"]["yolo"], false);
    assert_eq!(body["appliesTo"], "next_dispatch");

    let mut saved = TestClient::put(format!("{BASE}{path}"))
        .add_header("host", "127.0.0.1:13300", true)
        .add_header("origin", BASE, true)
        .add_header("sec-fetch-site", "same-origin", true)
        .add_header("cookie", &cookie, true)
        .json(&json!({"executionPolicy": {"yolo": true}}))
        .send(&service)
        .await;
    assert_eq!(saved.status_code, Some(StatusCode::OK));
    let body: Value = serde_json::from_str(&saved.take_string().await.unwrap()).unwrap();
    assert_eq!(
        body,
        json!({"ok": true, "executionPolicy": {"yolo": true}, "appliesTo": "next_dispatch"}),
        "the retained PUT reply (backend-v2.js:10884)"
    );

    let mut stored = get(&path, &cookie).send(&service).await;
    let body: Value = serde_json::from_str(&stored.take_string().await.unwrap()).unwrap();
    assert_eq!(body["executionPolicy"]["yolo"], true, "the edit persisted");
}

/// A malformed policy is a 400 (`normalizeExecutionPolicy` throws
/// `bad_request`, backend-v2.js:10875-10876), and an unknown agent is a 404.
#[tokio::test]
async fn native_console_execution_policy_refuses_bad_values_and_unknown_agents() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let cookie = owner(&service).await;

    let bad = TestClient::put(format!(
        "{BASE}/console/api/agents/{}/execution-policy",
        f.engagement
    ))
    .add_header("host", "127.0.0.1:13300", true)
    .add_header("origin", BASE, true)
    .add_header("sec-fetch-site", "same-origin", true)
    .add_header("cookie", &cookie, true)
    .json(&json!({"executionPolicy": {"yolo": "yes"}}))
    .send(&service)
    .await;
    assert_eq!(bad.status_code, Some(StatusCode::BAD_REQUEST));

    let missing = get("/console/api/agents/no_such_agent/execution-policy", &cookie)
        .send(&service)
        .await;
    assert_eq!(missing.status_code, Some(StatusCode::NOT_FOUND));
}
