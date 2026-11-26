//! Direct, operator-authorized remote bindings for the native owner runtime.
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use hagency_agent_local::inbox::Prepared;
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use super::RuntimeError;

const UNAVAILABLE: RuntimeError = RuntimeError::Execution("remote_runtime_unavailable");
const PENDING: RuntimeError = RuntimeError::Execution("remote_runtime_pending_reconciliation");
const INCOMPATIBLE: RuntimeError = RuntimeError::Execution("remote_runtime_incompatible");

#[derive(Clone)]
pub(in crate::console) struct RemoteBinding {
    settings: Settings,
    settings_path: PathBuf,
    endpoint: Url,
    client: Client,
    receipts: PathBuf,
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Settings {
    endpoint: String,
    binding_id: String,
    token: String,
    provider: String,
    model: String,
    #[serde(default)]
    allow_encrypted_owner_direct: bool,
}

#[derive(Deserialize)]
struct Capabilities {
    protocol_version: u8,
    binding_id: String,
    agent_id: String,
    owner: String,
    provider: String,
    model: String,
    active: bool,
    available: bool,
    scopes: Vec<String>,
}

#[derive(Serialize)]
struct Submission<'a> {
    binding_id: &'a str,
    agent_id: &'a str,
    caller: &'a str,
    scope: DirectScope<'a>,
    provider: &'a str,
    model: &'a str,
    prompt: &'a str,
}

#[derive(Serialize)]
struct DirectScope<'a> {
    kind: &'static str,
    room_id: &'a str,
    thread_id: &'a str,
}

#[derive(Deserialize)]
struct Run {
    id: String,
    binding_id: String,
    status: String,
    answer: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Receipt {
    run_id: String,
    binding_id: String,
    agent_id: String,
    dispatch_id: String,
    endpoint: String,
}

pub(super) fn pending_receipt(profile: &Path, agent: &str) -> Option<Receipt> {
    if !safe_id(agent) {
        return None;
    }
    let directory = profile.with_file_name("remote-runtime-runs").join(agent);
    for entry in std::fs::read_dir(directory).ok()?.take(128).flatten() {
        let Ok(file) = hagency_store::private::open(&entry.path(), false) else {
            continue;
        };
        let mut bytes = Vec::new();
        if file.take(16_384).read_to_end(&mut bytes).is_err() {
            continue;
        }
        let Ok(receipt) = serde_json::from_slice::<Receipt>(&bytes) else {
            continue;
        };
        if receipt.agent_id == agent
            && safe_id(&receipt.run_id)
            && valid_binding_id(&receipt.binding_id)
            && parse_endpoint(&receipt.endpoint).is_ok()
        {
            return Some(receipt);
        }
    }
    None
}

#[cfg_attr(test, derive(Debug))]
pub(super) enum RemoteOutcome {
    Succeeded(String),
    Rejected,
    Failed,
}

async fn decode<T: DeserializeOwned>(mut response: Response, size: usize) -> Option<T> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len().saturating_add(chunk.len()) > size {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).ok()
}

pub(super) async fn verify_matrix_transport(origin: &str) -> Result<(), RuntimeError> {
    let matrix = crate::server_admission::discover(origin)
        .await
        .map_err(|_| RuntimeError::Transport)?;
    if !matrix
        .capabilities
        .iter()
        .any(|cap| cap == "agent-encrypted-inbox-v1")
    {
        return Err(RuntimeError::Execution(
            "remote_runtime_encrypted_inbox_required",
        ));
    }
    Ok(())
}

pub(in crate::console) fn load(
    ledger_path: &Path,
    agent: &str,
) -> Result<Option<RemoteBinding>, RuntimeError> {
    if !safe_id(agent) {
        return Err(INCOMPATIBLE);
    }
    let path = ledger_path
        .with_file_name("runtime-bindings")
        .join(format!("{agent}.json"));
    let Some(settings) = read_settings(&path)? else {
        return Ok(None);
    };
    let endpoint = parse_endpoint(&settings.endpoint)?;
    if !matches!(settings.provider.as_str(), "codex" | "claude")
        || settings.model.is_empty()
        || settings.model.len() > 128
        || settings.model.chars().any(char::is_control)
        || !valid_binding_id(&settings.binding_id)
        || settings.token.len() != 64
        || !settings.token.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(INCOMPATIBLE);
    }
    let client = Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| UNAVAILABLE)?;
    Ok(Some(RemoteBinding {
        settings,
        settings_path: path,
        endpoint,
        client,
        receipts: ledger_path
            .with_file_name("remote-runtime-runs")
            .join(agent),
    }))
}

