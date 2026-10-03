//! #29 acceptance: a snapshot taken while the service RUNS, restored back, and
//! credentials rotated without rebuilding.
//!
//! These drive the real binary (`CARGO_BIN_EXE_hagency`), not the library, so
//! they exercise the operator's actual path: `init` → `serve` → `backup` (with
//! serve live) → `stop` → `restore` → `serve` on the restored directory.
//!
//! The WAL hazard these guard against is the one ADR-135:158-167 and the parity
//! audit (row 29) name: the store runs WAL with checkpoints-on-close disabled,
//! so committed frames legitimately live in the `-wal` and a plain copy loses
//! them. `native_ops_backup_captures_a_live_service` fails if the snapshot is
//! taken by copying files.

use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A free loopback port, released so the child can bind it.
fn free_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    address
}

fn native() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hagency"));
    command.env("PATH", "").stdin(Stdio::null());
    command
}

fn ok(output: std::process::Output, what: &str) -> String {
    assert!(
        output.status.success(),
        "{what} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn init(state: &Path) {
    ok(
        native()
            .args(["init", "--state-dir"])
            .arg(state)
            .output()
            .unwrap(),
        "init",
    );
}

fn launch(state: &Path, address: SocketAddr) -> Running {
    let child = native()
        .args(["serve", "--state-dir"])
        .arg(state)
        .args(["--listen", &address.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut running = Running(child);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = running.0.try_wait().unwrap() {
            let mut error = String::new();
            if let Some(mut stderr) = running.0.stderr.take() {
                stderr.read_to_string(&mut error).unwrap();
            }
            panic!("native service exited before health ({status}): {error}");
        }
        if let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let handshake = write!(
                stream,
                "GET /health HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
            )
            .and_then(|()| {
                let mut response = String::new();
                stream.read_to_string(&mut response).map(|_| response)
            });
            match handshake {
                Ok(response) if response.starts_with("HTTP/1.1 200") => return running,
                Ok(_) => {}
                Err(_) => {}
            }
        }
        assert!(
            Instant::now() < deadline,
            "native service startup timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    token: &str,
    body: &str,
) -> (u16, String) {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap();
    (
        status,
        response.split("\r\n\r\n").nth(1).unwrap().to_owned(),
    )
}

/// One custody record — the durable write whose survival proves the database
/// round-tripped, not just its file name.
fn custody(address: SocketAddr, token: &str) -> (u16, String) {
    request(
        address,
        "POST",
        "/api/native/v1/custody",
        token,
        r#"{"binding":"fixture","generation":1,"id":"ops_round_trip","lane":"work","kind":"request","payload":{"name":"小白"}}"#,
    )
}

fn create_resource(address: SocketAddr, token: &str) -> String {
    let (status, body) = request(
        address,
        "POST",
        "/api/native/v1/resources",
        token,
        r#"{"presetId":"ops_pool","seatId":"ops_seat","framework":"codex","model":"gpt-5.6-sol","reasoning":"medium","ceiling":{"tokens":100}}"#,
    );
    assert_eq!(status, 200, "resource create: {body}");
    body
}

/// A snapshot taken WHILE the service holds the store, restored back, and read
/// by a fresh service. Every step is the operator's real path.
#[test]
fn native_ops_backup_round_trips_while_the_service_runs() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    init(&state);
    let token = fs::read_to_string(state.join("operator.token")).unwrap();

    let address = free_address();
    let running = launch(&state, address);
    // Durable writes into BOTH databases, made while the service is live.
    let (status, receipt) = custody(address, &token);
    assert_eq!(status, 202, "custody: {receipt}");
    let resource = create_resource(address, &token);

    // The snapshot is taken with the service STILL RUNNING.
    let out = root.path().join("snapshot");
    let stdout = ok(
        native()
            .args(["backup", "--state-dir"])
            .arg(&state)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
        "backup",
    );
    let summary: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(summary["ok"], true);
    assert!(summary["files"].as_u64().unwrap() > 0);
    // The service is unharmed by the snapshot: it still answers.
    assert_eq!(custody(address, &token).0, 202);
    drop(running);

    // A naive copy would have lost the WAL frames; the snapshot did not.
    let restored = root.path().join("restored");
    ok(
        native()
            .args(["restore", "--state-dir"])
            .arg(&restored)
            .arg("--from")
            .arg(&out)
            .output()
            .unwrap(),
        "restore",
    );
    // The operator token travelled inside the snapshot.
    assert_eq!(
        fs::read(restored.join("operator.token")).unwrap(),
        fs::read(state.join("operator.token")).unwrap()
    );

    // A fresh service on the restored state serves the same durable facts.
    let address = free_address();
    let _restored_service = launch(&restored, address);
    assert_eq!(
        custody(address, &token),
        (202, receipt),
        "the custody record did not survive the round trip"
    );
    let (status, listed) = request(
        address,
        "GET",
        "/api/native/v1/resources?limit=100",
        &token,
        "",
    );
    assert_eq!(status, 200);
    let expected: serde_json::Value = serde_json::from_str(&resource).unwrap();
    let listed: serde_json::Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(
        listed,
        serde_json::json!([expected]),
        "the restored catalog is not the original row"
    );
}

/// A restore never overwrites live state, and a snapshot never replaces an
/// earlier one.
#[test]
fn native_ops_backup_and_restore_refuse_to_clobber() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    init(&state);
    let out = root.path().join("snapshot");
    ok(
        native()
            .args(["backup", "--state-dir"])
            .arg(&state)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
        "first backup",
    );
    // A second backup at the same path is refused, not silently overwritten.
    let second = native()
        .args(["backup", "--state-dir"])
        .arg(&state)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        !second.status.success(),
        "backup overwrote an existing snapshot"
    );

    // Restoring over a non-empty state is refused: no existing data is replaced.
    let restore = native()
        .args(["restore", "--state-dir"])
        .arg(&state)
        .arg("--from")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        !restore.status.success(),
        "restore overwrote a non-empty state directory"
    );
    // The original state is untouched by the refusal.
    assert!(state.join("operator.token").is_file());
}

