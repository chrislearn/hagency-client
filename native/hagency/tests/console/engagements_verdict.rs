use super::*;

// The console verdict slice (board #16, parity backend-v2.js:15154-15221):
// the operator approves or refuses a pending engagement from the console.
// Approve reaches the SAME store verdict the Matrix intake reaches
// (`DomainStore::approve`: pending → reserved + a pending `provision_{id}`
// effect — provisioning enqueued); the acceptance scenario drives the HTTP
// route and asserts exactly that store state. Candidates is the retained
// GET read; refuse already exists (`POST /agents/{id}/refuse`) and gets one
// end-to-end pass here from the engagements surface's point of view.

/// The seeded store's effects rows, read directly.
fn effects(state: &std::path::Path) -> Vec<(String, String, String)> {
    let db = rusqlite::Connection::open(state.join("domain.sqlite3")).unwrap();
    let mut stmt = db
        .prepare("SELECT engagement_id,kind,state FROM effects ORDER BY rowid")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Scenario (acceptance): the operator approves a pending engagement in the
/// console and the same effect happens as the Matrix approval — the store
/// state becomes reserved and provisioning is enqueued.
#[tokio::test]
async fn native_engagement_verdict_approve_reserves_and_enqueues_provision() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let state = f.root.path().join("state");
    let pending = f.new_engagement().await;

    // Candidates before the decision: one project-definition candidate,
    // unlocked, naming the stored resource.
    let read_only = session(&service).await;
    let mut listed = get(
        &format!("/console/api/engagements/{pending}/candidates"),
        &read_only,
    )
    .send(&service)
    .await;
    assert_eq!(listed.status_code, Some(StatusCode::OK));
    let body = listed.take_json::<Value>().await.unwrap();
    assert_eq!(body["locked"], false);
    assert_eq!(body["allocation"], json!({"kind": "project-definition"}));
    let candidate = body["candidates"].as_array().unwrap().first().cloned().unwrap();
    assert_eq!(candidate["choice"], json!({"kind": "project-definition"}));
    assert_eq!(candidate["name"], "NewUsageWorker");
    assert_eq!(candidate["resource"], "private_usage_pool");
    assert_eq!(candidate["provision"], true);

    // TS parity (#31): there is no read-only login — one login is the whole
    // console. An anonymous caller is refused before any store job; every
    // logged-in session may decide. The anonymous caller needs no ticket at
    // all (the console's authenticate hoop rejects it without a cookie).
    let anonymous = TestClient::post(format!(
        "{BASE}/console/api/engagements/{pending}/approve"
    ))
    .add_header("host", "127.0.0.1:13300", true)
    .add_header("origin", BASE, true)
    .add_header("sec-fetch-site", "same-origin", true)
    .json(&json!({"commandId": "cmd_verdict_1"}))
    .send(&service)
    .await;
    assert_eq!(anonymous.status_code, Some(StatusCode::UNAUTHORIZED));

    // Ticket issuance is rate-limited to one per second.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let cookie = lifecycle_session(&service).await;
    let mut response = post(
        &format!("/console/api/engagements/{pending}/approve"),
        &cookie,
    )
    .json(&json!({"commandId": "cmd_verdict_1"}))
    .send(&service)
    .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let body = response.take_json::<Value>().await.unwrap();
    assert_eq!(body["id"], pending);
    assert_eq!(body["state"], "reserved");
    assert_eq!(body["cleanup"], "not_required");
    assert_eq!(body.as_object().unwrap().len(), 3);

    // The store effect: the provision row for THIS engagement is enqueued.
    let rows = effects(&state);
    let provision = rows
        .iter()
        .find(|r| r.0 == pending && r.1 == "provision")
        .expect("provision effect enqueued");
    assert_eq!(provision.2, "pending");

    // Candidates after the decision: locked, the retained
    // `e.state !== 'pending'` word.
    let mut listed = get(
        &format!("/console/api/engagements/{pending}/candidates"),
        &read_only,
    )
    .send(&service)
    .await;
    assert_eq!(listed.status_code, Some(StatusCode::OK));
    let body = listed.take_json::<Value>().await.unwrap();
    assert_eq!(body["locked"], true);

    // A second approve with a new command id is the pending-only guard
    // (domain.rs:1212-1214): conflict, never a second reservation.
    let again = post(
        &format!("/console/api/engagements/{pending}/approve"),
        &cookie,
    )
    .json(&json!({"commandId": "cmd_verdict_2"}))
    .send(&service)
    .await;
    assert_eq!(again.status_code, Some(StatusCode::CONFLICT));
    f.close().await;
}

/// Scenario: the operator refuses a pending engagement from the console; the
/// existing refuse route ends it rejected with no provision work.
#[tokio::test]
async fn native_engagement_verdict_refuse_rejects_pending() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let state = f.root.path().join("state");
    let pending = f.new_engagement().await;
    let cookie = lifecycle_session(&service).await;
    let mut response = post(&format!("/console/api/agents/{pending}/refuse"), &cookie)
        .json(&json!({"commandId": "cmd_refuse_1"}))
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let body = response.take_json::<Value>().await.unwrap();
    assert_eq!(body["engagement"]["id"], pending);
    assert_eq!(body["engagement"]["state"], "rejected");
    // No provision row was ever enqueued for the refused request.
    let rows = effects(&state);
    assert!(rows.iter().all(|r| r.0 != pending));
    f.close().await;
}

/// Scenario: an unknown engagement answers 404, the retained
/// `'Engagement not found'` word.
#[tokio::test]
async fn native_engagement_verdict_unknown_engagement_is_not_found() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    // Ticket issuance is rate-limited to one per second (shared across the
    // concurrent console tests); one ticket here, reused for the read.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let cookie = lifecycle_session(&service).await;
    let response = post("/console/api/engagements/en_does_not_exist/approve", &cookie)
        .json(&json!({"commandId": "cmd_missing"}))
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::NOT_FOUND));
    // Reads stay scope-free: the same session may read candidates.
    let response = get(
        "/console/api/engagements/en_does_not_exist/candidates",
        &cookie,
    )
    .send(&service)
    .await;
    assert_eq!(response.status_code, Some(StatusCode::NOT_FOUND));
    f.close().await;
}
