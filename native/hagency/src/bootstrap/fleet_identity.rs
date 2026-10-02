//! ADR-187 B: an imported Palpo fleet's own accounts and local keys, created
//! by Hagency through the fleet's App Service — no rig script, no owner
//! password.
//!
//! - The approval bot `@<fleet>_approval` and the representative
//!   `@<fleet>_representative` each get one device by App Service login
//!   (Palpo creates a namespace user when the App Service first acts as it,
//!   as TS does). The device is created once and reused on every later run;
//!   a stored credential the homeserver no longer accepts is refused, never
//!   silently replaced, because room custody and the approval SDK store are
//!   bound to that device.
//! - `matrix.appservice_token` carries the App Service token the provisioning
//!   host acts with; `approval.sdk_key` and `matrix.provisioning_key` are
//!   random keys minted once and never sent anywhere.
//!
//! The files keep the names the existing driver configuration reads, so the
//! fleet service uses the same readers.
use hagency_store::private;
use reqwest::{StatusCode, Url, redirect::Policy};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Error {
    #[error("palpo-appservice.json is missing or malformed")]
    Appservice,
    #[error("the homeserver could not be reached")]
    Unreachable,
    #[error("the App Service could not act as {0}")]
    Refused(String),
    #[error("the stored {0} credential is no longer accepted; the operator must re-create it")]
    Revoked(&'static str),
    #[error("the state directory refused a write")]
    Store,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Device {
    pub(crate) user_id: String,
    pub(crate) device_id: String,
}
#[derive(Debug)]
pub(crate) struct Identities {
    pub(crate) approval: Device,
    pub(crate) representative: Device,
}

struct Client {
    http: reqwest::Client,
    origin: Url,
    as_token: String,
    server_name: String,
}
impl Client {
    async fn send(
        &self,
        method: reqwest::Method,
        path: &[&str],
        user: Option<&str>,
        token: &str,
        body: Option<Value>,
    ) -> Result<(StatusCode, Value), Error> {
        let mut url = self.origin.clone();
        url.path_segments_mut().map_err(|_| Error::Appservice)?.extend(path);
        if let Some(user) = user {
            url.query_pairs_mut().append_pair("user_id", user);
        }
        let mut request = self.http.request(method, url).bearer_auth(token);
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(serde_json::to_vec(&body).map_err(|_| Error::Appservice)?);
        }
        let response = request.send().await.map_err(|_| Error::Unreachable)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| Error::Unreachable)?;
        if bytes.len() > 64 * 1024 {
            return Err(Error::Unreachable);
        }
        Ok((status, serde_json::from_slice(&bytes).unwrap_or(Value::Null)))
    }
    async fn whoami(&self, token: &str, user: Option<&str>) -> Result<(StatusCode, Value), Error> {
        self.send(reqwest::Method::GET, &["_matrix", "client", "v3", "account", "whoami"], user, token, None)
            .await
    }
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// One namespace user's device: reuse the stored one, or create it once.
async fn device(client: &Client, state: &Path, name: &'static str, token_file: &str, localpart: &str) -> Result<Device, Error> {
    let user = format!("@{localpart}:{}", client.server_name);
    let identity = state.join(format!("{name}.identity.json"));
    let token_path = state.join(token_file);
    if let (Ok(raw), Ok(token)) = (private::read_secret(&identity), private::read_secret(&token_path)) {
        let stored: Value = serde_json::from_slice(&raw).map_err(|_| Error::Store)?;
        let token = String::from_utf8(token).map_err(|_| Error::Store)?;
        let (status, who) = client.whoami(token.trim(), None).await?;
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(Error::Revoked(name));
        }
        if !status.is_success() {
            return Err(Error::Unreachable);
        }
        let device = Device {
            user_id: text(&stored, "user_id").ok_or(Error::Store)?,
            device_id: text(&stored, "device_id").ok_or(Error::Store)?,
        };
        if text(&who, "user_id").as_deref() != Some(device.user_id.as_str()) || device.user_id != user {
            return Err(Error::Revoked(name));
        }
        return Ok(device);
    }
    // Acting as the user creates it on Palpo (TS `mintAgentIdentity`), then
    // App Service login gives it a device of its own.
    let (status, who) = client.whoami(&client.as_token, Some(&user)).await?;
    if !status.is_success() || text(&who, "user_id").as_deref() != Some(user.as_str()) {
        return Err(Error::Refused(user));
    }
    let (status, login) = client
        .send(
            reqwest::Method::POST,
            &["_matrix", "client", "v3", "login"],
            None,
            &client.as_token,
            Some(json!({
                "type": "m.login.application_service",
                "identifier": {"type": "m.id.user", "user": localpart},
                "initial_device_display_name": format!("Hagency {name}"),
            })),
        )
        .await?;
    let (Some(token), Some(user_id), Some(device_id)) =
        (text(&login, "access_token"), text(&login, "user_id"), text(&login, "device_id"))
    else {
        return Err(Error::Refused(user));
    };
    if !status.is_success() || user_id != user {
        return Err(Error::Refused(user));
    }
    // Token before identity: a crash between them leaves no identity file,
    // so the next run logs in again instead of trusting a half-written pair.
    private::replace(&token_path, token.as_bytes()).map_err(|_| Error::Store)?;
    let identity_value = json!({"user_id": user_id, "device_id": device_id});
    private::replace(&identity, identity_value.to_string().as_bytes()).map_err(|_| Error::Store)?;
    Ok(Device { user_id, device_id })
}

