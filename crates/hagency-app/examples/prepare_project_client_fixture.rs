//! Prepare an isolated, explicitly fake OAuth session for the real `hagency`
//! binary. No production auth path is replaced; SDK restore/issuer/whoami and
//! NativeOwner authorization run normally against project_board_fixture.
#[cfg(debug_assertions)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::{Value, json};
    let info: Value = serde_json::from_slice(&std::fs::read(std::env::var(
        "HAGENCY_BOARD_FIXTURE_INFO",
    )?)?)?;
    let origin = info["origin"].as_str().ok_or("missing fixture origin")?;
    let url: url::Url = origin.parse()?;
    if url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || info["identitySource"]
            != "fixture Pasion/Matrix; real native authorization and board PostgreSQL/API"
    {
        return Err("only the explicit disposable loopback fixture is supported".into());
    }
    let root = std::env::temp_dir().join("hagency-full-client-local");
    // Refuse to overwrite even a previous fixture session.
    std::fs::create_dir(&root)?;
    let user = "@alice:example.test";
    let state = root
        .join(format!(
            "account_{}",
            blake3::hash(user.as_bytes()).to_hex()
        ))
        .join("persistent_state");
    std::fs::create_dir_all(&state)?;
    let session = json!({
        "client_session":{"homeserver":origin,"db_path":"matrix-fixture-db","passphrase":"disposable-fixture-only"},
        "user_session":{"oauth_client_id":"fixture-native-client","user_id":user,"device_id":"FIXTURE_ALICE","access_token":"fixture-oauth-alice"},
        "sliding_sync_version":"Native"
    });
    // Validate against the same persistence type used by the complete app.
    let _: hagency_app::persistence::FullSessionPersisted =
        serde_json::from_value(session.clone())?;
    std::fs::write(state.join("session"), serde_json::to_vec_pretty(&session)?)?;
    std::fs::write(root.join("latest_user_id.txt"), user)?;
    std::fs::write(root.join("ui-language.json"), "\"zh-CN\"")?;
    std::fs::write(
        root.join("FIXTURE.txt"),
        "Disposable identity sources; real SDK + complete client + local Server + PostgreSQL. No production account data.\n",
    )?;
    println!("Prepared isolated full-client data: {}", root.display());
    Ok(())
}
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("This fixture preparation requires a debug build.");
}