fn read_settings(path: &Path) -> Result<Option<Settings>, RuntimeError> {
    match path.symlink_metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(INCOMPATIBLE),
        Ok(meta) if !meta.is_file() => return Err(INCOMPATIBLE),
        Ok(_) => {}
    }
    let file = hagency_store::private::open(path, false).map_err(|_| INCOMPATIBLE)?;
    let mut bytes = Vec::new();
    file.take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| INCOMPATIBLE)?;
    if bytes.len() > 8192 {
        return Err(INCOMPATIBLE);
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| INCOMPATIBLE)
}

fn parse_endpoint(value: &str) -> Result<Url, RuntimeError> {
    let url = Url::parse(value).map_err(|_| INCOMPATIBLE)?;
    if url.scheme() != "https"
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || ch == '\\' || ch.is_control())
    {
        return Err(INCOMPATIBLE);
    }
    Ok(url)
}

fn safe_id(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
}

fn valid_binding_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}

impl RemoteBinding {
    pub(super) fn endpoint(&self) -> &str {
        self.endpoint.as_str()
    }

    pub(in crate::console) fn provider(&self) -> &str {
        &self.settings.provider
    }

    pub(in crate::console) fn model(&self) -> &str {
        &self.settings.model
    }

    pub(super) fn allows_encrypted(&self) -> bool {
        self.settings.allow_encrypted_owner_direct
    }

    pub(super) fn consent(&self) -> String {
        let identity = serde_json::json!([
            self.endpoint.as_str(),
            self.settings.binding_id,
            self.settings.provider,
            self.settings.model,
            self.settings.token,
            self.settings.allow_encrypted_owner_direct,
        ]);
        format!("{:x}", Sha256::digest(identity.to_string()))
    }

    fn check_settings(&self) -> Result<(), RuntimeError> {
        if read_settings(&self.settings_path)?.as_ref() != Some(&self.settings) {
            return Err(RuntimeError::Execution("remote_runtime_binding_changed"));
        }
        Ok(())
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(
                method,
                self.endpoint.join(path).expect("fixed runtime route"),
            )
            .bearer_auth(&self.settings.token)
    }