/// A 32-byte private key, created once.
fn key(state: &Path, name: &str) -> Result<(), Error> {
    let path = state.join(name);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes).map_err(|_| Error::Store)?;
            private::write_new(&path, &bytes).map_err(|_| Error::Store)
        }
        Err(_) => Err(Error::Store),
    }
}

/// Ensure the fleet's identities and keys exist, from the imported files.
pub(crate) async fn ensure(state: &Path, fleet_id: &str, server_name: &str) -> Result<Identities, Error> {
    let raw = private::read_secret(&state.join("palpo-appservice.json")).map_err(|_| Error::Appservice)?;
    let appservice: Value = serde_json::from_slice(&raw).map_err(|_| Error::Appservice)?;
    let homeserver = text(&appservice, "homeserver").ok_or(Error::Appservice)?;
    let as_token = text(&appservice, "as_token").ok_or(Error::Appservice)?;
    let client = Client {
        http: reqwest::Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| Error::Unreachable)?,
        origin: Url::parse(&homeserver).map_err(|_| Error::Appservice)?,
        as_token: as_token.clone(),
        server_name: server_name.to_owned(),
    };
    let representative = device(
        &client,
        state,
        "representative",
        "matrix.representative_token",
        &format!("{fleet_id}_representative"),
    )
    .await?;
    let approval = device(&client, state, "approval", "approval.access_token", &format!("{fleet_id}_approval")).await?;
    private::replace(&state.join("matrix.appservice_token"), as_token.as_bytes()).map_err(|_| Error::Store)?;
    key(state, "approval.sdk_key")?;
    key(state, "matrix.provisioning_key")?;
    Ok(Identities { approval, representative })
}

/// ADR-187 §C: the owner's master key as the homeserver reports it now, read
/// with the representative's device. `None` when the owner has no
/// cross-signing yet: that is a wait, never "no anchor needed".
pub(crate) async fn fetch_master_key(state: &Path, owner: &str) -> Result<Option<String>, Error> {
    let raw = private::read_secret(&state.join("palpo-appservice.json")).map_err(|_| Error::Appservice)?;
    let appservice: Value = serde_json::from_slice(&raw).map_err(|_| Error::Appservice)?;
    let homeserver = text(&appservice, "homeserver").ok_or(Error::Appservice)?;
    let token = String::from_utf8(
        private::read_secret(&state.join("matrix.representative_token")).map_err(|_| Error::Appservice)?,
    )
    .map_err(|_| Error::Store)?;
    let client = Client {
        http: reqwest::Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| Error::Unreachable)?,
        origin: Url::parse(&homeserver).map_err(|_| Error::Appservice)?,
        as_token: String::new(),
        server_name: String::new(),
    };
    let (status, value) = client
        .send(
            reqwest::Method::POST,
            &["_matrix", "client", "v3", "keys", "query"],
            None,
            token.trim(),
            Some(json!({"device_keys": {owner: []}})),
        )
        .await?;
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(Error::Revoked("representative"));
    }
    if !status.is_success() {
        return Err(Error::Unreachable);
    }
    let keys: Vec<String> = value
        .pointer(&format!("/master_keys/{}/keys", owner.replace('~', "~0").replace('/', "~1")))
        .and_then(Value::as_object)
        .map(|keys| keys.values().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    match keys.as_slice() {
        [] => Ok(None),
        [key] => Ok(Some(key.clone())),
        _ => Err(Error::Refused(owner.to_owned())),
    }
}

