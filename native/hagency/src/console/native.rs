//! Token-free native commands routed through the existing authenticated service.
use super::*;
use crate::owner_host::OwnerHost;
use salvo::test::{ResponseExt, TestClient};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex as StdMutex, Weak},
};
use tokio::sync::Mutex;

/// The SDK remains the sole owner of OAuth refresh and revocation.
pub struct MatrixAccessToken {
    pub access_token: String,
    pub client_id: String,
}
impl std::fmt::Debug for MatrixAccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatrixAccessToken")
            .field("access_token", &"[redacted]")
            .field("client_id", &self.client_id)
            .finish()
    }
}
pub(in crate::console) fn sdk_source_failure(error: NativeError) -> &'static str {
    match error.code.as_str() {
        "matrix_account_changed" => "matrix_account_changed",
        "matrix_authorization_unavailable" => "matrix_authorization_unavailable",
        "matrix_oauth_sign_in_required" => "matrix_oauth_sign_in_required",
        "unsupported_hagency_issuer" => "unsupported_hagency_issuer",
        "matrix_account_mismatch" => "matrix_account_mismatch",
        "unsupported_hagency_server" => "unsupported_hagency_server",
        "unsupported_hagency_protocol" => "unsupported_hagency_protocol",
        "unsupported_hagency_capabilities" => "unsupported_hagency_capabilities",
        "invalid_hagency_metadata" => "invalid_hagency_metadata",
        "hagency_server_unavailable" => "hagency_server_unavailable",
        _ => "matrix_authorization_required",
    }
}
pub trait MatrixTokenSource: Send + Sync {
    /// Return the current SDK token, checking its account epoch before and after
    /// awaiting. Implementations must not call this native service recursively.
    fn access_token(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>> + Send + '_>,
    >;
    /// Ask the SDK refresh owner for a fresh grant before a short authorization
    /// window expires. No refresh credential crosses this interface.
    fn refresh_access_token(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>> + Send + '_>,
    > {
        self.access_token()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentAction {
    Pause,
    Resume,
    Retire,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BindingAction {
    Status,
    Pause,
    Resume,
    Leave,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProviderAction {
    UseLocal,
    Disconnect,
    Models,
    Status,
    Login,
    Cancel,
    Logout,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LocalAction {
    AgentModel,
    SaveAgentModel,
    AgentPolicy,
    SaveAgentPolicy,
    ResetAgentPolicy,
    Policy,
    Model,
    SavePolicy,
    ResetPolicy,
    SaveModel,
    Runtime,
    RuntimeStart,
    RuntimeStop,
    Approvals,
    Decision { approval_id: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectCreationPolicy {
    pub default_allow: bool,
    pub allow: std::collections::BTreeSet<String>,
    pub deny: std::collections::BTreeSet<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum RoomCreationPolicy {
    InheritProject {
        deny: std::collections::BTreeSet<String>,
    },
    AllowList {
        allow: std::collections::BTreeSet<String>,
        deny: std::collections::BTreeSet<String>,
    },
    Disabled,
}
impl<'de> Deserialize<'de> for RoomCreationPolicy {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
        enum Strict {
            InheritProject {
                deny: std::collections::BTreeSet<String>,
            },
            AllowList {
                allow: std::collections::BTreeSet<String>,
                deny: std::collections::BTreeSet<String>,
            },
            Disabled {},
        }
        Ok(match Strict::deserialize(d)? {
            Strict::InheritProject { deny } => Self::InheritProject { deny },
            Strict::AllowList { allow, deny } => Self::AllowList { allow, deny },
            Strict::Disabled {} => Self::Disabled,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    AgentCommands,
    ResumeAgentCommand {
        id: String,
    },
    ScopeServiceState {
        project_id: String,
        room_id: Option<String>,
    },
    SetProjectCreationPolicy {
        project_id: String,
        expected_revision: i64,
        policy: ProjectCreationPolicy,
    },
    SetRoomCreationPolicy {
        project_id: String,
        room_id: String,
        expected_revision: i64,
        policy: RoomCreationPolicy,
    },
    SetScopeServicePause {
        project_id: String,
        room_id: Option<String>,
        paused: bool,
    },
    Devices,
    AgentDetails {
        agent_id: String,
    },
    AssignAgentToCurrentDevice {
        agent_id: String,
        expected_generation: i64,
    },
    AgentOwnerDirectStatus {
        agent_id: String,
    },
    EnsureAgentOwnerDirect {
        agent_id: String,
    },
    AgentOwnerDirect {
        agent_id: String,
        room_id: String,
    },

    LoginStatus,
    BeginLogin,
    Logout,
    Projects,
    ProjectRooms {
        project_id: String,
    },
    SpaceCandidates {
        cursor: Option<String>,
    },
    RoomRoster {
        project_id: String,
        room_id: String,
    },
    AdoptProject {
        space_id: String,
    },
    AdoptRoom {
        project_id: String,
        room_id: String,
    },
    CreateMatrix {
        input: Value,
    },
    Creations,
    Creation {
        id: String,
    },
    ResumeCreation {
        id: String,
        room_id: Option<String>,
    },
    Agents,
    Bindings {
        agent_id: String,
    },
    CreateAgent {
        input: Value,
    },
    BindAgent {
        agent_id: String,
        input: Value,
    },
    Agent {
        agent_id: String,
        action: AgentAction,
    },
    Binding {
        agent_id: String,
        binding_id: String,
        action: BindingAction,
    },
    Local {
        agent_id: String,
        action: LocalAction,
        input: Option<Value>,
        binding_id: Option<String>,
        requester: Option<String>,
    },
    PrepareAgentWorkspace {
        agent_id: String,
    },
    Provider {
        action: ProviderAction,
    },
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Identity {
    pub origin: String,
    pub issuer: String,
    pub subject: String,
    pub owner: String,
}
#[derive(Debug, thiserror::Error)]
#[error("{code}")]
pub struct NativeError {
    pub status: u16,
    pub code: String,
}
fn failure(status: u16, code: &str) -> NativeError {
    NativeError {
        status,
        code: code.into(),
    }
}
fn key(id: &str) -> Result<(), NativeError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(failure(400, "invalid_arguments"));
    }
    Ok(())
}
fn segment(id: &str) -> Result<String, NativeError> {
    if id.len() > 255
        || !id.starts_with('!')
        || !id.contains(':')
        || id
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || ['/', '\\', '?', '#', '%'].contains(&c))
    {
        return Err(failure(400, "invalid_arguments"));
    }
    Ok(percent_encoding::utf8_percent_encode(id, percent_encoding::NON_ALPHANUMERIC).to_string())
}
fn project_scope(project: &str, room: Option<&str>) -> Result<String, NativeError> {
    key(project)?;
    match room {
        Some(room) => Ok(format!("{project}/rooms/{}", segment(room)?)),
        None => Ok(project.into()),
    }
}
fn request(command: Command) -> Result<(&'static str, String, Option<Value>), NativeError> {
    let root = "/console/api";
    let mut body = None;
    let (method, path) = match command {
        Command::LoginStatus => ("GET", "/console/server-login".into()),
        Command::Logout => ("POST", "/console/server-login/logout".into()),
        Command::BeginLogin => return Err(failure(400, "login_requires_native_context")),
        Command::Projects => ("GET", format!("{root}/owner-projects")),
        Command::AgentCommands => ("GET", format!("{root}/owned-agents/commands")),
        Command::ResumeAgentCommand { id } => {
            key(&id)?;
            ("POST", format!("{root}/owned-agents/commands/{id}/resume"))
        }
        Command::ScopeServiceState {
            project_id,
            room_id,
        } => {
            let scope = project_scope(&project_id, room_id.as_deref())?;
            (
                "GET",
                format!("{root}/owner-projects/{scope}/service-state"),
            )
        }
        Command::SetProjectCreationPolicy {
            project_id,
            expected_revision,
            policy,
        } => {
            key(&project_id)?;
            if expected_revision < 0 {
                return Err(failure(400, "invalid_arguments"));
            }
            body = Some(json!({"expectedRevision":expected_revision,"policy":policy}));
            (
                "PUT",
                format!("{root}/owner-projects/{project_id}/creation-policy"),
            )
        }
        Command::SetRoomCreationPolicy {
            project_id,
            room_id,
            expected_revision,
            policy,
        } => {
            let scope = project_scope(&project_id, Some(&room_id))?;
            if expected_revision < 0 {
                return Err(failure(400, "invalid_arguments"));
            }
            body = Some(json!({"expectedRevision":expected_revision,"policy":policy}));
            (
                "PUT",
                format!("{root}/owner-projects/{scope}/creation-policy"),
            )
        }
        Command::SetScopeServicePause {
            project_id,
            room_id,
            paused,
        } => {
            let scope = project_scope(&project_id, room_id.as_deref())?;
            let suffix = if paused {
                "pause-service"
            } else {
                "clear-service-pause"
            };
            ("POST", format!("{root}/owner-projects/{scope}/{suffix}"))
        }
        Command::Devices => ("GET", format!("{root}/owned-agents/devices")),
        Command::AgentDetails { agent_id } => {
            key(&agent_id)?;
            ("GET", format!("{root}/owned-agents/{agent_id}"))
        }
        Command::AssignAgentToCurrentDevice {
            agent_id,
            expected_generation,
        } => {
            key(&agent_id)?;
            if expected_generation < 0 {
                return Err(NativeError {
                    status: 400,
                    code: "invalid_arguments".into(),
                });
            }
            body = Some(json!({"expectedGeneration":expected_generation}));
            (
                "PUT",
                format!("{root}/owned-agents/{agent_id}/execution-device"),
            )
        }
        Command::AgentOwnerDirectStatus { agent_id } => {
            key(&agent_id)?;
            (
                "GET",
                format!("{root}/owned-agents/{agent_id}/owner-direct"),
            )
        }
        Command::EnsureAgentOwnerDirect { agent_id } => {
            key(&agent_id)?;
            (
                "POST",
                format!("{root}/owned-agents/{agent_id}/owner-direct/ensure"),
            )
        }
        Command::AgentOwnerDirect { agent_id, room_id } => {
            key(&agent_id)?;
            segment(&room_id)?;
            body = Some(json!({"roomId":room_id}));
            (
                "POST",
                format!("{root}/owned-agents/{agent_id}/owner-direct"),
            )
        }
        Command::ProjectRooms { project_id } => {
            key(&project_id)?;
            ("GET", format!("{root}/owner-projects/{project_id}/rooms"))
        }
        Command::RoomRoster {
            project_id,
            room_id,
        } => {
            key(&project_id)?;
            (
                "GET",
                format!(
                    "{root}/owner-projects/{project_id}/rooms/{}/agents",
                    segment(&room_id)?
                ),
            )
        }
        Command::SpaceCandidates { cursor } => {
            let mut p = format!("{root}/owner-projects/space-candidates");
            if let Some(c) = cursor {
                p.push_str(&format!("?cursor={}", segment(&c)?));
            }
            ("GET", p)
        }
        Command::AdoptProject { space_id } => {
            segment(&space_id)?;
            body = Some(json!({"spaceId":space_id}));
            ("POST", format!("{root}/owner-projects"))
        }
        Command::AdoptRoom {
            project_id,
            room_id,
        } => {
            key(&project_id)?;
            segment(&room_id)?;
            body = Some(json!({"roomId":room_id}));
            ("POST", format!("{root}/owner-projects/{project_id}/rooms"))
        }
        Command::CreateMatrix { input } => {
            body = Some(input);
            ("POST", format!("{root}/matrix-creations"))
        }
        Command::Creations => ("GET", format!("{root}/matrix-creations")),
        Command::Creation { id } => {
            key(&id)?;
            ("GET", format!("{root}/matrix-creations/{id}"))
        }
        Command::ResumeCreation { id, room_id } => {
            key(&id)?;
            if let Some(r) = &room_id {
                segment(r)?;
            }
            body = Some(json!({"roomId":room_id}));
            ("POST", format!("{root}/matrix-creations/{id}/resume"))
        }
        Command::Agents => ("GET", format!("{root}/owned-agents")),
        Command::CreateAgent { input } => {
            body = Some(input);
            ("POST", format!("{root}/owned-agents"))
        }
        Command::Bindings { agent_id } => {
            key(&agent_id)?;
            ("GET", format!("{root}/owned-agents/{agent_id}/bindings"))
        }
        Command::BindAgent { agent_id, input } => {
            key(&agent_id)?;
            body = Some(input);
            ("POST", format!("{root}/owned-agents/{agent_id}/bindings"))
        }
        Command::Agent { agent_id, action } => {
            key(&agent_id)?;
            let (m, s) = match action {
                AgentAction::Pause => ("POST", "/pause"),
                AgentAction::Resume => ("POST", "/resume"),
                AgentAction::Retire => ("DELETE", ""),
            };
            (m, format!("{root}/owned-agents/{agent_id}{s}"))
        }
        Command::Binding {
            agent_id,
            binding_id,
            action,
        } => {
            key(&agent_id)?;
            key(&binding_id)?;
            let (m, s) = match action {
                BindingAction::Status => ("GET", ""),
                BindingAction::Pause => ("POST", "/pause"),
                BindingAction::Resume => ("POST", "/resume"),
                BindingAction::Leave => ("DELETE", ""),
            };
            (
                m,
                format!("{root}/owned-agents/{agent_id}/bindings/{binding_id}{s}"),
            )
        }
        Command::PrepareAgentWorkspace { agent_id } => {
            key(&agent_id)?;
            (
                "POST",
                format!("{root}/owner-provider/workspaces/{agent_id}"),
            )
        }
        Command::Provider { action } => {
            let (m, s) = match action {
                ProviderAction::UseLocal => ("POST", "/use-local"),
                ProviderAction::Disconnect => ("POST", "/disconnect"),
                ProviderAction::Models => ("GET", "/models"),
                ProviderAction::Status => ("GET", ""),
                ProviderAction::Login => ("POST", "/login"),
                ProviderAction::Cancel => ("POST", "/cancel"),
                ProviderAction::Logout => ("POST", "/logout"),
            };
            (m, format!("{root}/owner-provider{s}"))
        }
        Command::Local {
            agent_id,
            action,
            input,
            binding_id,
            requester,
        } => {
            key(&agent_id)?;
            if matches!(
                action,
                LocalAction::AgentModel
                    | LocalAction::SaveAgentModel
                    | LocalAction::AgentPolicy
                    | LocalAction::SaveAgentPolicy
                    | LocalAction::ResetAgentPolicy
            ) && (binding_id.is_some() || requester.is_some())
            {
                return Err(failure(400, "invalid_arguments"));
            }

            body = input;
            let (m, s) = match action {
                LocalAction::AgentModel => ("GET", "agent-model-profile".into()),
                LocalAction::SaveAgentModel => ("PUT", "agent-model-profile".into()),
                LocalAction::AgentPolicy => ("GET", "agent-policy".into()),
                LocalAction::SaveAgentPolicy => ("PUT", "agent-policy".into()),
                LocalAction::ResetAgentPolicy => ("DELETE", "agent-policy".into()),
                LocalAction::Policy => ("GET", "local-policy".into()),
                LocalAction::Model => ("GET", "model-profile".into()),
                LocalAction::SavePolicy => ("PUT", "local-policy".into()),
                LocalAction::ResetPolicy => ("DELETE", "local-policy".into()),
                LocalAction::SaveModel => ("PUT", "model-profile".into()),
                LocalAction::Runtime => ("GET", "runtime".into()),
                LocalAction::RuntimeStart => ("POST", "runtime/start".into()),
                LocalAction::RuntimeStop => ("POST", "runtime/stop".into()),
                LocalAction::Approvals => ("GET", "runtime/approvals".into()),
                LocalAction::Decision { approval_id } => {
                    key(&approval_id)?;
                    ("POST", format!("runtime/approvals/{approval_id}/decision"))
                }
            };
            let mut p = format!("{root}/owned-agents/{agent_id}/{s}");
            if m == "GET" && ["local-policy", "model-profile"].contains(&s.as_str()) {
                let b = binding_id.ok_or(failure(400, "invalid_arguments"))?;
                key(&b)?;
                let r = requester.ok_or(failure(400, "invalid_arguments"))?;
                let mut url = reqwest::Url::parse("http://local.invalid/").unwrap();
                url.query_pairs_mut()
                    .append_pair("bindingId", &b)
                    .append_pair("requester", &r);
                p.push('?');
                p.push_str(url.query().unwrap());
            }
            (m, p)
        }
    };
    Ok((method, path, body))
}
struct Inner {
    host: OwnerHost,
    service: salvo::Service,
    address: SocketAddr,
    expected_origin: String,
    expected_owner: String,
    cookie: Mutex<String>,
    nonce: Mutex<Option<String>>,
    calls: Mutex<()>,
    callback_task: StdMutex<Option<tokio::task::JoinHandle<()>>>,
    matrix_source: StdMutex<Option<Arc<dyn MatrixTokenSource>>>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        self.host.console.retire();
        if let Ok(task) = self.callback_task.get_mut()
            && let Some(task) = task.take()
        {
            task.abort();
        }
    }
}
/// A native owner service. Neither credentials nor its private local grants are
/// returned in command responses. Only the OAuth callback listens on loopback.
#[derive(Clone)]
pub struct NativeOwner(Arc<Inner>);
impl NativeOwner {
    pub async fn open(state: &Path, origin: &str, owner: &str) -> Result<Self, NativeError> {
        let origin = server_login::origin(origin)
            .map_err(|e| failure(400, e))?
            .to_string();
        if !owner.starts_with('@') || !owner.contains(':') || owner.len() > 255 {
            return Err(failure(400, "invalid_owner_identity"));
        }
        let acceptor = salvo::conn::TcpListener::new("127.0.0.1:0")
            .try_bind()
            .await
            .map_err(|_| failure(503, "native_callback_unavailable"))?;
        let address = acceptor
            .local_addr()
            .map_err(|_| failure(503, "native_callback_unavailable"))?;
        let host = OwnerHost::open_native(state, address)
            .map_err(|_| failure(409, "native_owner_state_unavailable"))?;
        *host
            .console
            .0
            .server_login
            .native_expected
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))? =
            Some((origin.clone(), owner.into()));
        let ticket = host
            .console
            .owner_access_ticket()
            .map_err(|_| failure(503, "native_state_unavailable"))?;
        let service = salvo::Service::new(host.clone().router());
        let mut grant = TestClient::post(format!("http://{address}/console/session"))
            .add_header("host", address.to_string(), true)
            .add_header("sec-fetch-site", "same-origin", true)
            .add_header("origin", format!("http://{address}"), true)
            .json(&json!({"ticket":ticket}))
            .send(&service)
            .await;
        if !grant.status_code.unwrap_or_default().is_success() {
            return Err(failure(503, "native_access_unavailable"));
        }
        let cookie = grant
            .headers()
            .get_all("set-cookie")
            .iter()
            .find_map(|v| {
                v.to_str()
                    .ok()
                    .filter(|s| s.starts_with("hagency_console="))
                    .map(|s| s.split(';').next().unwrap().to_owned())
            })
            .ok_or(failure(503, "native_access_unavailable"))?;
        let native = Self(Arc::new(Inner {
            host,
            service,
            address,
            expected_origin: origin,
            expected_owner: owner.into(),
            cookie: Mutex::new(cookie),
            nonce: Mutex::new(None),
            calls: Mutex::new(()),
            callback_task: StdMutex::new(None),
            matrix_source: StdMutex::new(None),
        }));
        let router = Router::new()
            .hoop(Callback(Arc::downgrade(&native.0)))
            .push(Router::with_path("console/server-login/callback").get(callback));
        let task = tokio::spawn(async move {
            salvo::Server::new(acceptor).serve(router).await;
        });
        *native.0.callback_task.lock().unwrap() = Some(task);
        // Consume any bootstrap response without exposing a grant to the caller.
        let _ = grant.take_json::<Value>().await;
        Ok(native)
    }
    pub async fn open_with_matrix(
        state: &Path,
        origin: &str,
        owner: &str,
        source: Arc<dyn MatrixTokenSource>,
    ) -> Result<Self, NativeError> {
        let native = Self::open(state, origin, owner).await?;
        *native
            .0
            .matrix_source
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))? = Some(source.clone());
        match native
            .0
            .host
            .console
            .0
            .server_login
            .authorize_matrix(
                &native.0.host.console,
                &native.0.expected_origin,
                &native.0.expected_owner,
                source,
            )
            .await
        {
            Ok(cookie) => {
                *native.0.cookie.lock().await = format!("hagency_console={cookie}");
                Ok(native)
            }
            Err(e) => {
                native.shutdown().await;
                Err(failure(
                    if e.ends_with("unavailable") { 503 } else { 401 },
                    e,
                ))
            }
        }
    }
    async fn sync_matrix(&self) -> Result<(), NativeError> {
        let source = self
            .0
            .matrix_source
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))?
            .clone();
        if let Some(source) = source {
            let cookie = self.0.cookie.lock().await.clone();
            let raw = cookie
                .strip_prefix("hagency_console=")
                .ok_or(failure(401, "owner_authorization_required"))?;
            let next = match self
                .0
                .host
                .console
                .0
                .server_login
                .renew_matrix(&self.0.host.console, raw)
                .await
            {
                Ok(next) => next,
                Err(_) => Some(
                    self.0
                        .host
                        .console
                        .0
                        .server_login
                        .authorize_matrix(
                            &self.0.host.console,
                            &self.0.expected_origin,
                            &self.0.expected_owner,
                            source.clone(),
                        )
                        .await
                        .map_err(|e| {
                            failure(if e.ends_with("unavailable") { 503 } else { 401 }, e)
                        })?,
                ),
            };
            if let Some(next) = next {
                *self.0.cookie.lock().await = format!("hagency_console={next}");
            }
        }
        Ok(())
    }
    #[cfg(test)]
    pub(in crate::console) fn test_console(&self) -> &super::Console {
        &self.0.host.console
    }
    pub async fn identity(&self) -> Result<Identity, NativeError> {
        let d = self
            .0
            .host
            .console
            .authorized_device()
            .await
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        if d.origin() != self.0.expected_origin || d.owner_mxid() != self.0.expected_owner {
            return Err(failure(401, "matrix_account_mismatch"));
        }
        Ok(Identity {
            origin: d.origin().into(),
            issuer: d.issuer().into(),
            subject: d.subject().into(),
            owner: d.owner_mxid().into(),
        })
    }
    pub fn retire(&self) {
        self.0.host.console.retire();
    }
    pub async fn shutdown(&self) {
        // Fence capabilities synchronously before any awaited cleanup, including
        // a late OAuth exchange or an in-flight runtime/provider launch.
        self.retire();
        self.0.host.console.0.authority.revoke_all().ok();
        let login = &self.0.host.console.0.server_login;
        login.revoke_native_all().await;
        self.0.host.console.stop_owned_runtimes().await;
        self.0.host.console.stop_owner_provider().await;
        login.flush_native_revocations().await;
        self.retire();
        if let Some(task) = self.0.callback_task.lock().unwrap().take() {
            task.abort();
        }
    }
    pub async fn execute(&self, command: Command) -> Result<Value, NativeError> {
        let _guard = self.0.calls.lock().await;
        let matrix_managed = self
            .0
            .matrix_source
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))?
            .is_some();
        if matrix_managed && matches!(command, Command::Logout) {
            self.shutdown().await;
            return Ok(
                json!({"state":"signed_out","deviceAuthorized":false,"transportOnline":false}),
            );
        }
        if matrix_managed && matches!(command, Command::BeginLogin) {
            return Err(failure(400, "sdk_owns_matrix_login"));
        }
        if matrix_managed && !matches!(command, Command::Logout) {
            self.sync_matrix().await?;
        }
        if matches!(command, Command::BeginLogin) {
            let ticket = self
                .0
                .host
                .console
                .owner_access_ticket()
                .map_err(|_| failure(503, "native_access_unavailable"))?;
            let bootstrap = self
                .call(
                    "POST",
                    "/console/session",
                    Some(json!({"ticket":ticket})),
                    "",
                )
                .await;
            if !bootstrap.status_code.unwrap_or(StatusCode::OK).is_success() {
                return Err(failure(503, "native_access_unavailable"));
            }
            let cookie = bootstrap
                .headers()
                .get_all("set-cookie")
                .iter()
                .find_map(|v| {
                    v.to_str()
                        .ok()
                        .filter(|s| s.starts_with("hagency_console="))
                        .map(|s| s.split(';').next().unwrap().to_owned())
                })
                .ok_or(failure(503, "native_access_unavailable"))?;
            *self.0.cookie.lock().await = cookie.clone();
            let mut response = self
                .call(
                    "POST",
                    "/console/server-login/start",
                    Some(json!({"server":self.0.expected_origin,"name":"Hagency Desktop"})),
                    &cookie,
                )
                .await;
            let nonce = response
                .headers()
                .get_all("set-cookie")
                .iter()
                .find_map(|v| {
                    v.to_str()
                        .ok()
                        .filter(|s| s.starts_with("hagency_server_login="))
                        .map(|s| s.split(';').next().unwrap().to_owned())
                });
            let value = read_response(&mut response).await?;
            *self.0.nonce.lock().await = nonce;
            return Ok(value);
        }
        if !matrix_managed && !matches!(command, Command::LoginStatus | Command::Logout) {
            self.identity().await?;
        }
        let (method, path, body) = request(command)?;
        let cookie = self.0.cookie.lock().await.clone();
        let mut response = self.call(method, &path, body, &cookie).await;
        if let Some(cookie) = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .find_map(|v| {
                v.to_str()
                    .ok()
                    .filter(|s| s.starts_with("hagency_console="))
                    .map(|s| s.split(';').next().unwrap().to_owned())
            })
        {
            *self.0.cookie.lock().await = cookie;
        }
        let result = read_response(&mut response).await;
        if matrix_managed {
            // Logout is allowed to finish without needing a live SDK token.
            if path != "/console/server-login/logout" {
                let cookie = self.0.cookie.lock().await.clone();
                let raw = cookie
                    .strip_prefix("hagency_console=")
                    .ok_or(failure(401, "owner_authorization_required"))?;
                if let Err(code) = self
                    .0
                    .host
                    .console
                    .0
                    .server_login
                    .check_matrix_source(raw)
                    .await
                {
                    if code == "matrix_authorization_unavailable" {
                        self.0.host.console.0.server_login.revoke_native_all().await;
                        self.0.host.console.stop_owned_runtimes().await;
                        self.0.host.console.stop_owner_provider().await;
                    } else {
                        self.shutdown().await;
                    }
                    return Err(failure(
                        if code.ends_with("unavailable") {
                            503
                        } else {
                            401
                        },
                        code,
                    ));
                }
            }
        }
        result
    }
    async fn call(&self, method: &str, path: &str, body: Option<Value>, cookie: &str) -> Response {
        let client = match method {
            "POST" => TestClient::post(format!("http://{}{path}", self.0.address)),
            "PUT" => TestClient::put(format!("http://{}{path}", self.0.address)),
            "DELETE" => TestClient::delete(format!("http://{}{path}", self.0.address)),
            _ => TestClient::get(format!("http://{}{path}", self.0.address)),
        };
        let client = client
            .add_header("host", self.0.address.to_string(), true)
            .add_header("sec-fetch-site", "same-origin", true)
            .add_header("origin", format!("http://{}", self.0.address), true)
            .add_header("cookie", cookie, true);
        match body {
            Some(body) => client.json(&body).send(&self.0.service).await,
            None => client.send(&self.0.service).await,
        }
    }
}
async fn read_response(response: &mut Response) -> Result<Value, NativeError> {
    let status = response.status_code.unwrap_or(StatusCode::OK).as_u16();
    let value = response
        .take_json::<Value>()
        .await
        .map_err(|_| failure(502, "invalid_native_response"))?;
    if status >= 400 {
        return Err(failure(
            status,
            value["code"].as_str().unwrap_or("native_operation_failed"),
        ));
    }
    Ok(value)
}
#[derive(Clone)]
struct Callback(Weak<Inner>);
#[handler]
impl Callback {
    async fn handle(&self, depot: &mut Depot) {
        depot.insert_typed(self.clone());
    }
}
#[handler]
async fn callback(req: &mut Request, depot: &Depot, res: &mut Response) {
    let result = async {
        let native = NativeOwner(
            depot
                .get_typed::<Callback>()
                .map_err(|_| failure(503, "native_state_unavailable"))?
                .0
                .upgrade()
                .ok_or(failure(410, "native_owner_stopped"))?,
        );
        let _guard = native.0.calls.lock().await;
        if native
            .0
            .matrix_source
            .lock()
            .map_err(|_| failure(503, "native_state_unavailable"))?
            .is_some()
        {
            return Err(failure(400, "sdk_owns_matrix_login"));
        }
        let query = req
            .uri()
            .query()
            .filter(|q| q.len() <= 8192)
            .ok_or(failure(400, "invalid_oauth_state"))?;
        let nonce = native
            .0
            .nonce
            .lock()
            .await
            .clone()
            .ok_or(failure(400, "invalid_oauth_state"))?;
        let mut response = native
            .call(
                "GET",
                &format!("/console/server-login/callback?{query}"),
                None,
                &nonce,
            )
            .await;
        let cookie = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .find_map(|v| {
                v.to_str()
                    .ok()
                    .filter(|s| s.starts_with("hagency_console="))
                    .map(|s| s.split(';').next().unwrap().to_owned())
            })
            .ok_or(failure(401, "owner_authorization_required"))?;
        *native.0.cookie.lock().await = cookie;
        native.0.nonce.lock().await.take();
        native.identity().await?;
        response.take_string().await.ok();
        Ok::<_, NativeError>(())
    }
    .await;
    res.add_header("cache-control", "no-store", true).unwrap();
    res.add_header("content-security-policy", "default-src 'none'", true)
        .unwrap();
    match result {
        Ok(()) => res.render("Hagency authorization completed. Return to Hagency Desktop."),
        Err(e) => {
            res.status_code(StatusCode::from_u16(e.status).unwrap_or(StatusCode::BAD_REQUEST));
            res.render("Authorization failed. Return to Hagency Desktop and retry with the same Matrix account.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_commands_cannot_select_an_arbitrary_path_or_foreign_binding_route() {
        assert!(
            request(Command::PrepareAgentWorkspace {
                agent_id: "../other".into()
            })
            .is_err()
        );
        let (method, path, body) = request(Command::PrepareAgentWorkspace {
            agent_id: "agt_owned".into(),
        })
        .unwrap();
        assert_eq!(method, "POST");
        assert_eq!(path, "/console/api/owner-provider/workspaces/agt_owned");
        assert!(body.is_none());
        let (method, path, body) = request(Command::Provider {
            action: ProviderAction::Models,
        })
        .unwrap();
        assert_eq!(method, "GET");
        assert_eq!(path, "/console/api/owner-provider/models");
        assert!(body.is_none());
        assert!(
            request(Command::ResumeAgentCommand {
                id: "../other".into()
            })
            .is_err()
        );
        assert!(
            request(Command::Local {
                agent_id: "a".into(),
                action: LocalAction::AgentPolicy,
                input: None,
                binding_id: Some("b".into()),
                requester: None
            })
            .is_err()
        );
        let (_, path, _) = request(Command::Local {
            agent_id: "a".into(),
            action: LocalAction::AgentPolicy,
            input: None,
            binding_id: None,
            requester: None,
        })
        .unwrap();
        assert_eq!(path, "/console/api/owned-agents/a/agent-policy");

        for id in [
            "",
            "../other",
            "agt?token=x",
            "agt/retire",
            "https://evil.test",
        ] {
            assert!(
                request(Command::Bindings {
                    agent_id: id.into()
                })
                .is_err()
            );
        }
        assert!(
            request(Command::SpaceCandidates {
                cursor: Some("!s%3Atest".into())
            })
            .is_err()
        );
        assert_eq!(
            request(Command::Binding {
                agent_id: "agt_1".into(),
                binding_id: "bnd_1".into(),
                action: BindingAction::Leave
            })
            .unwrap()
            .0,
            "DELETE"
        );
        assert!(
            request(Command::Local {
                agent_id: "agt_1".into(),
                action: LocalAction::Policy,
                input: None,
                binding_id: None,
                requester: None
            })
            .is_err()
        );
        let (_, path, _) = request(Command::Local {
            agent_id: "agt_1".into(),
            action: LocalAction::Policy,
            input: None,
            binding_id: Some("bnd_1".into()),
            requester: Some("@user:test&url=https://evil.test".into()),
        })
        .unwrap();
        assert!(path.starts_with("/console/api/owned-agents/agt_1/local-policy?"));
        assert!(!path.contains("&url="));
    }
    #[tokio::test]
    async fn native_host_needs_no_web_assets_and_restart_never_restores_remote_authority() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        let owner = NativeOwner::open(&state, "https://example.test/", "@owner:example.test")
            .await
            .unwrap();
        assert_eq!(
            owner.execute(Command::Projects).await.unwrap_err().code,
            "owner_authorization_required"
        );
        assert!(owner.execute(Command::LoginStatus).await.is_ok());
        owner.shutdown().await;
        assert!(owner.execute(Command::BeginLogin).await.is_err());
        drop(owner);
        let restored = NativeOwner::open(&state, "https://example.test/", "@owner:example.test")
            .await
            .unwrap();
        assert_eq!(
            restored.execute(Command::Agents).await.unwrap_err().code,
            "owner_authorization_required"
        );
        restored.shutdown().await;
    }
}

#[cfg(test)]
mod owner_scope_command_tests {
    use super::*;
    #[test]
    fn native_scope_admin_commands_are_closed_and_do_not_claim_admin_identity() {
        let policy = ProjectCreationPolicy {
            default_allow: true,
            allow: Default::default(),
            deny: Default::default(),
        };
        let (method, path, body) = request(Command::SetProjectCreationPolicy {
            project_id: "p".into(),
            expected_revision: 4,
            policy,
        })
        .unwrap();
        assert_eq!(
            (method, path.as_str()),
            ("PUT", "/console/api/owner-projects/p/creation-policy")
        );
        assert_eq!(body.unwrap()["expectedRevision"], 4);
        let (method, path, body) = request(Command::SetScopeServicePause {
            project_id: "p".into(),
            room_id: Some("!r:test".into()),
            paused: true,
        })
        .unwrap();
        assert_eq!(method, "POST");
        assert!(path.ends_with("/pause-service"));
        assert!(path.contains("%21r%3Atest"));
        assert!(body.is_none());
        assert!(
            request(Command::ScopeServiceState {
                project_id: "../p".into(),
                room_id: None
            })
            .is_err()
        );
        assert!(
            request(Command::SetRoomCreationPolicy {
                project_id: "p".into(),
                room_id: "!r:test".into(),
                expected_revision: -1,
                policy: RoomCreationPolicy::Disabled
            })
            .is_err()
        );
    }
    #[test]
    fn policy_dtos_reject_unknown_fields_and_disabled_payload() {
        assert!(
            serde_json::from_value::<RoomCreationPolicy>(json!({"mode":"disabled","allow":[]}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<ProjectCreationPolicy>(
                json!({"defaultAllow":true,"allow":[],"deny":[],"owner":"override"})
            )
            .is_err()
        );
        assert_eq!(
            serde_json::to_value(RoomCreationPolicy::Disabled).unwrap(),
            json!({"mode":"disabled"})
        );
    }
    #[test]
    fn agent_resource_commands_need_no_synthetic_room_and_reject_room_scope() {
        let (_, path, _) = request(Command::Local {
            agent_id: "a".into(),
            action: LocalAction::AgentPolicy,
            input: None,
            binding_id: None,
            requester: None,
        })
        .unwrap();
        assert_eq!(path, "/console/api/owned-agents/a/agent-policy");
        assert!(
            request(Command::Local {
                agent_id: "a".into(),
                action: LocalAction::SaveAgentModel,
                input: Some(json!({})),
                binding_id: Some("b".into()),
                requester: None
            })
            .is_err()
        );
        assert!(request(Command::ResumeAgentCommand { id: "../x".into() }).is_err());
        assert_eq!(
            request(Command::AgentCommands).unwrap().1,
            "/console/api/owned-agents/commands"
        );
    }
    #[test]
    fn device_and_assignment_commands_expose_no_bearer_or_arbitrary_path() {
        assert_eq!(
            request(Command::Devices).unwrap().1,
            "/console/api/owned-agents/devices"
        );
        let (method, path, body) = request(Command::AssignAgentToCurrentDevice {
            agent_id: "a".into(),
            expected_generation: 3,
        })
        .unwrap();
        assert_eq!(
            (method, path.as_str()),
            ("PUT", "/console/api/owned-agents/a/execution-device")
        );
        assert_eq!(body, Some(json!({"expectedGeneration":3})));
        assert!(
            request(Command::AssignAgentToCurrentDevice {
                agent_id: "a".into(),
                expected_generation: -1
            })
            .is_err()
        );
        assert!(
            request(Command::AgentDetails {
                agent_id: "../x".into()
            })
            .is_err()
        );
        assert_eq!(
            request(Command::AgentDetails {
                agent_id: "a".into()
            })
            .unwrap()
            .1,
            "/console/api/owned-agents/a"
        );
        assert!(
            request(Command::AgentOwnerDirect {
                agent_id: "a".into(),
                room_id: "!r:test/escape".into()
            })
            .is_err()
        );
    }
}
