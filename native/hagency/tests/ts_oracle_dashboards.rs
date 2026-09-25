//! TS-oracle tests for `tests/dashboard-native-usage.test.js` and
//! `tests/dashboard-usage-consistency.test.js` (board #63).
//!
//! These two retained files render `mockup/app/usage/page.jsx` in jsdom and
//! assert the EVENTUAL HTML. Only part of what they pin is a Rust surface: the
//! `validateReport` guard they exercise is the client's mirror of the server's
//! wire contract, and THAT is what native serves
//! (`/api/native/v1/engagements/:id/usage`). The render assertions themselves
//! have no Rust counterpart — the page is the shared mockup JS, not a Rust
//! surface — and are listed as skipped in the report.
//!
//! The `validateReport` cases, in Rust terms: exactly six keys (so a stray
//! `billingVerified` cannot ride along), `engagement_id` identity with the
//! request, and counts that are integers or null — never `NaN`, which is
//! unrepresentable in the native typed counters.
#[path = "usage/fixture.rs"]
mod fixture;
use fixture::*;
use salvo::{
    prelude::*,
    test::{ResponseExt, TestClient},
};
use serde_json::{Value, json};

const REPORT_KEYS: [&str; 6] = [
    "at_ms",
    "ceiling",
    "daily",
    "engagement_id",
    "monthly",
    "summary",
];

async fn read(f: &Fixture, query: &str) -> (StatusCode, Value) {
    let mut response = TestClient::get(format!("{}{query}", f.url()))
        .add_header("host", "127.0.0.1:13300", true)
        .bearer_auth(TOKEN)
        .send(&f.service)
        .await;
    let status = response.status_code.unwrap();
    let value = response.take_json::<Value>().await.unwrap();
    (status, value)
}

/// TS: `native observations preserve nulls lower bounds and evidence`
/// (`tests/dashboard-native-usage.test.js`) — the report the page renders is
/// exactly six keys, its `engagement_id` is the one that was read, and the
/// counters are integers or null.
#[tokio::test]
async fn ts_oracle_dashboard_report_is_the_exact_client_contract() {
    let high = snapshot(7, 2, 3);
    let f = Fixture::new(&[(high.as_str(), 2000)], true, true);
    let (status, value) = read(&f, "?at_ms=2000").await;
    assert_eq!(status, StatusCode::OK);
    // Exactly the six keys the validator allow-lists: a seventh (the retained
    // `billingVerified` trap) makes the client throw, so the server must never
    // emit one.
    let keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, REPORT_KEYS);
    assert_eq!(
        value["engagement_id"], json!(f.engagement),
        "the report describes the engagement that was read"
    );
    assert_eq!(value["at_ms"], json!(2000));
    // The TS `headroom(v.ceiling)` contract, exactly: the three ceiling keys are
    // present and each is an integer or null — never NaN, and never a key the
    // client would have to guess. `tokens_drawn` is the fixture's reserved
    // commitment (the request asked for 100), not a zero.
    let ceiling = value["ceiling"].as_object().unwrap().clone();
    assert_eq!(
        {
            let mut keys: Vec<&str> = ceiling.keys().map(String::as_str).collect();
            keys.sort_unstable();
            keys
        },
        ["remaining_tokens", "tokens_drawn", "tokens_used"]
    );
    for (field, value) in &ceiling {
        assert!(
            value.is_null() || value.is_u64(),
            "ceiling.{field} is an integer or null, never NaN"
        );
    }
    assert_eq!(ceiling["tokens_drawn"], json!(100), "the reserved commitment");
    for field in [
        "sources",
        "latest_incomplete_sources",
        "historically_incomplete_sources",
        "regression_observations",
    ] {
        assert!(
            value["summary"][field].is_u64(),
            "summary.{field} is an integer"
        );
    }
    assert_eq!(
        value["summary"]["evidence"],
        json!("host_attributed_untrusted_usage"),
        "evidence stays untrusted, never a billing claim"
    );
    f.close().await;
}

/// TS: `unknown native state cannot become a fixture or zero response`
/// (`tests/dashboard-native-usage.test.js`) — `selection({search:'?data=fixture'})`
/// throws, duplicate selections throw, and a report with nothing measured
/// reports `null`, not zero.
#[tokio::test]
async fn ts_oracle_dashboard_unknown_state_is_refused_not_defaulted() {
    let f = Fixture::new(&[], false, true);
    // A fixture-style parameter is refused, not answered.
    let (status, _) = read(&f, "?data=fixture").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // A duplicated selection is refused, not silently resolved.
    let (status, _) = read(&f, "?at_ms=2&at_ms=2").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Nothing measured → `latest_counts` is null, never a zero row.
    let (status, value) = read(&f, "?at_ms=2000").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["summary"]["sources"], json!(0));
    assert_eq!(
        value["summary"]["latest_counts"],
        json!(null),
        "nothing measured is null, not a zero response"
    );
    assert_eq!(value["summary"]["known_high_water_lower_bound"], json!(null));
    assert!(value["daily"].is_null() && value["monthly"].is_null());
    f.close().await;
}

// Skipped (no Rust counterpart), listed in `.peer/report-63.md`:
//
//   * `tests/dashboard-native-usage.test.js` — the two `renderDashboard(...)`
//     assertions (`Historical high-water lower bounds`, `No observation for
//     this period`, `Console access required`, and the absence of `NaN` in the
//     HTML). These render `mockup/app/usage/page.jsx` itself; the page is the
//     shared mockup JS in both worlds and the native console serves it as a
//     static document, so there is no Rust surface to assert against.
//
//   * `tests/dashboard-usage-consistency.test.js` — all 6 cases (`renders live
//     agent task counts in the chart and summary`, `does not duplicate live
//     tasks across projects or legacy usage rows`, `explains missing project
//     attribution without inventing rows`, `excludes ended and pending
//     engagements from the allocation donut`, `renders no allocation when only
//     ended engagements remain`, `renders zero task counts for an observed agent
//     without activity`) render the same page and assert its HTML. Same reason.
