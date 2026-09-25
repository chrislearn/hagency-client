//! Console live stream (#26): `GET /console/api/stream` — TS parity with the
//! retained `/api/stream` (lib/backend/sse-adapter.js:20 installRoute): a
//! `text/event-stream` response that writes the `:\n\n` comment heartbeat on
//! connect, broadcasts `event: <name>\ndata: <JSON>\n\n` frames, and keeps the
//! client alive with `: ` comments every 30 s (startKeepalive, default 30000).
//!
//! The retained backend had ONE process and broadcast in-memory at each write
//! site (`broadcastSSE('task_updated', task)`, backend-v2.js:8577 et al.).
//! Native serves from a shared DomainStore whose writers are separate tasks,
//! so the route polls the store's bounded change feed (`console_feed`) — the
//! same freshness question `routerStore.snapshot()` answered (:3655) — and
//! emits a category event when a fingerprint changes. The page then refetches
//! its own bounded read, exactly the division the retained dashboard used.
//!
//! The connection holds NO console permit for its lifetime: the
//! `browser_boundary` semaphore (8) is released before streaming starts, so
//! one live tab cannot starve the console's bounded request budget.
use super::{Error, Session, failed, usage::query};
use crate::resources::domain;
use crate::refusal;
use salvo::prelude::*;
use serde_json::json;

pub(super) fn router() -> Router {
    Router::with_path("stream")
        .push(Router::with_path("snapshot").get(snapshot))
        .push(Router::new().get(stream))
        .push(Router::with_path("events").get(events))
}

/// `GET /console/api/stream` — the SSE wire itself. Snapshot-first so a
/// reconnecting page learns the current fingerprints without a separate
/// round trip; then change events; then keepalive comments.
#[handler]
async fn stream(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    // The connection is long-lived; a permit would be held forever. The
    // boundary already authenticated the browser; this handler only reads
    // the store.
    depot.remove("console_permit");
    if query(req, &["after"], 160).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    // A session is still required — the stream is same-origin console data.
    if depot.get_typed::<Session>().is_err() {
        failed(res, Error::Unauthorized);
        return;
    }
    let Some(store) = domain(depot, res) else {
        return;
    };
    let mut last = match req.query::<String>("after") {
        Some(after) if !after.is_empty() => after,
        _ => match store.console_feed().await {
            Ok(feed) => feed["version"].as_str().unwrap_or_default().to_owned(),
            Err(_) => String::new(),
        },
    };
    let headers = [
        ("content-type", "text/event-stream; charset=utf-8"),
        ("cache-control", "no-cache"),
    ];
    for (name, value) in headers {
        if let Ok(parsed) = value.parse() {
            res.headers_mut().insert(name, parsed);
        }
    }
    let sender = res.channel();
    let mut sender = sender;
    let _ = sender.send_data(":\n\n".to_owned()).await;
    let _ = sender
        .send_data(format!("event: hello\ndata: {}\n\n", json!({"version": last})))
        .await;
    // Poll cadence: the console page polled at 15 s; the feed read is one
    // bounded query, so 1 s keeps a live page tight without touching any
    // writer. The first interval tick fires immediately — the initial
    // fingerprint is emitted without waiting a second.
    let mut poll = tokio::time::interval(std::time::Duration::from_secs(1));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut keepalive = tokio::time::interval(std::time::Duration::from_secs(30));
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = keepalive.tick() => {
                if sender.send_data(":\n\n".to_owned()).await.is_err() {
                    return;
                }
            }
            _ = poll.tick() => {
                let Ok(feed) = store.console_feed().await else {
                    continue;
                };
                let version = feed["version"].as_str().unwrap_or_default().to_owned();
                if !version.is_empty() && version != last {
                    for category in ["agents", "tasks", "alerts"] {
                        let _ = sender
                            .send_data(format!(
                                "event: {}\ndata: {}\n\n",
                                category,
                                json!({
                                    "version": feed[category]["version"],
                                    "count": feed[category]["count"],
                                    "feed_version": version,
                                })
                            ))
                            .await;
                    }
                    last = version;
                }
            }
        }
    }
}

/// `GET /console/api/stream/snapshot` — the bounded one-shot feed read
/// (router snapshot parity, store.ts:3655): the page's cursor baseline.
#[handler]
async fn snapshot(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if query(req, &[], 0).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    if depot.get_typed::<Session>().is_err() {
        failed(res, Error::Unauthorized);
        return;
    }
    let Some(store) = domain(depot, res) else {
        return;
    };
    match store.console_feed().await {
        Ok(feed) => res.render(Json(feed)),
        Err(error) => failure(res, error),
    }
}

/// `GET /console/api/stream/events?after=` — router events parity
/// (store.ts:3811 `eventsAfter`): the categories whose version changed
/// since the cursor, plus the new cursor. Bounded to one page.
#[handler]
async fn events(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let prepared = async {
        query(req, &["after"], 160)?;
        let after = req.query::<String>("after").unwrap_or_default();
        if !after.is_empty()
            && !after
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(Error::Invalid);
        }
        Ok::<_, Error>(after)
    }
    .await;
    let after = match prepared {
        Ok(value) => value,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let current = match store.console_feed().await {
        Ok(feed) => feed,
        Err(error) => {
            failure(res, error);
            return;
        }
    };
    let version = current["version"].as_str().unwrap_or_default().to_owned();
    let mut changed: Vec<&str> = Vec::new();
    if after.is_empty() || after != version {
        // One page: every category that differs from the cursor's feed. The
        // cursor is the whole-feed version, so a mismatch means at least one
        // category changed; report all three with their own versions so the
        // page can diff per-category.
        for category in ["agents", "tasks", "alerts"] {
            changed.push(category);
        }
    }
    res.render(Json(json!({
        "low_watermark": 0,
        "high_watermark": version,
        "gap": false,
        "events": changed
            .into_iter()
            .map(|kind| json!({
                "kind": format!("{kind}_changed"),
                "version": current[kind]["version"],
                "count": current[kind]["count"],
            }))
            .collect::<Vec<_>>(),
    })));
}

fn failure(res: &mut Response, error: hagency_store::Error) {
    let (status, code) = match error {
        hagency_store::Error::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_stream_query"),
        hagency_store::Error::Busy => (StatusCode::SERVICE_UNAVAILABLE, "busy"),
        hagency_store::Error::OutcomeUnknown => {
            (StatusCode::GATEWAY_TIMEOUT, "outcome_unknown")
        }
        _ => (StatusCode::SERVICE_UNAVAILABLE, "stream_unavailable"),
    };
    refusal(res, status, code);
}
