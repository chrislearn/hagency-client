//! NativeOwner → actual Server board API → PostgreSQL. The separately launched
//! server fixture explicitly doubles Pasion and Matrix, and launches no model.
use hagency::native_owner::{self, Command, MatrixAccessToken, MatrixTokenSource, NativeError};
use hagency::project_board::*;
use serde_json::Value;
use std::sync::Arc;
struct Source(&'static str);
impl MatrixTokenSource for Source {
    fn access_token(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>> + Send + '_>,
    > {
        Box::pin(async {
            Ok(MatrixAccessToken {
                access_token: format!("fixture-oauth-{}", self.0),
                client_id: "fixture-native-client".into(),
            })
        })
    }
}
#[tokio::test]
#[ignore = "requires the disposable Server project_board_fixture via HAGENCY_BOARD_FIXTURE_INFO"]
async fn native_project_board_two_owners_share_durable_tasks_and_revocation() {
    let info: Value = serde_json::from_slice(
        &std::fs::read(std::env::var("HAGENCY_BOARD_FIXTURE_INFO").expect("fixture info required"))
            .unwrap(),
    )
    .unwrap();
    let origin = info["origin"].as_str().unwrap();
    assert!(origin.starts_with("http://127.0.0.1:"));
    let project = info["projectId"].as_str().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let alice = native_owner::open_with_matrix(
        &temp.path().join("alice"),
        origin,
        "@alice:example.test",
        Arc::new(Source("alice")),
    )
    .await
    .unwrap();
    let bob = native_owner::open_with_matrix(
        &temp.path().join("bob"),
        origin,
        "@bob:example.test",
        Arc::new(Source("bob")),
    )
    .await
    .unwrap();
    let command = |action| Command::ProjectBoard {
        project_id: project.into(),
        action,
    };
    let intent_prefix = format!(
        "native-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let intent = Create {
        idempotency_key: intent_prefix.clone(),
        fields: Fields {
            title: "Native two-owner shared task / 中文".into(),
            ..Default::default()
        },
        state: "todo".into(),
    };
    let created = alice
        .execute(command(BoardAction::Create {
            input: intent.clone(),
        }))
        .await
        .unwrap();
    let id = created["task"]["id"].as_str().unwrap().to_owned();
    let replay = alice
        .execute(command(BoardAction::Create { input: intent }))
        .await
        .unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(created["task"], replay["task"]);
    let read = bob
        .execute(command(BoardAction::Detail { task: id.clone() }))
        .await
        .unwrap();
    assert_eq!(read["task"], created["task"]);
    bob.execute(command(BoardAction::Move {
        task: id.clone(),
        input: Move {
            idempotency_key: format!("{intent_prefix}-move"),
            expected_version: 1,
            state: "in_review".into(),
            expected_sequence: None,
            before: None,
            after: None,
            position: Position::Append,
        },
    }))
    .await
    .unwrap();
    let updated = alice
        .execute(command(BoardAction::Detail { task: id.clone() }))
        .await
        .unwrap();
    assert_eq!(updated["task"]["state"], "in_review");
    assert_eq!(updated["task"]["version"], 2);
    let conflict = alice
        .execute(command(BoardAction::Update {
            task: id.clone(),
            input: Update {
                idempotency_key: format!("{intent_prefix}-stale"),
                expected_version: 1,
                fields: Fields {
                    title: "Must not overwrite".into(),
                    ..Default::default()
                },
            },
        }))
        .await
        .unwrap_err();
    assert_eq!(conflict.code, "board_version_conflict");
    alice.shutdown().await;
    drop(alice);
    let alice = native_owner::open_with_matrix(
        &temp.path().join("alice"),
        origin,
        "@alice:example.test",
        Arc::new(Source("alice")),
    )
    .await
    .unwrap();
    assert_eq!(
        alice
            .execute(command(BoardAction::Detail { task: id.clone() }))
            .await
            .unwrap()["task"],
        updated["task"]
    );
    reqwest::Client::new()
        .post(format!("{origin}fixture/bob/leave"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let denied = bob
        .execute(command(BoardAction::Snapshot {
            query: Query::default(),
        }))
        .await
        .unwrap_err();
    assert_eq!(denied.code, "project_membership_required");
    let outsider = native_owner::open_with_matrix(
        &temp.path().join("outsider"),
        origin,
        "@outsider:example.test",
        Arc::new(Source("outsider")),
    )
    .await
    .unwrap();
    assert_eq!(
        outsider
            .execute(command(BoardAction::Detail { task: id }))
            .await
            .unwrap_err()
            .code,
        "project_membership_required"
    );
    outsider.shutdown().await;
    bob.shutdown().await;
    alice.shutdown().await;
}
