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

/// §B: a quota hold shows on Engagements (`quotaPaused`, with the allocation
/// and the known spend) and on Workforce (`quota_paused`). The hold row is
/// written directly: the store tests own how it opens, this pins the reads.
#[tokio::test]
async fn native_allocation_route_reads_show_the_quota_pause() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let state = f.root.path().join("state");
    let cookie = lifecycle_session(&service).await;
    let paused = |value: &Value| -> Option<bool> {
        value["engagements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == f.engagement.as_str())
            .map(|row| row["quotaPaused"].as_bool().unwrap())
    };
    let mut listed = get("/console/api/engagements", &cookie).send(&service).await;
    let before = listed.take_json::<Value>().await.unwrap();
    assert_eq!(paused(&before), Some(false));
    let row = before["engagements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == f.engagement.as_str())
        .unwrap()
        .clone();
    assert_eq!(row["allocatedTokens"], 100, "the request is the allocation");
    // The seed's period is incomplete (an empty snapshot between two
    // counts) but carries counts: its known lower bound is the spend.
    assert!(row["spentTokens"].as_u64().is_some(), "the seeded usage is counted");
    rusqlite::Connection::open(state.join("domain.sqlite3"))
        .unwrap()
        .execute(
            "INSERT INTO quota_holds(engagement_id,dispatch_id,spend,allocation,began_at) VALUES(?1,NULL,100,100,1)",
            [&f.engagement],
        )
        .unwrap();
    let mut listed = get("/console/api/engagements", &cookie).send(&service).await;
    assert_eq!(paused(&listed.take_json::<Value>().await.unwrap()), Some(true));
    let mut roster = get("/console/api/agents", &cookie).send(&service).await;
    assert_eq!(roster.status_code, Some(StatusCode::OK));
    let roster = roster.take_json::<Value>().await.unwrap();
    let agent = roster["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["engagement_id"] == f.engagement.as_str())
        .unwrap()
        .clone();
    assert_eq!(agent["quota_paused"], true);
    f.close().await;
}
