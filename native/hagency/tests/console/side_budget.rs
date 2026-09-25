use super::*;

/// `PUT /api/project-sides/:id/allocation` and `GET …/budget`
/// (backend-v2.js:9541, :9567) on the console wire: the write behind the
/// configuration scope, the read beside the scope-free project-sides list,
/// and the fleet totals block (backend-v2.js:15700-15720) that names its
/// own denominator. Every expected figure is derived from the shared seed:
/// two approved (reserved) engagements of 100 tokens each commit 200, the
/// third admitted-but-pending one commits nothing, and exactly one of the
/// three agents carries observed usage.

/// The configuration-scoped session the resource writes use (brief 28's
/// one-concept-one-scope: the allocation is an operator budget decision).
/// Ticket issuance is rate-limited to one per second (authority.rs
/// `issue_scope`), so the second issuance waits the 1010ms the
/// configuration tests sleep between scopes.
async fn configuration(service: &Service) -> String {
    tokio::time::sleep(std::time::Duration::from_millis(1010)).await;
    let mut response = TestClient::post(format!(
        "{BASE}/api/native/v1/console/resource-configuration-access"
    ))
    .add_header("host", "127.0.0.1:13300", true)
    .bearer_auth(TOKEN)
    .send(service)
    .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let ticket = response.take_json::<Value>().await.unwrap()["ticket"]
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

fn put(path: &str, cookie: &str) -> salvo::test::RequestBuilder {
    TestClient::put(format!("{BASE}{path}"))
        .add_header("host", "127.0.0.1:13300", true)
        .add_header("origin", BASE, true)
        .add_header("sec-fetch-site", "same-origin", true)
        .add_header("cookie", cookie, true)
}

/// The allocation write is a configuration decision: the plain read-only
/// console session is refused, the configuration-scoped one sets and
/// clears, and the reply carries the SAME six-key side record the list
/// serves beside the budget (backend-v2.js:9545 `{ok, side, budget}`).
#[tokio::test]
async fn native_console_side_allocation_write_and_read() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let readonly = session(&service).await;
    let body = json!({"allocated_tokens": 500});
    assert_eq!(
        put("/console/api/project-sides/example.test/allocation", &readonly)
            .json(&body)
            .send(&service)
            .await
            .status_code,
        Some(StatusCode::FORBIDDEN),
        "the write takes the configuration scope"
    );
    let operator = configuration(&service).await;
    let mut response = put("/console/api/project-sides/example.test/allocation", &operator)
        .json(&body)
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let value: Value = response.take_json().await.unwrap();
    assert_eq!(
        value.as_object().unwrap().len(),
        3,
        "exactly ok, side, budget"
    );
    assert_eq!(value["ok"], true);
    let side = &value["side"];
    assert_eq!(side["id"], "example.test", "the id IS the server name");
    assert_eq!(
        side.as_object().unwrap().len(),
        6,
        "the same six-key projection the list serves"
    );
    let budget = &value["budget"];
    assert_eq!(budget["allocated"], 500);
    assert_eq!(budget["committed"], 200, "two reserved engagements of 100");
    assert_eq!(budget["remaining"], 300);
    assert_eq!(budget["commitments"].as_array().unwrap().len(), 2);
    assert_eq!(budget["totalCommitted"], 200);
    assert_eq!(budget["poolCommitted"], 0);
    assert_eq!(budget["orphanedCommitted"], 0);

    // The camelCase spelling is accepted exactly like the retained
    // `req.body?.allocated_tokens ?? req.body?.allocatedTokens`
    // (backend-v2.js:9543).
    let mut response = put("/console/api/project-sides/example.test/allocation", &operator)
        .json(&json!({"allocatedTokens": 50}))
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let value: Value = response.take_json().await.unwrap();
    assert_eq!(value["budget"]["allocated"], 50);
    assert_eq!(value["budget"]["remaining"], 0, "saturates, never negative");

    // The read spreads the budget FLAT beside sideId — the retained route's
    // own spread (backend-v2.js:9571) — with no nested budget key.
    let mut response = get("/console/api/project-sides/example.test/budget", &readonly)
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let value: Value = response.take_json().await.unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["sideId"], "example.test");
    assert!(value.get("budget").is_none(), "the budget travels flat");
    assert_eq!(value["allocated"], 50);
    assert_eq!(value["committed"], 200);
    assert_eq!(value["remaining"], 0);
    assert_eq!(value["commitments"].as_array().unwrap().len(), 2);

    // The clear: NULL is unallocated — not unlimited — and remaining goes
    // null with it (lib/project-side-store.js:443-452).
    let mut response = put("/console/api/project-sides/example.test/allocation", &operator)
        .json(&json!({}))
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let value: Value = response.take_json().await.unwrap();
    assert_eq!(value["budget"]["allocated"], Value::Null);
    assert_eq!(value["budget"]["remaining"], Value::Null);

    // The store owns the not-found verdict (backend-v2.js:9544, :9568).
    assert_eq!(
        put("/console/api/project-sides/nope.test/allocation", &operator)
            .json(&body)
            .send(&service)
            .await
            .status_code,
        Some(StatusCode::NOT_FOUND)
    );
    assert_eq!(
        get("/console/api/project-sides/nope.test/budget", &readonly)
            .send(&service)
            .await
            .status_code,
        Some(StatusCode::NOT_FOUND)
    );
    // A list observation takes no selection: the budget reads likewise
    // refuse every query parameter.
    assert_eq!(
        get("/console/api/project-sides/example.test/budget?limit=1", &readonly)
            .send(&service)
            .await
            .status_code,
        Some(StatusCode::BAD_REQUEST)
    );
}

/// The fleet totals block (backend-v2.js:15700-15720): the seed's three
/// agents, exactly one measured, with the busy-time and task columns named
/// as unavailable rather than invented as zero.
#[tokio::test]
async fn native_console_usage_totals() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let cookie = session(&service).await;
    let mut response = get("/console/api/usage/totals", &cookie)
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let value: Value = response.take_json().await.unwrap();
    assert_eq!(value["ok"], true);
    let totals = &value["totals"];
    assert_eq!(totals["agents"], 3, "UsageWorker, AlertWorker, PageWorker");
    assert_eq!(totals["tokensMeasuredFor"], 1, "only UsageWorker is observed");
    assert_eq!(totals["tokensPartial"], true);
    assert!(
        totals["tokensDrawn"].is_u64() && totals["tokensDrawn"].as_u64().unwrap() > 0,
        "the measured figure is served, not dropped to null"
    );
    assert!(
        totals["tokensUsed"].as_u64().unwrap() >= totals["tokensDrawn"].as_u64().unwrap(),
        "display volume adds the cache reads the ceiling figure omits"
    );
    assert_eq!(
        value["unavailable"],
        json!(["busy_sec", "tasks"]),
        "named gaps, never zero"
    );
}
