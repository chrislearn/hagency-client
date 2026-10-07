//! Pasion login and Fleet enrollment. All OAuth and machine credentials stay
//! in the Rust host. The browser receives navigation URLs and a local cookie.
use super::{COOKIE, Error, console, current, same_origin};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Url};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const CALLBACK: &str = "/console/server-login/callback";
const NONCE_COOKIE: &str = "hagency_server_login";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    origin: String,
    client_id: String,
    installation_id: String,
    name: String,
    owner: Option<String>,
    subject: Option<String>,
}
struct Pending {
    state: String,
    verifier: String,
    nonce: String,
    redirect: String,
    binding: Binding,
    expires: Instant,
}
struct RemoteSession {
    token: String,
    binding: Binding,
    checked: Instant,
    expires: Instant,
}
pub(super) struct ServerLogin {
    path: Option<PathBuf>,
    binding: Mutex<Option<Binding>>,
    pending: Mutex<Option<Pending>>,
    status: Mutex<Value>,
    sessions: Mutex<std::collections::HashMap<String, RemoteSession>>,
}
impl ServerLogin {
    pub(super) fn new(state: Option<&Path>) -> Result<Self, Error> {
        let path = state.map(|dir| dir.join("server-login.json"));
        let binding = match &path {
            Some(path) if path.exists() => {
                let raw =
                    hagency_store::private::read_secret(path).map_err(|_| Error::Unavailable)?;
                if raw.len() > 16384 {
                    return Err(Error::Unavailable);
                }
                let value: Binding =
                    serde_json::from_slice(&raw).map_err(|_| Error::Unavailable)?;
                origin(&value.origin).map_err(|_| Error::Unavailable)?;
                if value.client_id.is_empty()
                    || value.client_id.len() > 128
                    || value.installation_id.len() < 32
                    || value.installation_id.len() > 128
                    || !value
                        .installation_id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    || value.name.trim().is_empty()
                    || value.name.chars().count() > 128
                    || value.owner.is_some() != value.subject.is_some()
                    || value
                        .owner
                        .as_ref()
                        .is_some_and(|v| v.is_empty() || v.len() > 255)
                    || value
                        .subject
                        .as_ref()
                        .is_some_and(|v| v.is_empty() || v.len() > 255)
                {
                    return Err(Error::Unavailable);
                }

                Some(value)
            }
            _ => None,
        };
        Ok(Self {
            path,
            binding: Mutex::new(binding),
            pending: Mutex::new(None),
            status: Mutex::new(json!({"state":"idle"})),
            sessions: Mutex::new(std::collections::HashMap::new()),
        })
    }
    /// Only Pasion sessions use this gate. Local recovery sessions remain local.
    /// Revalidate against the pinned server every 30 seconds; fail closed when
    /// its authentication service is unavailable or the token is revoked.
    pub(super) async fn validate(&self, cookie: &str) -> Result<(), Error> {
        let key = URL_SAFE_NO_PAD.encode(Sha256::digest(cookie.as_bytes()));
        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&key) else {
            return Ok(());
        };
        if session.expires <= Instant::now() {
            return Err(Error::Unauthorized);
        }
        if session.checked.elapsed() < Duration::from_secs(30) {
            return Ok(());
        }
        let server = origin(&session.binding.origin).map_err(|_| Error::Unauthorized)?;
        let client = http().map_err(|_| Error::Unavailable)?;
        let identity = get(
            &client,
            server.join("/_hagency/client/v1/identity").unwrap(),
            Some(&session.token),
        )
        .await
        .map_err(|_| Error::Unauthorized)?;
        if identity["userId"] != session.binding.owner.as_deref().unwrap_or("")
            || identity["subject"] != session.binding.subject.as_deref().unwrap_or("")
            || identity["clientId"] != session.binding.client_id
        {
            return Err(Error::Unauthorized);
        }
        session.checked = Instant::now();
        Ok(())
    }
    async fn save(&self, binding: Binding) -> Result<(), &'static str> {
        let path = self.path.as_ref().ok_or("local_state_unavailable")?;
        let raw = serde_json::to_vec(&binding).map_err(|_| "local_state_unavailable")?;
        hagency_store::private::replace(path, &raw).map_err(|_| "local_state_unavailable")?;
        *self.binding.lock().await = Some(binding);
        Ok(())
    }
}
fn random() -> Result<String, &'static str> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| "random_unavailable")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
fn origin(value: &str) -> Result<Url, &'static str> {
    let url = Url::parse(value).map_err(|_| "invalid_server")?;
    let loopback = url.host_str().is_some_and(|h| {
        h.trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    });
    if value.len() > 2048
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || !(url.scheme() == "https" || url.scheme() == "http" && loopback)
    {
        return Err("invalid_server");
    }
    Ok(url)
}
fn http() -> Result<Client, &'static str> {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| "server_unavailable")
}
async fn response(mut response: reqwest::Response) -> Result<Value, &'static str> {
    let status = response.status();
    let mut raw = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "server_unavailable")? {
        if raw.len() + chunk.len() > 128 * 1024 {
            return Err("invalid_server_response");
        }
        raw.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&raw).map_err(|_| "invalid_server_response")?;
    if !status.is_success() {
        return Err(match value["code"].as_str() {
            Some("self_service_disabled") => "self_service_disabled",
            Some("fleet_limit") => "fleet_limit",
            Some("sign_in_required") => "sign_in_required",
            _ => "server_request_failed",
        });
    }
    Ok(value)
}
async fn get(client: &Client, url: Url, token: Option<&str>) -> Result<Value, &'static str> {
    let mut request = client.get(url);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    response(request.send().await.map_err(|_| "server_unavailable")?).await
}
fn cookie(req: &Request, name: &str) -> Option<String> {
    req.headers()
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|v| {
            v.trim()
                .strip_prefix(&format!("{name}="))
                .map(str::to_owned)
        })
}
fn local_redirect(req: &Request) -> Result<String, &'static str> {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .ok_or("invalid_local_origin")?;
    let url =
        Url::parse(&format!("http://{host}{CALLBACK}")).map_err(|_| "invalid_local_origin")?;
    if !url.host_str().is_some_and(|h| {
        h.trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    }) {
        return Err("invalid_local_origin");
    }
    Ok(url.to_string())
}
pub(super) fn router() -> Router {
    Router::with_path("server-login")
        .get(login_status)
        .push(Router::with_path("start").post(start))
        .push(Router::with_path("callback").get(callback))
}
#[handler]
async fn login_status(req: &mut Request, depot: &Depot, res: &mut Response) {
    if !same_origin(req, depot, false) {
        super::failed(res, Error::Invalid);
        return;
    }
    let Ok(console) = console(depot) else {
        super::failed(res, Error::Unavailable);
        return;
    };
    let binding = console.0.server_login.binding.lock().await;
    res.render(Json(json!({"configured":binding.as_ref().is_some_and(|b|b.owner.is_some()),"server":binding.as_ref().map(|b|&b.origin),"name":binding.as_ref().map(|b|&b.name),"status":*console.0.server_login.status.lock().await})));
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    server: String,
    name: String,
}
#[handler]
async fn start(req: &mut Request, depot: &Depot, res: &mut Response) {
    let result = begin(req, depot).await;
    match result {
        Ok((url, nonce)) => {
            res.add_header("set-cookie",format!("{NONCE_COOKIE}={nonce}; HttpOnly; SameSite=Lax; Path=/console/server-login; Max-Age=300"),true).unwrap();
            res.render(Json(json!({"url":url})));
        }
        Err(code) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({"code":code})));
        }
    }
}
async fn begin(req: &mut Request, depot: &Depot) -> Result<(String, String), &'static str> {
    if !same_origin(req, depot, true) {
        return Err("invalid_local_origin");
    }
    let console = console(depot).map_err(|_| "local_state_unavailable")?;
    let login = &console.0.server_login;
    let binding = login.binding.lock().await.clone();
    // A server account can re-open its own previously bound client. The first
    // binding and any server change still require trusted local operator access.
    if binding.as_ref().is_none_or(|b| b.owner.is_none()) && current(req, depot).is_err() {
        return Err("local_access_required");
    }
    let raw = super::body(req, 8192).await.map_err(|_| "invalid_login")?;
    let input: Start = serde_json::from_slice(&raw).map_err(|_| "invalid_login")?;
    let server = origin(&input.server)?;
    if input.name.trim().is_empty() || input.name.chars().count() > 128 {
        return Err("invalid_name");
    }
    if binding
        .as_ref()
        .is_some_and(|b| b.owner.is_some() && b.origin != server.as_str())
    {
        return Err("owner_mismatch");
    }
    let client = http()?;
    get(
        &client,
        server.join("/_hagency/client/v1/discovery").unwrap(),
        None,
    )
    .await?;
    let mut binding = match binding.filter(|b| b.origin == server.as_str()) {
        Some(binding) => binding,
        None => {
            let redirect = local_redirect(req)?;
            let mut registered = Url::parse(&redirect).unwrap();
            registered
                .set_port(None)
                .map_err(|_| "invalid_local_origin")?;
            let registration=response(client.post(server.join("/_pasion/oauth2/registration").unwrap()).json(&json!({"client_name":"Hagency Client","client_uri":"https://github.com/chrislearn/hagency-client","application_type":"native","token_endpoint_auth_method":"none","grant_types":["authorization_code","refresh_token"],"response_types":["code"],"redirect_uris":[registered]})).send().await.map_err(|_|"server_unavailable")?).await?;
            let client_id = registration["client_id"]
                .as_str()
                .filter(|v| v.len() <= 128)
                .ok_or("invalid_server_response")?
                .to_owned();
            Binding {
                origin: server.to_string(),
                client_id,
                installation_id: random()?,
                name: input.name.trim().to_owned(),
                owner: None,
                subject: None,
            }
        }
    };
    if binding.owner.is_none() {
        binding.name = input.name.trim().to_owned();
    }
    login.save(binding.clone()).await?;
    let state = random()?;
    let verifier = random()?;
    let nonce = random()?;
    let redirect = local_redirect(req)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut authorize = server.join("/_pasion/authorize").unwrap();
    let device = format!("Hagency{}", &binding.installation_id[..16]);
    authorize.query_pairs_mut().extend_pairs([
        ("response_type", "code"),
        ("client_id", binding.client_id.as_str()),
        ("redirect_uri", redirect.as_str()),
        ("state", state.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        (
            "scope",
            format!("urn:matrix:client:api:* urn:matrix:client:device:{device}").as_str(),
        ),
    ]);
    *login.pending.lock().await = Some(Pending {
        state,
        verifier,
        nonce: nonce.clone(),
        redirect,
        binding,
        expires: Instant::now() + Duration::from_secs(300),
    });
    *login.status.lock().await = json!({"state":"signing_in"});
    Ok((authorize.to_string(), nonce))
}
#[handler]
async fn callback(req: &mut Request, depot: &Depot, res: &mut Response) {
    let result = finish(req, depot).await;
    let Ok(console) = console(depot) else {
        super::failed(res, Error::Unavailable);
        return;
    };
    if result
        .as_ref()
        .is_err_and(|code| *code == "invalid_oauth_state")
    {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(json!({"code":"invalid_oauth_state"})));
        return;
    }
    match result {
        Ok((session, status)) => {
            *console.0.server_login.status.lock().await = status;
            res.add_header(
                "set-cookie",
                format!(
                    "{COOKIE}={session}; HttpOnly; SameSite=Strict; Path=/console; Max-Age=900"
                ),
                true,
            )
            .unwrap();
        }
        Err(code) => {
            *console.0.server_login.status.lock().await = json!({"state":"failed","code":code});
        }
    }
    res.add_header(
        "set-cookie",
        format!("{NONCE_COOKIE}=; HttpOnly; SameSite=Lax; Path=/console/server-login; Max-Age=0"),
        false,
    )
    .unwrap();
    res.status_code(StatusCode::SEE_OTHER);
    res.add_header("location", "/console/project-sides/", true)
        .unwrap();
}
async fn finish(req: &mut Request, depot: &Depot) -> Result<(String, Value), &'static str> {
    let console = console(depot).map_err(|_| "local_state_unavailable")?;
    let login = &console.0.server_login;
    let state = req.query::<String>("state").ok_or("invalid_oauth_state")?;
    let mut lock = login.pending.lock().await;
    let pending = lock.as_ref().ok_or("invalid_oauth_state")?;
    if pending.expires <= Instant::now()
        || pending.state != state
        || cookie(req, NONCE_COOKIE).as_deref() != Some(pending.nonce.as_str())
    {
        return Err("invalid_oauth_state");
    }
    let mut pending = lock.take().unwrap();
    drop(lock);
    let code = req
        .query::<String>("code")
        .filter(|c| c.len() <= 4096)
        .ok_or("oauth_login_cancelled")?;
    let client = http()?;
    let server = origin(&pending.binding.origin)?;
    let tokens = response(
        client
            .post(server.join("/_pasion/oauth2/token").unwrap())
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", pending.binding.client_id.as_str()),
                ("redirect_uri", pending.redirect.as_str()),
                ("code", code.as_str()),
                ("code_verifier", pending.verifier.as_str()),
            ])
            .send()
            .await
            .map_err(|_| "server_unavailable")?,
    )
    .await?;
    let token = tokens["access_token"]
        .as_str()
        .filter(|t| !t.is_empty() && t.len() <= 4096)
        .ok_or("invalid_server_response")?;
    let identity = get(
        &client,
        server.join("/_hagency/client/v1/identity").unwrap(),
        Some(token),
    )
    .await?;
    let user = identity["userId"]
        .as_str()
        .ok_or("invalid_server_response")?;
    let subject = identity["subject"]
        .as_str()
        .ok_or("invalid_server_response")?;
    if identity["clientId"] != pending.binding.client_id
        || pending
            .binding
            .owner
            .as_ref()
            .is_some_and(|owner| owner != user)
        || pending
            .binding
            .subject
            .as_ref()
            .is_some_and(|owner| owner != subject)
    {
        return Err("owner_mismatch");
    }
    pending.binding.owner = Some(user.to_owned());
    pending.binding.subject = Some(subject.to_owned());
    login.save(pending.binding.clone()).await?;
    let expiry = tokens["expires_in"].as_u64().unwrap_or(900).min(900);
    if expiry == 0 {
        return Err("sign_in_required");
    }
    let session = console
        .0
        .authority
        .server_session(Duration::from_secs(expiry))
        .map_err(|_| "local_state_unavailable")?;
    let mut sessions = login.sessions.lock().await;
    sessions.retain(|_, s| s.expires > Instant::now());
    if sessions.len() >= 64 {
        return Err("local_session_limit");
    }
    sessions.insert(
        URL_SAFE_NO_PAD.encode(Sha256::digest(session.as_bytes())),
        RemoteSession {
            token: token.to_owned(),
            binding: pending.binding.clone(),
            checked: Instant::now(),
            expires: Instant::now() + Duration::from_secs(expiry),
        },
    );
    drop(sessions);
    // A successful identity login remains usable locally even if enrollment is
    // refused by policy or temporarily unavailable; never fake connectedness.
    let result = enroll(&client, &server, token, &pending.binding, depot).await;
    let status = match result {
        Ok(value) => value,
        Err(code) => json!({"state":"failed","code":code}),
    };
    Ok((session, status))
}
async fn enroll(
    client: &Client,
    server: &Url,
    token: &str,
    binding: &Binding,
    depot: &Depot,
) -> Result<Value, &'static str> {
    let app = depot
        .get_typed::<crate::App>()
        .map_err(|_| "local_state_unavailable")?;
    let live = app.palpo_live().ok_or("transport_unavailable")?;
    let enrolled = response(
        client
            .post(server.join("/_hagency/client/v1/fleets").unwrap())
            .bearer_auth(token)
            .json(&json!({"installationId":binding.installation_id,"name":binding.name}))
            .send()
            .await
            .map_err(|_| "server_unavailable")?,
    )
    .await?;
    if enrolled["identity"]["userId"] != binding.owner.as_deref().unwrap()
        || enrolled["identity"]["subject"] != binding.subject.as_deref().unwrap()
        || enrolled["homeserver"] != server.as_str().trim_end_matches('/')
            && enrolled["homeserver"] != server.as_str()
    {
        return Err("invalid_server_response");
    }
    let config =
        serde_json::to_string(&enrolled["configuration"]).map_err(|_| "invalid_server_response")?;
    // Validate returned custody before writing it: the machine endpoint must
    // belong to the selected server, not an arbitrary URL from the response.
    let endpoint = enrolled["configuration"]["transport"]["url"]
        .as_str()
        .ok_or("invalid_server_response")?;
    if Url::parse(endpoint)
        .map_err(|_| "invalid_server_response")?
        .origin()
        != server.origin()
    {
        return Err("invalid_server_response");
    }
    let imported = live
        .import(&config, server.as_str())
        .await
        .map_err(|_| "configuration_import_failed")?;
    if !imported.started {
        return Ok(json!({"state":"saved","fleetId":imported.imported.fleet_id,"started":false}));
    }
    let id = imported.imported.fleet_id;
    let url = server
        .join(&format!("/_hagency/client/v1/fleets/{id}/connect"))
        .unwrap();
    for _ in 0..3 {
        match client
            .post(url.clone())
            .bearer_auth(token)
            .json(&json!({}))
            .send()
            .await
        {
            Ok(reply) if reply.status().is_success() => {
                let status = response(reply).await?;
                if status["fleet"]["readiness"]["ready"] == true {
                    return Ok(json!({"state":"connected","fleetId":id,"started":true}));
                }
            }
            _ => {}
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let console = console(depot)
        .map_err(|_| "local_state_unavailable")?
        .clone();
    let client = client.clone();
    let token = token.to_owned();
    let background_id = id.clone();
    tokio::spawn(async move {
        for _ in 0..60 {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let current = console.0.server_login.status.lock().await.clone();
            if current["state"] != "verifying" || current["fleetId"] != background_id {
                return;
            }
            if let Ok(reply) = client
                .post(url.clone())
                .bearer_auth(&token)
                .json(&json!({}))
                .send()
                .await
            {
                if reply.status() == StatusCode::UNAUTHORIZED {
                    break;
                }
                if let Ok(result) = response(reply).await
                    && result["fleet"]["readiness"]["ready"] == true
                {
                    *console.0.server_login.status.lock().await =
                        json!({"state":"connected","fleetId":background_id,"started":true});
                    return;
                }
            }
        }
        let mut state = console.0.server_login.status.lock().await;
        if state["state"] == "verifying" && state["fleetId"] == background_id {
            *state = json!({"state":"failed","code":"verification_timeout"});
        }
    });
    Ok(json!({"state":"verifying","fleetId":id,"started":true}))
}
