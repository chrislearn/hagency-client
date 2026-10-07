use super::*;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct OAuthFixture {
    origin: String,
    state: Arc<Mutex<OAuthState>>,
}
struct OAuthState {
    owner: String,
    active: bool,
    challenge: Option<String>,
    enrollments: usize,
    lifetime: u64,
}
#[handler]
impl OAuthFixture {
    async fn handle(&self, depot: &mut Depot) {
        depot.insert_typed(self.clone());
    }
}
#[handler]
async fn oauth(req: &mut Request, depot: &Depot, res: &mut Response) {
    let f = depot.get_typed::<OAuthFixture>().unwrap();
    let path = req.uri().path().to_owned();
    let result = match path.as_str() {
        "/_hagency/client/v1/discovery" => json!({"selfService":true}),
        "/_pasion/oauth2/registration" => {
            let value: Value = req.parse_json().await.unwrap();
            assert_eq!(value["token_endpoint_auth_method"], "none");
            // Pasion's Matrix DCR policy requires HTTPS application metadata,
            // even when a native client's callback uses HTTP loopback.
            assert_eq!(
                value["client_uri"],
                "https://github.com/chrislearn/hagency-client"
            );
            assert_eq!(value["application_type"], "native");
            let redirect =
                reqwest::Url::parse(value["redirect_uris"][0].as_str().unwrap()).unwrap();
            assert_eq!(redirect.port(), None);
            assert_eq!(redirect.path(), "/console/server-login/callback");
            json!({"client_id":"fixture-public-client"})
        }
        "/_pasion/oauth2/token" => {
            use base64::Engine;
            let payload = String::from_utf8(req.payload().await.unwrap().to_vec()).unwrap();
            let fields: std::collections::HashMap<_, _> =
                reqwest::Url::parse(&format!("http://fixture/?{payload}"))
                    .unwrap()
                    .query_pairs()
                    .into_owned()
                    .collect();
            assert_eq!(fields["code"], "valid-code");
            let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(Sha256::digest(fields["code_verifier"].as_bytes()));
            assert_eq!(
                f.state.lock().unwrap().challenge.as_deref(),
                Some(challenge.as_str())
            );
            json!({"access_token":"oauth-user-secret-0123456789","expires_in":f.state.lock().unwrap().lifetime})
        }
        "/_hagency/client/v1/identity" => {
            assert_eq!(
                req.headers().get("authorization").unwrap(),
                "Bearer oauth-user-secret-0123456789"
            );
            let state = f.state.lock().unwrap();
            if !state.active {
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(json!({"code":"sign_in_required"})));
                return;
            }
            json!({"userId":state.owner,"subject":state.owner,"clientId":"fixture-public-client"})
        }
        "/_hagency/client/v1/fleets" => {
            let input: Value = req.parse_json().await.unwrap();
            assert!(input["ownerMxid"].is_null());
            assert!(!input["installationId"].as_str().unwrap().is_empty());
            let mut state = f.state.lock().unwrap();
            state.enrollments += 1;
            let fleet = format!("hf_{}", "c".repeat(32));
            json!({"identity":{"userId":state.owner,"subject":state.owner},"homeserver":f.origin,
            "configuration":{"fleetId":fleet,"serverName":"example.test","credentialVersion":1,
                "registration":{"id":fleet,"url":format!("{}/api/relay/v2/fixture",f.origin),"as_token":"as-token-value-secret","hs_token":"hs-token-value-secret","sender_localpart":format!("{fleet}_representative"),"namespaces":{"users":[{"exclusive":true,"regex":format!("^@{fleet}_[a-z0-9_]+:example\\.test$")}],"aliases":[],"rooms":[]},"rate_limited":true,"receive_ephemeral":false},
                "transport":{"mode":"outbound","url":format!("{}/api/fleet/v2/{fleet}",f.origin),"token":"machine-token-secret-0123456789","generation":1}}})
        }
        _ => panic!("Unexpected OAuth fixture path {path}"),
    };
    res.render(Json(result));
}
async fn start(service: &Service, f: &OAuthFixture, cookie: &str) -> (String, String) {
    let mut response = post("/console/server-login/start", cookie)
        .json(&json!({"server":f.origin,"name":"Workstation"}))
        .send(service)
        .await;
    assert_eq!(response.status_code, Some(StatusCode::OK));
    let nonce = response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let url = response.take_json::<Value>().await.unwrap()["url"]
        .as_str()
        .unwrap()
        .to_owned();
    let url = reqwest::Url::parse(&url).unwrap();
    let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query["code_challenge_method"], "S256");
    assert!(query["scope"].contains("urn:matrix:client:api:*"));
    f.state.lock().unwrap().challenge = Some(query["code_challenge"].clone());
    (query["state"].clone(), nonce)
}
async fn callback(service: &Service, state: &str, nonce: &str) -> Response {
    TestClient::get(format!(
        "{BASE}/console/server-login/callback?state={state}&code=valid-code"
    ))
    .add_header("host", "127.0.0.1:13300", true)
    .add_header("cookie", nonce, true)
    .send(service)
    .await
}
#[tokio::test]
async fn native_server_login_pkce_binding_import_and_revocation() {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let fake = OAuthFixture {
        origin: format!("http://{address}"),
        state: Arc::new(Mutex::new(OAuthState {
            owner: "@owner:example.test".into(),
            active: true,
            challenge: None,
            enrollments: 0,
            lifetime: 900,
        })),
    };
    let handler = Service::new(
        Router::new()
            .hoop(fake.clone())
            .push(Router::with_path("{**path}").goal(oauth)),
    );
    let acceptor = TcpListener::new(address).bind().await;
    let server = tokio::spawn(async move {
        Server::new(acceptor).serve(handler).await;
    });
    let f = Fixture::new("127.0.0.1:13300".parse().unwrap(), None);
    let state = f.root.path().join("state");
    let service = Service::new(f.app.clone().with_palpo_import(state.clone()).router());
    let mut denied = post("/console/server-login/start", "")
        .json(&json!({"server":fake.origin,"name":"Workstation"}))
        .send(&service)
        .await;
    assert_eq!(
        denied.take_json::<Value>().await.unwrap()["code"],
        "local_access_required"
    );
    let local = session(&service).await;
    let (oauth_state, nonce) = start(&service, &fake, &local).await;
    let bad = callback(&service, &oauth_state, "hagency_server_login=wrong").await;
    assert!(
        !bad.headers()
            .get_all("set-cookie")
            .iter()
            .any(|v| v.to_str().unwrap().starts_with("hagency_console="))
    );
    let reply = callback(&service, &oauth_state, &nonce).await;
    assert_eq!(reply.status_code, Some(StatusCode::SEE_OTHER));
    let mut observation = get("/console/server-login", &local).send(&service).await;
    let observed = observation.take_json::<Value>().await.unwrap();
    assert_eq!(observed["status"]["state"], "saved", "{observed}");
    let cookie = reply
        .headers()
        .get_all("set-cookie")
        .iter()
        .find(|v| v.to_str().unwrap().starts_with("hagency_console="))
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie.contains("Max-Age=900"));
    let cookie = cookie.split(';').next().unwrap().to_owned();
    assert_eq!(
        get("/console/api/project-sides", &cookie)
            .send(&service)
            .await
            .status_code,
        Some(StatusCode::OK)
    );
    let binding: Value =
        serde_json::from_slice(&std::fs::read(state.join("server-login.json")).unwrap()).unwrap();
    assert_eq!(binding["owner"], "@owner:example.test");
    assert!(!binding.to_string().contains("secret"));
    assert_eq!(
        std::fs::read_to_string(state.join("palpo.machine_token")).unwrap(),
        "machine-token-secret-0123456789"
    );
    let mut status = get("/console/server-login", &cookie).send(&service).await;
    assert_eq!(
        status.take_json::<Value>().await.unwrap()["status"]["state"],
        "saved"
    );
    // Bound users can sign in without bootstrap credentials. A different
    // Pasion account cannot get a new local cookie or change the binding.
    let (next, nonce) = start(&service, &fake, "").await;
    fake.state.lock().unwrap().owner = "@other:example.test".into();
    let wrong = callback(&service, &next, &nonce).await;
    assert!(
        !wrong
            .headers()
            .get_all("set-cookie")
            .iter()
            .any(|v| v.to_str().unwrap().starts_with("hagency_console="))
    );
    assert_eq!(fake.state.lock().unwrap().enrollments, 1);
    fake.state.lock().unwrap().owner = "@owner:example.test".into();
    fake.state.lock().unwrap().lifetime = 1;
    let (short, nonce) = start(&service, &fake, "").await;
    let reply = callback(&service, &short, &nonce).await;
    let short_cookie = reply
        .headers()
        .get_all("set-cookie")
        .iter()
        .find(|v| v.to_str().unwrap().starts_with("hagency_console="))
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    fake.state.lock().unwrap().lifetime = 900;
    let (long, nonce) = start(&service, &fake, "").await;
    let _ = callback(&service, &long, &nonce).await;
    assert_eq!(
        get("/console/api/project-sides", &short_cookie)
            .send(&service)
            .await
            .status_code,
        Some(StatusCode::UNAUTHORIZED),
        "expired remote sessions cannot become local recovery sessions when their token is pruned"
    );
    fake.state.lock().unwrap().active = false;
    // The validation cache is deliberately bounded to 30 seconds.
    tokio::time::sleep(std::time::Duration::from_secs(31)).await;
    assert_eq!(
        get("/console/api/project-sides", &cookie)
            .send(&service)
            .await
            .status_code,
        Some(StatusCode::UNAUTHORIZED)
    );
    let hashes: Value =
        serde_json::from_slice(&std::fs::read(state.join("console-logins.json")).unwrap()).unwrap();
    assert_eq!(
        hashes["sessions"].as_array().unwrap().len(),
        1,
        "only the local bootstrap session persists"
    );
    f.close().await;
    server.abort();
}