/// ADR-187 §C: the anchor to trust for `owner`. A pinned anchor is returned
/// as pinned (a later change is caught by enrollment's own key check); with
/// none pinned, the key the homeserver reports now is pinned on first use.
pub(crate) async fn owner_anchor(
    domain: &hagency_store::DomainStore,
    state: &Path,
    owner: &str,
    now: u64,
) -> Result<Option<String>, Error> {
    if let Some(pinned) = domain.owner_anchor(owner.to_owned()).await.map_err(|_| Error::Store)? {
        return Ok(Some(pinned.master_key));
    }
    let Some(key) = fetch_master_key(state, owner).await? else {
        return Ok(None);
    };
    let pinned = domain
        .observe_owner_anchor(owner.to_owned(), key, now)
        .await
        .map_err(|_| Error::Store)?;
    Ok(Some(pinned.master_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    const FLEET: &str = "hf_0123456789abcdef0123456789abcdef";

    /// A minimal homeserver: masquerade whoami creates the user, App Service
    /// login hands out a device, and a stored token answers whoami until it
    /// is revoked.
    async fn homeserver(revoked: Arc<Mutex<bool>>) -> (String, Arc<Mutex<u32>>) {
        let logins = Arc::new(Mutex::new(0u32));
        let seen = logins.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else { return };
                let seen = seen.clone();
                let revoked = revoked.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buffer = vec![0u8; 8192];
                    let n = socket.read(&mut buffer).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let line = request.lines().next().unwrap_or_default().to_owned();
                    let auth = request
                        .lines()
                        .find_map(|l| l.strip_prefix("authorization: Bearer "))
                        .unwrap_or_default()
                        .trim()
                        .to_owned();
                    let (status, body) = if line.starts_with("POST /_matrix/client/v3/login") {
                        let mut n = seen.lock().unwrap();
                        *n += 1;
                        let local = if request.contains("_approval") { "approval" } else { "representative" };
                        (200, json!({"access_token": format!("tok-{local}-{n}"),
                            "user_id": format!("@{FLEET}_{local}:example.test"), "device_id": format!("DEV{local}{n}")}))
                    } else if line.contains("/whoami?user_id=") {
                        let user = urlencoding(&line);
                        (200, json!({"user_id": user}))
                    } else if line.contains("/whoami") {
                        if *revoked.lock().unwrap() || !auth.starts_with("tok-") {
                            (401, json!({"errcode": "M_UNKNOWN_TOKEN"}))
                        } else {
                            let local = if auth.contains("approval") { "approval" } else { "representative" };
                            (200, json!({"user_id": format!("@{FLEET}_{local}:example.test")}))
                        }
                    } else if line.starts_with("POST /_matrix/client/v3/keys/query") {
                        if request.contains("@nokey:") {
                            (200, json!({"master_keys": {}}))
                        } else {
                            (200, json!({"master_keys": {"@owner:example.test": {"keys": {"ed25519:K": "K".repeat(43)}}}}))
                        }
                    } else {
                        (404, json!({}))
                    };
                    let body = body.to_string();
                    let response = format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });
        (format!("http://{address}"), logins)
    }
    fn urlencoding(line: &str) -> String {
        let raw = line.split("user_id=").nth(1).unwrap_or_default().split(' ').next().unwrap_or_default();
        raw.replace("%40", "@").replace("%3A", ":")
    }
    fn state(homeserver: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        private::replace(
            &dir.path().join("palpo-appservice.json"),
            json!({"homeserver": homeserver, "as_token": "as-secret"}).to_string().as_bytes(),
        )
        .unwrap();
        dir
    }

    #[tokio::test]
    async fn native_fleet_identity_creates_each_device_once() {
        let revoked = Arc::new(Mutex::new(false));
        let (origin, logins) = homeserver(revoked.clone()).await;
        let dir = state(&origin);
        let first = ensure(dir.path(), FLEET, "example.test").await.unwrap();
        assert_eq!(first.approval.user_id, format!("@{FLEET}_approval:example.test"));
        assert_eq!(first.representative.user_id, format!("@{FLEET}_representative:example.test"));
        assert_eq!(*logins.lock().unwrap(), 2);
        for file in ["approval.access_token", "matrix.representative_token", "matrix.appservice_token",
            "approval.sdk_key", "matrix.provisioning_key"] {
            assert!(dir.path().join(file).exists(), "{file}");
        }
        let key = std::fs::read(dir.path().join("matrix.provisioning_key")).unwrap();
        assert_eq!(key.len(), 32);
        // A second run reuses both devices and both keys.
        let again = ensure(dir.path(), FLEET, "example.test").await.unwrap();
        assert_eq!(again.approval, first.approval);
        assert_eq!(again.representative, first.representative);
        assert_eq!(*logins.lock().unwrap(), 2, "no second login");
        assert_eq!(std::fs::read(dir.path().join("matrix.provisioning_key")).unwrap(), key);
        // A revoked stored credential is refused, never silently replaced.
        *revoked.lock().unwrap() = true;
        assert_eq!(
            ensure(dir.path(), FLEET, "example.test").await.unwrap_err(),
            Error::Revoked("representative")
        );
        assert_eq!(*logins.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn native_fleet_identity_reads_the_owner_master_key() {
        let (origin, _) = homeserver(Arc::new(Mutex::new(false))).await;
        let dir = state(&origin);
        ensure(dir.path(), FLEET, "example.test").await.unwrap();
        assert_eq!(
            fetch_master_key(dir.path(), "@owner:example.test").await.unwrap(),
            Some("K".repeat(43))
        );
        assert_eq!(fetch_master_key(dir.path(), "@nokey:example.test").await.unwrap(), None);
    }
}