/// Rotate the operator token in place. The running service keeps the old token
/// until it restarts — exactly TS's "edit `API_TOKEN` and restart" — and the
/// state directory is NOT rebuilt.
#[test]
fn native_ops_rotate_operator_token_without_rebuilding() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    init(&state);
    let before = fs::read(state.join("operator.token")).unwrap();

    let address = free_address();
    let running = launch(&state, address);
    let old = String::from_utf8(before.clone()).unwrap();
    assert_eq!(custody(address, &old).0, 202);

    // Rotate while the service runs.
    let stdout = ok(
        native()
            .args(["rotate", "--state-dir"])
            .arg(&state)
            .arg("operator-token")
            .output()
            .unwrap(),
        "rotate",
    );
    let receipt: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(receipt["ok"], true);
    assert_eq!(receipt["rotated"], "operator_token");
    assert_eq!(receipt["applies_to"], "restart");
    // The receipt names the new token by digest only, never its bytes.
    let after = fs::read(state.join("operator.token")).unwrap();
    assert_ne!(after, before, "the token did not change");
    assert!(
        !stdout.contains(String::from_utf8(after.clone()).unwrap().as_str()),
        "the rotation echoed the new token"
    );

    // Until restart the running service still accepts the ONE it loaded, which
    // is TS's documented behaviour and why the receipt says `restart`.
    assert_eq!(custody(address, &old).0, 202);
    drop(running);

    // Same state directory, no rebuild: the file is the operator surface.
    let new = String::from_utf8(after).unwrap();
    let address = free_address();
    let _service = launch(&state, address);
    assert_eq!(
        custody(address, &old).0,
        401,
        "the retired token was still accepted after restart"
    );
    assert_eq!(custody(address, &new).0, 202, "the new token was refused");
    // The state is not merely re-readable: it is the SAME state, migrated in
    // place, with the earlier write still present.
    let (status, body) = request(
        address,
        "GET",
        "/api/native/v1/resources?limit=100",
        &new,
        "",
    );
    assert_eq!(status, 200, "{body}");
}

/// Every operator verb requires an initialized private state, so none of them
/// can manufacture a credential or a snapshot where there is no state.
#[test]
fn native_ops_verbs_require_initialized_state() {
    let root = tempfile::tempdir().unwrap();
    let empty = root.path().join("empty");
    fs::create_dir(&empty).unwrap();
    for args in [vec!["backup", "--state-dir"], vec!["rotate", "--state-dir"]] {
        let mut command = native();
        command.args(&args).arg(&empty);
        if args[0] == "backup" {
            command.arg("--out").arg(root.path().join("x"));
        } else {
            command.arg("operator-token");
        }
        let output = command.output().unwrap();
        assert!(
            !output.status.success(),
            "{args:?} succeeded on uninitialized state"
        );
    }
}
