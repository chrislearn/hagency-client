use super::*;

// ADR-186 on the console routes: the approval chooses its amount (§A), the
// quota pause shows on the reads (§B), and a top-up raises a running
// engagement's allocation and lifts the pause (§C).

fn allocated_column(state: &std::path::Path, id: &str) -> Option<u64> {
    rusqlite::Connection::open(state.join("domain.sqlite3"))
        .unwrap()
        .query_row(
            "SELECT allocated_tokens FROM engagements WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
}

/// §A1: approving with a lower amount reserves exactly that amount.
#[tokio::test]
async fn native_allocation_route_approves_a_lower_amount() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let state = f.root.path().join("state");
    let pending = f.new_engagement_requesting("allocation_lower", 300).await;
    let cookie = lifecycle_session(&service).await;
    // A zero amount and an unknown key are invalid bodies, decided before
    // any store job.
    for body in [
        json!({"commandId": "cmd_lower_zero", "allocatedTokens": 0}),
        json!({"commandId": "cmd_lower_extra", "allocatedTokens": 10, "extra": 1}),
        json!({"commandId": "cmd_lower_negative", "allocatedTokens": -5}),
    ] {
        let refused = post(&format!("/console/api/engagements/{pending}/approve"), &cookie)
            .json(&body)
            .send(&service)
            .await;
        assert_eq!(refused.status_code, Some(StatusCode::BAD_REQUEST), "{body}");
    }
    let mut response = post(&format!("/console/api/engagements/{pending}/approve"), &cookie)
        .json(&json!({"commandId": "cmd_lower", "allocatedTokens": 40}))
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let body = response.take_json::<Value>().await.unwrap();
    assert_eq!(body["state"], "reserved");
    assert_eq!(body.as_object().unwrap().len(), 3, "the bounded receipt");
    assert_eq!(allocated_column(&state, &pending), Some(40));
    f.close().await;
}

/// §A2/§A3: the candidate read's `remainingTokens` is the approval's own
/// headroom: approving exactly that ("All remaining") succeeds, one token
/// more is refused with `over_commit` and the human message.
#[tokio::test]
async fn native_allocation_route_all_remaining_and_named_refusal() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let state = f.root.path().join("state");
    let pending = f.new_engagement_requesting("allocation_all", 100).await;
    let cookie = lifecycle_session(&service).await;
    let mut listed = get(&format!("/console/api/engagements/{pending}/candidates"), &cookie)
        .send(&service)
        .await;
    assert_eq!(listed.status_code, Some(StatusCode::OK));
    let body = listed.take_json::<Value>().await.unwrap();
    let remaining = body["candidates"][0]["remainingTokens"].as_u64().unwrap();
    // The 1000-token pool holds the seeded UsageWorker's 100.
    assert_eq!(remaining, 900);
    let mut over = post(&format!("/console/api/engagements/{pending}/approve"), &cookie)
        .json(&json!({"commandId": "cmd_all_over", "allocatedTokens": remaining + 1}))
        .send(&service)
        .await;
    assert_eq!(over.status_code, Some(StatusCode::CONFLICT));
    let refusal = over.take_json::<Value>().await.unwrap();
    assert_eq!(refusal["code"], "over_commit");
    let message = refusal["message"].as_str().expect("the human message travels");
    assert!(message.contains("would exceed"), "{message}");
    assert!(message.contains("NewUsageWorker"), "{message}");
    let ok = post(&format!("/console/api/engagements/{pending}/approve"), &cookie)
        .json(&json!({"commandId": "cmd_all", "allocatedTokens": remaining}))
        .send(&service)
        .await;
    assert_eq!(ok.status_code, Some(StatusCode::OK));
    assert_eq!(allocated_column(&state, &pending), Some(900));
    f.close().await;
}
