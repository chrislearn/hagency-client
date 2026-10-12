//! Real loopback HTTP protocol fixture. No provider/model or personal secrets.
use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
struct Fixture {
    origin: String,
    requests: Arc<Mutex<Vec<(String, Value)>>>,
    stop: Arc<AtomicBool>,
    device: Arc<Mutex<Option<String>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Fixture {
    fn new() -> Self {
        let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", socket.local_addr().unwrap());
        socket.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let root = origin.clone();
        let device = Arc::new(Mutex::new(None::<String>));
        let current_device = device.clone();
        let thread = std::thread::spawn(move || {
            while !done.load(Ordering::Acquire) {
                let Ok((mut stream, _)) = socket.accept() else {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut block = [0; 4096];
                let (head, body) = loop {
                    let n = stream.read(&mut block).unwrap_or(0);
                    if n == 0 {
                        break (String::new(), Vec::new());
                    }
                    bytes.extend_from_slice(&block[..n]);
                    if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let h = String::from_utf8(bytes[..i].to_vec()).unwrap();
                        let length = h
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|x| x.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= i + 4 + length {
                            break (h, bytes[i + 4..i + 4 + length].to_vec());
                        }
                    }
                };
                if head.is_empty() {
                    continue;
                };
                let route = head.lines().next().unwrap().to_owned();
                let payload = serde_json::from_slice(&body).unwrap_or_else(|_| {
                    Value::Object(
                        url::form_urlencoded::parse(&body)
                            .map(|(k, v)| (k.into_owned(), Value::String(v.into_owned())))
                            .collect(),
                    )
                });
                seen.lock().unwrap().push((route.clone(), payload.clone()));
                let until = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64
                    + 30_000;
                let response = if route.contains("/versions ") {
                    json!({"versions":["v1.15"],"unstable_features":{}})
                } else if route.contains("/api/hagency/v1/discovery ") {
                    json!({"product":"hagency-server","version":"0.1.0","protocolVersion":3,
                        "homeserver":format!("{root}/"),"issuer":format!("{root}/auth/"),
                        "capabilities":["pasion-oauth","owner-agent-appservice-v1","global-agent-identity-v2","execution-device-v1","owner-direct-v1"]})
                } else if route.contains("/auth_metadata ") {
                    json!({"issuer":format!("{root}/auth/"),"authorization_endpoint":format!("{root}/auth/authorize"),"token_endpoint":format!("{root}/auth/token"),"registration_endpoint":format!("{root}/auth/register"),"revocation_endpoint":format!("{root}/auth/revoke"),"response_types_supported":["code"],"grant_types_supported":["authorization_code","refresh_token"],"code_challenge_methods_supported":["S256"],"token_endpoint_auth_methods_supported":["none"]})
                } else if route.contains("/register ") {
                    assert!(
                        payload["grant_types"]
                            .as_array()
                            .unwrap()
                            .contains(&json!("refresh_token"))
                    );
                    assert!(
                        payload["grant_types"]
                            .as_array()
                            .unwrap()
                            .contains(&json!("authorization_code"))
                    );
                    assert_eq!(payload["token_endpoint_auth_method"], "none");
                    json!({"client_id":"desktop-fixture","client_id_issued_at":1,"client_name":"Hagency Desktop","application_type":"native","token_endpoint_auth_method":"none","grant_types":payload["grant_types"],"response_types":["code"],"redirect_uris":payload["redirect_uris"]})
                } else if route.contains("/token ") {
                    if payload["grant_type"] == "authorization_code" {
                        assert!(payload["code_verifier"].as_str().unwrap().len() >= 43);
                        assert!(
                            payload["code"] == "fixture-code"
                                || payload["code"] == "wrong-account-code"
                                || payload["code"] == "wrong-device-code"
                        );
                        json!({"access_token":if payload["code"]=="fixture-code"{"oauth-access"}else if payload["code"]=="wrong-account-code"{"oauth-wrong"}else{"oauth-wrong-device"},"refresh_token":if payload["code"]=="fixture-code"{"oauth-refresh"}else if payload["code"]=="wrong-account-code"{"oauth-refresh-wrong"}else{"oauth-refresh-wrong-device"},"token_type":"Bearer","expires_in":1})
                    } else {
                        assert!(
                            payload["refresh_token"] == "oauth-refresh"
                                || payload["refresh_token"] == "oauth-refresh-2"
                                || payload["refresh_token"] == "oauth-refresh-3"
                        );
                        let next = if payload["refresh_token"] == "oauth-refresh" {2} else if payload["refresh_token"] == "oauth-refresh-2" {3} else {4};
                        json!({"access_token":format!("oauth-access-{next}"),"refresh_token":format!("oauth-refresh-{next}"),"token_type":"Bearer","expires_in":3600})
                    }
                } else if route.contains("/api/hagency/v1/sessions/pasion ") {
                    json!({"token":"a".repeat(64),"userId":"fixture-user","mxid":"@fixture:localhost","validUntilMs":until})
                } else if route.contains("/api/hagency/v1/identity ") {
                    json!({"userId":"fixture-user","mxid":"@fixture:localhost","subject":"fixture-subject","issuer":format!("{root}/auth/"),"clientId":"desktop-fixture","validUntilMs":until})
                } else if route.contains("/api/hagency/v1/devices ") {
                    json!({"deviceId":"fixture-runtime-device","token":"b".repeat(64),"generation":1,"validUntilMs":until})
                } else if route.contains("/api/hagency/v1/projects ") {
                    json!({"projects":[]})
                } else if route.contains("/api/hagency/v1/sessions/current/renew ") {
                    json!({"validUntilMs":until})
                } else if route.contains("/whoami ") {
                    json!({"user_id":if head.lines().any(|line| line.eq_ignore_ascii_case("authorization: Bearer oauth-wrong")){"@other:localhost"}else{"@fixture:localhost"},"device_id":if head.lines().any(|line| line.eq_ignore_ascii_case("authorization: Bearer oauth-wrong-device")){Some("different-device".to_owned())}else{current_device.lock().unwrap().clone()}})
                } else {
                    json!({})
                };
                let body = response.to_string();
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        Self {
            origin,
            requests,
            stop,
            device,
            thread: Some(thread),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}
#[tokio::test]
async fn sdk_oauth_dcr_pkce_callback_whoami_refresh_and_session_roundtrip() {
    let fixture = Fixture::new();
    let client = Client::builder()
        .homeserver_url(&fixture.origin)
        .handle_refresh_tokens()
        .build()
        .await
        .unwrap();
    let methods = login_methods(&client).await.unwrap();
    assert!(methods.oauth);
    assert!(!methods.password && !methods.sso && methods.providers.is_empty());
    let device = fixture.device.clone();
    oauth_browser_login(&client, move |authorization| async move {
        let url = Url::parse(&authorization).unwrap();
        let params: std::collections::HashMap<_, _> = url
            .query_pairs()
            .map(|(a, b)| (a.into_owned(), b.into_owned()))
            .collect();
        *device.lock().unwrap() = Some(
            params["scope"]
                .split_whitespace()
                .find_map(|s| s.split_once("device:").map(|(_, d)| d.to_owned()))
                .unwrap(),
        );
        assert_eq!(params["prompt"], "login");
        assert_eq!(params["code_challenge_method"], "S256");
        assert!(!params["code_challenge"].is_empty());
        assert!(
            params["scope"].contains("client:api:*")
                || params["scope"].contains("msc2967.client:api:*")
        );
        let mut redirect = Url::parse(&params["redirect_uri"]).unwrap();
        redirect
            .query_pairs_mut()
            .append_pair("code", "fixture-code")
            .append_pair("state", &params["state"]);
        crate::http::client_builder()
            .build()?
            .get(redirect)
            .send()
            .await?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        client.whoami().await.unwrap().user_id.as_str(),
        "@fixture:localhost"
    );
    let saved =
        crate::persistence::PersistedUserSession::from_sdk(client.session().unwrap()).unwrap();
    let json = serde_json::to_vec(&saved).unwrap();
    let restored: crate::persistence::PersistedUserSession = serde_json::from_slice(&json).unwrap();
    assert_eq!(saved.meta(), restored.meta());
    let restored_client = Client::builder()
        .homeserver_url(&fixture.origin)
        .handle_refresh_tokens()
        .build()
        .await
        .unwrap();
    restored_client
        .restore_session(restored.into_sdk())
        .await
        .unwrap();
    assert_eq!(
        restored_client.oauth().client_id().unwrap().as_str(),
        "desktop-fixture"
    );
    let state = std::env::temp_dir().join(format!(
        "hagency-sdk-oauth-native-{}",
        rand::random::<u64>()
    ));
    let native = ::hagency::native_owner::NativeOwner::open_with_matrix(
        &state,
        &fixture.origin,
        "@fixture:localhost",
        Arc::new(SdkSource(restored_client.clone())),
    )
    .await
    .unwrap();
    assert_eq!(
        native
            .execute(::hagency::native_owner::Command::Projects)
            .await
            .unwrap()["projects"],
        json!([])
    );
    let stored_path = state.join("sdk-session-fixture.json");
    let stored = crate::persistence::FullSessionPersisted {
        client_session: crate::persistence::ClientSessionPersisted {
            homeserver: fixture.origin.clone(),
            db_path: "fixture-db".into(),
            passphrase: "fixture-passphrase".into(),
        },
        user_session: saved.clone(),
        sync_token: None,
        sliding_sync_version: crate::persistence::SlidingSyncVersion::None,
    };
    crate::persistence::write_private_session(&stored_path, &serde_json::to_vec(&stored).unwrap())
        .await
        .unwrap();
    crate::persistence::install_session_callbacks(&restored_client, stored_path.clone()).unwrap();
    let mut changed = restored_client.subscribe_to_session_changes();
    crate::hagency::backend::refresh_sdk_oauth(&restored_client, || Ok(())).await.unwrap();
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(2), changed.recv())
            .await
            .unwrap()
            .unwrap(),
        matrix_sdk::SessionChange::TokensRefreshed
    ));
    let persisted: crate::persistence::FullSessionPersisted =
        serde_json::from_slice(&std::fs::read(&stored_path).unwrap()).unwrap();
    assert!(
        matches!(&persisted.user_session,crate::persistence::PersistedUserSession::OAuth{user,..} if user.tokens.access_token=="oauth-access-2" && user.tokens.refresh_token.as_deref()==Some("oauth-refresh-2"))
    );
    assert_eq!(
        restored_client.access_token().as_deref(),
        Some("oauth-access-2")
    );
    assert_eq!(
        restored_client
            .oauth()
            .full_session()
            .unwrap()
            .user
            .tokens
            .refresh_token
            .as_deref(),
        Some("oauth-refresh-2")
    );
    let committed = std::fs::read(&stored_path).unwrap();
    assert!(
        crate::persistence::save_oauth_session_for_test(
            &restored_client,
            stored.client_session.clone(),
            crate::account_session::epoch(),
            stored_path.clone()
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&stored_path).unwrap(), committed); // Duplicate SDK callback registration fails before replacing credentials.
    // A fresh Hagency proof may be reused briefly; after its bounded cache,
    // the next request must use the SDK's rotated access snapshot.
    tokio::time::sleep(std::time::Duration::from_millis(5100)).await;
    assert_eq!(
        native
            .execute(::hagency::native_owner::Command::Projects)
            .await
            .unwrap()["projects"],
        json!([])
    );
    native.shutdown().await;
    assert_eq!(
        restored_client.access_token().as_deref(),
        Some("oauth-access-2")
    );
    restored_client.whoami().await.unwrap();
    let prior_requests = fixture.requests.lock().unwrap().len();
    let rejected = crate::hagency::backend::refresh_sdk_oauth(&restored_client, || Err(::hagency::native_owner::NativeError {status:401,code:"matrix_account_changed".into()})).await.unwrap_err();
    assert_eq!(rejected.code, "matrix_account_changed");
    assert_eq!(fixture.requests.lock().unwrap().len(), prior_requests, "stale identity cannot trigger an SDK refresh");
    let checks = std::sync::atomic::AtomicUsize::new(0);
    let late = crate::hagency::backend::refresh_sdk_oauth(&restored_client, || {
        if checks.fetch_add(1, Ordering::AcqRel) == 0 {Ok(())} else {Err(::hagency::native_owner::NativeError {status:401,code:"matrix_account_changed".into()})}
    }).await.unwrap_err();
    assert_eq!(late.code, "matrix_account_changed", "a refresh may finish but its old identity cannot be adopted");
    assert_eq!(checks.load(Ordering::Acquire), 2);
    let requests = fixture.requests.lock().unwrap();
    assert!(
        requests
            .iter()
            .any(|(p, b)| p.contains("/sessions/pasion ") && b["accessToken"] == "oauth-access")
    );
    assert!(requests.iter().any(
        |(p, b)| p.contains("/sessions/current/renew ") && b["accessToken"] == "oauth-access-2"
    ));
    assert!(
        !requests
            .iter()
            .any(|(p, _)| p.contains("/auth/revoke") || p.contains("oauth2/revoke"))
    );
    drop(requests);
    // A removed account cannot be recreated by a late refresh callback.
    {
        let _guard = crate::persistence::SESSION_IO.lock().unwrap();
        std::fs::remove_file(&stored_path).unwrap();
    }
    restored_client
        .oauth()
        .refresh_access_token()
        .await
        .unwrap();
    assert!(!stored_path.exists());
    let _ = std::fs::remove_dir_all(state);
    assert_eq!(
        fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(p, b)| p.contains("/token ") && b["grant_type"] == "authorization_code")
            .count(),
        1
    );
}
#[test]
fn oauth_http_is_only_permitted_on_loopback() {
    for host in [
        "http://127.0.0.1:8089/",
        "http://[::1]:8089/",
        "https://matrix.example/",
    ] {
        validate_oauth_origin(&Url::parse(host).unwrap()).unwrap();
    }
    for host in [
        "http://192.168.1.1/",
        "http://example.org/",
        "https://user:pass@matrix.example/",
    ] {
        assert!(validate_oauth_origin(&Url::parse(host).unwrap()).is_err());
    }
}

struct SdkSource(Client);
impl ::hagency::native_owner::MatrixTokenSource for SdkSource {
    fn access_token(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        ::hagency::native_owner::MatrixAccessToken,
                        ::hagency::native_owner::NativeError,
                    >,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            Ok(::hagency::native_owner::MatrixAccessToken {
                access_token: self.0.access_token().unwrap(),
                client_id: self.0.oauth().client_id().unwrap().as_str().into(),
            })
        })
    }
}

#[tokio::test]
async fn oauth_reauthentication_mismatch_revokes_only_new_grant_and_retains_previous_session() {
    let fixture = Fixture::new();
    let initial = Client::builder()
        .homeserver_url(&fixture.origin)
        .handle_refresh_tokens()
        .build()
        .await
        .unwrap();
    async fn authorize(
        client: &Client,
        device: Arc<Mutex<Option<String>>>,
        code: &'static str,
    ) -> anyhow::Result<()> {
        oauth_browser_login(client, move |authorization| async move {
            let url = Url::parse(&authorization)?;
            let params: std::collections::HashMap<_, _> = url
                .query_pairs()
                .map(|(a, b)| (a.into_owned(), b.into_owned()))
                .collect();
            *device.lock().unwrap() = Some(
                params["scope"]
                    .split_whitespace()
                    .find_map(|s| s.split_once("device:").map(|(_, d)| d.to_owned()))
                    .unwrap(),
            );
            let mut redirect = Url::parse(&params["redirect_uri"])?;
            redirect
                .query_pairs_mut()
                .append_pair("code", code)
                .append_pair("state", &params["state"]);
            crate::http::client_builder()
                .build()?
                .get(redirect)
                .send()
                .await?;
            Ok(())
        })
        .await
    }
    authorize(&initial, fixture.device.clone(), "fixture-code")
        .await
        .unwrap();
    let saved =
        crate::persistence::PersistedUserSession::from_sdk(initial.session().unwrap()).unwrap();
    let bytes = serde_json::to_vec(&saved).unwrap();
    for code in ["wrong-account-code", "wrong-device-code"] {
        let client = Client::builder()
            .homeserver_url(&fixture.origin)
            .handle_refresh_tokens()
            .build()
            .await
            .unwrap();
        client
            .restore_session(saved.clone().into_sdk())
            .await
            .unwrap();
        let result = authorize(&client, fixture.device.clone(), code).await;
        assert!(result.is_err());
        assert_eq!(serde_json::to_vec(&saved).unwrap(), bytes); // Failed temporary client never replaces the saved session.
        let recovered = Client::builder()
            .homeserver_url(&fixture.origin)
            .build()
            .await
            .unwrap();
        recovered
            .restore_session(saved.clone().into_sdk())
            .await
            .unwrap();
        assert_eq!(recovered.user_id().unwrap().as_str(), "@fixture:localhost");
        assert_eq!(recovered.access_token().as_deref(), Some("oauth-access"));
        assert_eq!(recovered.device_id(), initial.device_id());
    }
    let requests = fixture.requests.lock().unwrap();
    let revoked: Vec<_> = requests
        .iter()
        .filter(|(p, _)| p.contains("/auth/revoke "))
        .collect();
    assert_eq!(revoked.len(), 2);
    assert_eq!(revoked[0].1["token"], "oauth-refresh-wrong");
    assert_eq!(revoked[1].1["token"], "oauth-refresh-wrong-device");
    assert_eq!(
        requests
            .iter()
            .filter(|(p, _)| p.contains("/auth/register "))
            .count(),
        1
    );
    drop(requests);
    // Even a successful OAuth callback can arrive after account retirement.
    let stale_epoch = crate::account_session::epoch().wrapping_add(1);
    let result = crate::sliding_sync::finish_oauth_login(
        initial,
        crate::persistence::ClientSessionPersisted {
            homeserver: fixture.origin.clone(),
            db_path: "not-written".into(),
            passphrase: "not-written".into(),
        },
        stale_epoch,
    )
    .await;
    assert!(result.is_err());
    assert!(
        fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|(p, b)| p.contains("/auth/revoke ") && b["token"] == "oauth-refresh")
    );
}

#[test]
fn corrupted_oauth_identity_never_falls_back_to_matrix_authentication() {
    for id in [Value::Null, json!(3), json!(""), json!("x".repeat(256))] {
        let value = json!({"oauth_client_id":id,"user_id":"@fixture:localhost","device_id":"DEVICE","access_token":"fixture"});
        assert!(serde_json::from_value::<crate::persistence::PersistedUserSession>(value).is_err());
    }
    assert!(matches!(
        serde_json::from_value::<crate::persistence::PersistedUserSession>(
            json!({"user_id":"@fixture:localhost","device_id":"DEVICE","access_token":"fixture"})
        )
        .unwrap(),
        crate::persistence::PersistedUserSession::Matrix(_)
    ));
}