    pub(super) async fn verify(&self, owner: &str, agent: &str) -> Result<(), RuntimeError> {
        self.check_settings()?;
        if !self.allows_encrypted() {
            return Err(RuntimeError::Execution("remote_runtime_consent_required"));
        }
        let response = self
            .request(Method::GET, "v1/runtime")
            .send()
            .await
            .map_err(|_| UNAVAILABLE)?;
        if matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            return Err(RuntimeError::Execution("remote_runtime_entitlement_denied"));
        }
        if !response.status().is_success() {
            return Err(UNAVAILABLE);
        }
        let caps: Capabilities = decode(response, 8192).await.ok_or(INCOMPATIBLE)?;
        if caps.protocol_version != 1
            || caps.binding_id != self.settings.binding_id
            || caps.owner != owner
            || caps.agent_id != agent
            || caps.provider != self.settings.provider
            || caps.model != self.settings.model
            || !caps.scopes.iter().any(|scope| scope == "owner_direct")
        {
            return Err(INCOMPATIBLE);
        }
        if !caps.active {
            return Err(RuntimeError::Execution("remote_runtime_entitlement_denied"));
        }
        if !caps.available {
            return Err(UNAVAILABLE);
        }
        Ok(())
    }

    // The existing ledger already owns the dispatch and execution ID. Save the
    // immutable runtime route before the network request, without prompt or token.
    pub(super) fn record(&self, prepared: &Prepared) -> Result<(), RuntimeError> {
        self.check_settings()?;
        if !safe_id(&prepared.execution_id) {
            return Err(INCOMPATIBLE);
        }
        let parent = self.receipts.parent().ok_or(RuntimeError::Profile)?;
        hagency_store::private::directory(parent).map_err(|_| RuntimeError::Profile)?;
        hagency_store::private::directory(&self.receipts).map_err(|_| RuntimeError::Profile)?;
        let path = self.receipt_path(&prepared.execution_id);
        let receipt = Receipt {
            run_id: prepared.execution_id.clone(),
            binding_id: self.settings.binding_id.clone(),
            agent_id: prepared.dispatch.agent_id.clone(),
            dispatch_id: prepared.dispatch.id.clone(),
            endpoint: self.endpoint.as_str().into(),
        };
        let data = serde_json::to_vec_pretty(&receipt).map_err(|_| RuntimeError::Profile)?;
        hagency_store::private::write_new(&path, &data).map_err(|_| RuntimeError::Profile)
    }

    fn receipt_path(&self, id: &str) -> PathBuf {
        self.receipts.join(format!("{id}.json"))
    }

    pub(super) fn resolved(&self, id: &str) {
        if let Err(error) = std::fs::remove_file(self.receipt_path(id))
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, run_id = id, "Remote run receipt could not be removed");
        }
    }

    pub(super) async fn run(&self, prepared: &Prepared) -> Result<RemoteOutcome, RuntimeError> {
        let dispatch = &prepared.dispatch;
        let id = &prepared.execution_id;
        let path = format!("v1/runs/{id}");
        let input = Submission {
            binding_id: &self.settings.binding_id,
            agent_id: &dispatch.agent_id,
            caller: &dispatch.requester_mxid,
            scope: DirectScope {
                kind: "owner_direct",
                room_id: &dispatch.room_id,
                thread_id: &dispatch.thread_root,
            },
            provider: &self.settings.provider,
            model: &self.settings.model,
            prompt: &dispatch.body,
        };
        let mut submit = true;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3660);
        while tokio::time::Instant::now() < deadline {
            let request = if submit {
                self.check_settings()?;
                self.request(Method::PUT, &path).json(&input)
            } else {
                self.request(Method::GET, &path)
            };
            let sent_submission = submit;
            submit = false;
            if let Ok(response) = request.send().await {
                match response.status() {
                    StatusCode::NOT_FOUND if !sent_submission => submit = true,
                    StatusCode::FORBIDDEN
                    | StatusCode::UNAUTHORIZED
                    | StatusCode::TOO_MANY_REQUESTS
                        if sent_submission =>
                    {
                        return Ok(RemoteOutcome::Rejected);
                    }
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(PENDING),
                    StatusCode::CONFLICT | StatusCode::BAD_REQUEST if sent_submission => {
                        return Err(INCOMPATIBLE);
                    }
                    status if status.is_success() => {
                        let record: Run = decode(response, 400_000).await.ok_or(PENDING)?;
                        if record.id != *id || record.binding_id != self.settings.binding_id {
                            return Err(PENDING);
                        }
                        match record.status.as_str() {
                            "succeeded" => {
                                return record
                                    .answer
                                    .filter(|text| !text.is_empty() && text.len() <= 65_536)
                                    .map(RemoteOutcome::Succeeded)
                                    .ok_or(PENDING);
                            }
                            "rejected" => return Ok(RemoteOutcome::Rejected),
                            "canceled" | "failed" => return Ok(RemoteOutcome::Failed),
                            "pending_reconciliation" => return Err(PENDING),
                            "queued" | "running" | "cancel_requested" => {}
                            _ => return Err(PENDING),
                        }
                    }
                    _ => {}
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Err(PENDING)
    }

    pub(super) async fn cancel(&self, id: &str) {
        let _ = self
            .request(Method::POST, &format!("v1/runs/{id}/cancel"))
            .send()
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_https_origins() {
        assert!(parse_endpoint("https://runtime.example.test:8443").is_ok());
        for url in [
            "http://localhost:8077",
            "https://user:pass@host",
            "https://host/base",
            "https://host?agent=x",
            "https://host/#secret",
        ] {
            assert!(parse_endpoint(url).is_err(), "{url}");
        }
    }

    #[tokio::test]
    async fn private_binding_checks_consent_and_submits_only_the_original_turn_over_https() {
        use crate::file_service::test_common::{Fake, scripted};
        use hagency_agent_local::inbox::Dispatch;
        use serde_json::{Value, json};

        let owner = "@alice:example.test";
        let agent = "agt_example";
        let binding = "90b2d181-a015-46b1-9fbf-7b9847f0e98c";
        let token = "a".repeat(64);
        let root = tempfile::tempdir().unwrap();
        let ledger_path = root.path().join("agent-local.sqlite");
        let settings_dir = root.path().join("runtime-bindings");
        hagency_store::private::directory(&settings_dir).unwrap();
        let settings_path = settings_dir.join(format!("{agent}.json"));
        let mut fake = Fake::start(true).await;
        let mut settings = json!({
            "endpoint":fake.endpoint,"binding_id":binding,"token":token,
            "provider":"codex","model":"fixture-model",
        });
        let write = |value: &Value| {
            hagency_store::private::replace(&settings_path, &serde_json::to_vec(value).unwrap())
                .unwrap();
        };
        write(&settings);
        let initial = load(&ledger_path, agent).unwrap().unwrap();
        assert!(!initial.allows_encrypted());
        assert!(matches!(
            initial.verify(owner, agent).await,
            Err(RuntimeError::Execution("remote_runtime_consent_required"))
        ));

        settings["allow_encrypted_owner_direct"] = true.into();
        write(&settings);
        let mut remote = load(&ledger_path, agent).unwrap().unwrap();
        assert_ne!(initial.consent(), remote.consent());
        remote.client = Client::builder()
            .no_proxy()
            .add_root_certificate(
                reqwest::Certificate::from_pem(include_bytes!(
                    "../../../../hagency-matrix/tests/fixtures/ca.pem"
                ))
                .unwrap(),
            )
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let prepared = Prepared {
            execution_id: "exec_original".into(),
            dispatch: Dispatch {
                id: "dispatch_original".into(),
                binding_id: "bnd_local".into(),
                agent_id: agent.into(),
                event_id: "$original".into(),
                room_id: "!private:example.test".into(),
                requester_mxid: owner.into(),
                thread_root: "!private:example.test".into(),
                body: "original private text".into(),
                state: "running".into(),
                binding_generation: 1,
                dispatch_epoch: Some(1),
                dispatch_device_id: Some("dev_example".into()),
                execution_id: Some("exec_original".into()),
                outcome: None,
            },
        };
        let authorization = format!("Bearer {token}");
        let (answer, ()) = scripted(async {
            remote.verify(owner, agent).await?;
            remote.record(&prepared)?;
            let receipt = serde_json::to_value(pending_receipt(&ledger_path, agent).unwrap()).unwrap();
            assert_eq!(receipt["run_id"], prepared.execution_id);
            assert!(receipt.get("token").is_none() && receipt.get("prompt").is_none());
            remote.run(&prepared).await
        }, async {
            let caps = fake.next().await;
            assert_eq!((&*caps.method, &*caps.target), ("GET", "/v1/runtime"));
            assert_eq!(caps.headers["authorization"], authorization);
            caps.json(200, json!({"protocol_version":1,"binding_id":binding,"agent_id":agent,
              "owner":owner,"provider":"codex","model":"fixture-model","active":true,
              "available":true,"scopes":["owner_direct"]}));
            let submit = fake.next().await;
            assert_eq!((&*submit.method, &*submit.target), ("PUT", "/v1/runs/exec_original"));
            assert_eq!(submit.headers["authorization"], authorization);
            assert_eq!(serde_json::from_slice::<Value>(&submit.body).unwrap(), json!({
                "binding_id":binding,"agent_id":agent,"caller":owner,"provider":"codex","model":"fixture-model",
                "scope":{"kind":"owner_direct","room_id":"!private:example.test","thread_id":"!private:example.test"},
                "prompt":"original private text",
            }));
            submit.json(202, json!({"id":"exec_original","binding_id":binding,"status":"queued"}));
            let poll = fake.next().await;
            assert_eq!((&*poll.method, &*poll.target), ("GET", "/v1/runs/exec_original"));
            assert!(poll.body.is_empty());
            poll.json(200, json!({"id":"exec_original","binding_id":binding,"status":"succeeded","answer":"fixture reply"}));
        }).await;
        assert!(
            matches!(answer.unwrap(), RemoteOutcome::Succeeded(text) if text == "fixture reply")
        );
        remote.resolved(&prepared.execution_id);
        assert!(pending_receipt(&ledger_path, agent).is_none());

        settings["allow_encrypted_owner_direct"] = false.into();
        write(&settings);
        assert!(matches!(
            remote.run(&prepared).await,
            Err(RuntimeError::Execution("remote_runtime_binding_changed"))
        ));
    }
}
