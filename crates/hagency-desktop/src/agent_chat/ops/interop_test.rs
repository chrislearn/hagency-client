//! Opt-in interoperability test against the pinned, isolated real Hagency backend.
use super::{backend::Transport, protocol::*};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
};
struct Backend {
    child: Child,
    output: BufReader<std::process::ChildStdout>,
}
impl Backend {
    fn start() -> Self {
        let mut child =
            Command::new(std::env::var("ROBRIX_TEST_NODE").unwrap_or_else(|_| "node".into()))
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tools/hagency/backend-fixture.mjs"
                ))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("start fixture Node process");
        let output = BufReader::new(child.stdout.take().unwrap());
        Self { child, output }
    }
    fn read(&mut self) -> Value {
        for line in self.output.by_ref().lines() {
            let line = line.unwrap();
            if let Some(value) = line.strip_prefix("ROBRIX_FIXTURE:") {
                return serde_json::from_str(value).unwrap();
            }
        }
        panic!("Backend fixture stopped before responding");
    }
    fn command(&mut self, value: Value) -> Value {
        writeln!(self.child.stdin.as_mut().unwrap(), "{value}").unwrap();
        self.read()
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        let _ = writeln!(
            self.child.stdin.as_mut().unwrap(),
            "{}",
            json!({"kind":"stop"})
        );
        for _ in 0..30 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn offered(session: &Session, kind: &str) -> Value {
    let snapshot = session.snapshot.as_ref().unwrap();
    ["attention", "tasks", "queue", "worktrees"]
        .iter()
        .flat_map(|k| snapshot[*k].as_array().into_iter().flatten())
        .flat_map(|row| row["available_actions"].as_array().into_iter().flatten())
        .find(|a| a["kind"] == kind)
        .expect("backend must offer the operation")
        .clone()
}
#[tokio::test]
#[ignore = "requires ROBRIX_HAGENCY_SOURCE pinned checkout with npm ci and build:router, and ROBRIX_TEST_NODE"]
async fn real_backend_signed_sessions_mutations_and_revocation() {
    let mut backend = Backend::start();
    let ready = backend.read();
    let config = Config {
        agent: "worker".into(),
        project_room: "!project:test".try_into().unwrap(),
        owner_room: "!approval:test".try_into().unwrap(),
        bridge: "@bridge:test".try_into().unwrap(),
        owner_agent: None,
        origin: ready["origin"].as_str().unwrap().into(),
        fingerprint: ready["fingerprint"].as_str().unwrap().into(),
    };
    let key = ClientKey::generate();
    let nonce = nonce();
    let grant = backend.command(json!({"kind":"bootstrap","jwk":key.jwk(),"nonce":nonce}));
    let transport = Transport::new().unwrap();
    let mut session = transport
        .exchange(
            key,
            config,
            "@owner:test".try_into().unwrap(),
            grant,
            &nonce,
        )
        .await
        .unwrap();
    transport.refresh(&mut session).await.unwrap();
    assert!(session.snapshot.as_ref().unwrap()["attention"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["kind"] == "parked_approval"));
    let action = offered(&session, "cancel_dispatch");
    let (path, body) = session
        .command(&action, None, None, "", "", "cancel-once")
        .unwrap();
    let result = transport
        .request(&session, &path, Some(&body))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["state"], "outcome_unknown");
    let replay = transport
        .request(&session, &path, Some(&body))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay["idempotent_request_replay"], true);
    assert!(transport.invalidated(&session).await.unwrap());
    transport.refresh(&mut session).await.unwrap();
    let source = offered(&session, "begin_outcome_inspection");
    let (path, body) = session
        .command(&source, None, None, "", "", "inspect-once")
        .unwrap();
    let inspection = transport
        .request(&session, &path, Some(&body))
        .await
        .unwrap()
        .unwrap();
    session.validate_inspection(&inspection, &source).unwrap();
    let action = &inspection["resolution_action"];
    let (path, body) = session
        .command(
            action,
            Some(&inspection),
            Some("continue"),
            "Fixture inspected",
            "Run fixture verification",
            "resolve-once",
        )
        .unwrap();
    let result = transport
        .request(&session, &path, Some(&body))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["resolution"], "continue");
    let replay = transport
        .request(&session, &path, Some(&body))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay["idempotent_request_replay"], true);
    backend.command(json!({"kind":"dirty"}));
    transport.refresh(&mut session).await.unwrap();
    let action = offered(&session, "mark_resource_inspected");
    let (path, body) = session
        .command(&action, None, None, "", "", "mark-once")
        .unwrap();
    transport
        .request(&session, &path, Some(&body))
        .await
        .unwrap();
    transport.refresh(&mut session).await.unwrap();
    assert_eq!(
        session.snapshot.as_ref().unwrap()["worktrees"][0]["dirty"],
        false
    );
    backend.command(json!({"kind":"revoke"}));
    assert!(transport.refresh(&mut session).await.is_err());
}
