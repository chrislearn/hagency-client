//! #26 acceptance: the SSE route emits an event when console-visible state
//! changes. TS parity: lib/backend/sse-adapter.js:20 (`text/event-stream`,
//! `:\n\n` on connect, `event:`/`data:` frames) and the retained dashboard's
//! consumption (event says "changed"; the page refetches its own read).
//!
//! The stream is unbounded, so this test drives a REAL socket server (the
//! warm_runtime.rs:155 pattern) and reads the wire with a deadline; the
//! bounded snapshot/events reads use the in-process client like every other
//! console test.
use super::*;
use salvo::Listener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The shared helpers hardcode the 13300 host; this test's server owns a
/// random port, so the session is issued against the REAL authority.
async fn stream_session(service: &Service, host: &str, base: &str) -> String {
    let mut response = TestClient::post(format!("{base}/api/native/v1/console/access"))
        .add_header("host", host, true)
        .bearer_auth(TOKEN)
        .send(service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK), "access issue");
    let ticket = response.take_json::<Value>().await.unwrap()["ticket"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut response = TestClient::post(format!("{base}/console/session"))
        .add_header("host", host, true)
        .add_header("origin", base, true)
        .add_header("sec-fetch-site", "same-origin", true)
        .json(&json!({"ticket": ticket}))
        .send(service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK), "session exchange");
    let cookie = response.headers().get("set-cookie").unwrap().to_str().unwrap();
    cookie.split(';').next().unwrap().to_owned()
}

async fn read_until(
    sock: &mut tokio::net::TcpStream,
    seen: &mut String,
    needle: &str,
    what: &str,
) {
    let mut buf = [0u8; 4096];
    let got = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let n = sock.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            seen.push_str(&String::from_utf8_lossy(&buf[..n]));
            if seen.contains(needle) {
                break;
            }
        }
    })
    .await;
    assert!(got.is_ok(), "timed out waiting for {what}; wire so far:\n{seen}");
}

#[tokio::test]
async fn native_console_stream_emits_on_state_change() {
    let acceptor = salvo::conn::TcpListener::new("127.0.0.1:0")
        .try_bind()
        .await
        .unwrap();
    let address = acceptor.local_addr().unwrap();
    let f = Fixture::new(address, None);
    let service = f.service();
    let host = address.to_string();
    let base = format!("http://{host}");
    let cookie = stream_session(&service, &host, &base).await;
    let server = salvo::Server::new(acceptor);
    let handle = server.handle();
    let app = f.app.clone();
    let serve = tokio::spawn(async move {
        server.try_serve(app.router()).await.unwrap();
    });

    let mut sock = tokio::net::TcpStream::connect(address).await.unwrap();
    let host = address.to_string();
    let request = format!(
        "GET /console/api/stream HTTP/1.1\r\nhost: {host}\r\norigin: http://{host}\r\n\
         sec-fetch-site: same-origin\r\ncookie: {cookie}\r\naccept: text/event-stream\r\n\r\n"
    );
    sock.write_all(request.as_bytes()).await.unwrap();
    let mut wire = String::new();
    read_until(&mut sock, &mut wire, "event: hello", "the hello frame").await;
    assert!(
        wire.contains("text/event-stream"),
        "the wire carries the SSE content type:\n{wire}"
    );
    // Chunked framing inserts length prefixes between frames; the SSE
    // contract is "the comment heartbeat is written on connect", so the
    // assertion looks for the comment itself, not the raw byte adjacency.
    assert!(
        wire.contains(":\n\n"),
        "the comment heartbeat is written on connect (sse-adapter installRoute):\n{wire}"
    );

    // State change: a new canonical task row moves the tasks fingerprint.
    f.domain
        .create_canonical_task(
            "stream_live_task".into(),
            "private_session".into(),
            "Live update".into(),
            now(),
        )
        .await
        .unwrap();
    read_until(&mut sock, &mut wire, "event: tasks", "the tasks change event").await;
    let frame = wire
        .split("event: tasks")
        .last()
        .unwrap()
        .split("\n\n")
        .next()
        .unwrap();
    assert!(
        frame.contains("\"feed_version\""),
        "the frame carries the new cursor:\n{frame}"
    );
    drop(sock);
    handle.stop_graceful(Some(std::time::Duration::from_secs(1)));
    let _ = serve.await;
    f.close().await;
}

#[tokio::test]
async fn native_console_stream_snapshot_and_events() {
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let service = f.service();
    let cookie = session(&service).await;
    // Snapshot: the bounded one-shot read (routerStore.snapshot parity).
    let mut response = get("/console/api/stream/snapshot", &cookie)
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let body = response.take_json::<Value>().await.unwrap();
    assert!(body["version"].is_string(), "the whole-feed cursor exists");
    for key in ["agents", "tasks", "alerts"] {
        assert!(
            body[key]["version"].is_string() && body[key]["count"].is_u64(),
            "the {key} fingerprint exists"
        );
    }
    let version = body["version"].as_str().unwrap().to_owned();
    // Events with a stale cursor: every category reports its change.
    let mut response = get("/console/api/stream/events?after=stale", &cookie)
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let body = response.take_json::<Value>().await.unwrap();
    let events = body["events"].as_array().unwrap();
    let kinds: Vec<&str> = events
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    for kind in ["agents_changed", "tasks_changed", "alerts_changed"] {
        assert!(kinds.contains(&kind), "kinds: {kinds:?}");
    }
    assert_eq!(body["gap"], false, "no gap on a fresh store");
    // The current cursor: nothing changed.
    let mut response = get(&format!("/console/api/stream/events?after={version}"), &cookie)
        .send(&service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let body = response.take_json::<Value>().await.unwrap();
    assert_eq!(
        body["events"].as_array().map(Vec::len),
        Some(0),
        "an unchanged cursor reports no events"
    );
    f.close().await;
}
