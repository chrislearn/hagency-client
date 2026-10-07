//! Explicit owner-started, chat-only Codex host. No legacy execution fallback.
use super::{
    AuthorizedDevice, Console,
    device_execution::{
        DeviceOperation as Op, DeviceResponse as Response, HistorySnapshot, Lease, LeaseRef,
        Outcome, Takeover,
    },
    owned_agents::{ledger_path, profile_identity},
    server_login::OwnerOperation,
};
use hagency_agent_local::{
    Ledger, Scope,
    codex::{self, BudgetMode, Profile},
    inbox::{self, Finish, State},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, mpsc, watch};

/// Construct only from an authenticated owner's explicit start action. Paths
/// select a dedicated provider login; credential_ref is metadata, NOT a resolver.
#[derive(Clone)]
pub(super) struct StartConfig {
    pub agent_id: String,
    pub binding_id: String,
    pub profile: Profile,
    pub mode: BudgetMode,
    pub credential_ref: String,
    pub takeover: Takeover,
    pub host_files: bool,
}
#[derive(Debug, thiserror::Error)]
pub(super) enum RuntimeError {
    #[error("strict token limits unavailable for Codex")]
    StrictUnavailable,
    #[error("owner authorization required")]
    Authorization,
    #[error("Agent runtime already active")]
    AlreadyActive,
    #[error("provider profile or local state unavailable")]
    Profile,
    #[error("shared provider account or environment unavailable")]
    Provider,
    #[error("device transport unavailable")]
    Transport,
    #[error("invalid or expired tool approval")]
    Approval,
    #[error("complete owner ledger recovery required")]
    LedgerRecovery,
    #[error("runtime stopped")]
    Stopped,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RuntimeStatus {
    pub agent_id: String,
    pub binding_id: Option<String>,
    pub bindings: Vec<RuntimeStatus>,
    pub phase: &'static str,
    pub last_error: Option<&'static str>,
    pub native_tools: bool,
    pub strict_token_cap: bool,
    pub host_file_tools: bool,
    pub file_replace: bool,
    pub network_tools: bool,
    pub capability_version: Option<&'static str>,
}
impl RuntimeStatus {
    fn with_host_files(mut self, enabled: bool) -> Self {
        self.host_file_tools = enabled;
        if enabled {
            self.capability_version = Some("hagency-room-files-v1");
        }
        self
    }
    fn new(agent: &str, phase: &'static str, error: Option<&'static str>) -> Self {
        Self {
            agent_id: agent.into(),
            binding_id: None,
            bindings: vec![],
            phase,
            last_error: error,
            native_tools: false,
            strict_token_cap: false,
            host_file_tools: false,
            file_replace: false,
            network_tools: false,
            capability_version: None,
        }
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApprovalView {
    pub proposal_id: String,
    pub args_digest: String,
    pub proposal: hagency_agent_local::ToolProposal,
}
type DevicePin = super::device_execution::DeviceIdentity;
struct ApprovalSlot {
    view: ApprovalView,
    pending: codex::PendingApproval,
    device: DevicePin,
}
struct BindingHandle {
    cancel: watch::Sender<bool>,
    status: watch::Receiver<RuntimeStatus>,
}
struct BindingWork {
    config: StartConfig,
    cancel: watch::Sender<bool>,
    receiver: watch::Receiver<bool>,
    status: watch::Sender<RuntimeStatus>,
}
fn binding_work(config: StartConfig) -> (BindingWork, BindingHandle, RuntimeStatus) {
    let (cancel, receiver) = watch::channel(false);
    let mut status = RuntimeStatus::new(&config.agent_id, "checking_provider", None)
        .with_host_files(config.host_files);
    status.binding_id = Some(config.binding_id.clone());
    let (sender, rx) = watch::channel(status.clone());
    (
        BindingWork {
            config,
            cancel: cancel.clone(),
            receiver,
            status: sender,
        },
        BindingHandle { cancel, status: rx },
        status,
    )
}
struct Entry {
    commands: mpsc::UnboundedSender<BindingWork>,
    bindings: BTreeMap<String, BindingHandle>,
    cancel: watch::Sender<bool>,
    status: watch::Receiver<RuntimeStatus>,
    task: tokio::task::JoinHandle<()>,
    device: DevicePin,
    approvals: BTreeMap<String, ApprovalSlot>,
}
impl Entry {
    fn binding_active(&self, binding: &str) -> bool {
        self.bindings
            .get(binding)
            .is_some_and(|b| !*b.cancel.borrow() && b.status.borrow().phase != "stopped")
    }
    fn take_approval(
        &mut self,
        pin: &DevicePin,
        agent: &str,
        id: &str,
        digest: &str,
        bindings: &[Binding],
        at: i64,
    ) -> Result<codex::PendingApproval, RuntimeError> {
        if *self.cancel.borrow() || self.task.is_finished() || &self.device != pin {
            return Err(RuntimeError::Authorization);
        }
        let slot = self.approvals.get(id).ok_or(RuntimeError::Approval)?;
        let proposal = &slot.view.proposal;
        if slot.view.args_digest != digest
            || &slot.device != pin
            || proposal.expires <= at
            || slot.pending.is_closed()
            || proposal.scope.agent != agent
            || !self.binding_active(&proposal.scope.binding)
            || !bindings.iter().any(|b| {
                b.id == proposal.scope.binding
                    && b.room_id == proposal.scope.room
                    && b.agent_id == agent
                    && b.state == "active"
            })
        {
            return Err(RuntimeError::Approval);
        }
        Ok(self
            .approvals
            .remove(id)
            .ok_or(RuntimeError::Approval)?
            .pending)
    }
}
struct AbortHeartbeat(tokio::task::AbortHandle);
impl Drop for AbortHeartbeat {
    fn drop(&mut self) {
        self.0.abort();
    }
}
/// The server lease alone does not distinguish two processes sharing the same
/// local device credentials. Hold a per-Agent OS lock for the entire job.
struct AgentLock {
    _file: std::fs::File,
}
fn lock_agent(path: &std::path::Path, agent: &str) -> Result<AgentLock, RuntimeError> {
    use sha2::{Digest, Sha256};
    let name = format!("runtime_{:x}.lock", Sha256::digest(agent.as_bytes()));
    let path = path.parent().ok_or(RuntimeError::Profile)?.join(name);
    let file = hagency_store::private::open(&path, true)
        .or_else(|_| hagency_store::private::open(&path, false))
        .map_err(|_| RuntimeError::Profile)?;
    file.try_lock().map_err(|_| RuntimeError::AlreadyActive)?;
    Ok(AgentLock { _file: file })
}
#[derive(Default, Clone)]
pub(super) struct OwnedRuntime {
    entries: Arc<Mutex<BTreeMap<String, Entry>>>,
}
fn identity(device: &AuthorizedDevice, agent: &str) -> String {
    serde_json::to_string(&(
        device.origin(),
        device.issuer(),
        device.subject(),
        device.owner_mxid(),
        agent,
    ))
    .expect("string tuple")
}
fn local<T>(r: hagency_agent_local::Result<T>) -> Result<T, RuntimeError> {
    r.map_err(|_| RuntimeError::Profile)
}
fn wall_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(i64::MAX)
}
fn now() -> i64 {
    wall_ms() / 1000
}
fn convert(event: super::device_execution::Dispatch) -> Result<inbox::Dispatch, RuntimeError> {
    serde_json::from_value(serde_json::to_value(event).map_err(|_| RuntimeError::Transport)?)
        .map_err(|_| RuntimeError::Transport)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Binding {
    id: String,
    agent_id: String,
    room_id: String,
    state: String,
}
impl OwnedRuntime {
    /// Holding this registry lock through publication lets switch revoke first,
    /// then drain: an in-flight old request cannot publish after the drain.
    async fn entries_for(
        &self,
        device: &AuthorizedDevice,
    ) -> Result<tokio::sync::MutexGuard<'_, BTreeMap<String, Entry>>, RuntimeError> {
        let entries = self.entries.lock().await;
        device.bearer().map_err(|_| RuntimeError::Authorization)?;
        Ok(entries)
    }
    /// Startup is not online until provider handshake/login and device lease are
    /// verified. Cookie is used here for owner scope and never retained by a job.
    pub async fn start(
        &self,
        console: Console,
        cookie: &str,
        config: StartConfig,
    ) -> Result<RuntimeStatus, RuntimeError> {
        match config.mode {
            BudgetMode::Strict => return Err(RuntimeError::StrictUnavailable),
            BudgetMode::Estimated { reservation: 0 } => {
                return Err(RuntimeError::Profile);
            }
            _ => {}
        }
        config
            .profile
            .validate()
            .map_err(|_| RuntimeError::Profile)?;
        if config.profile.executable
            != super::owner_provider::executable().map_err(|_| RuntimeError::Profile)?
        {
            return Err(RuntimeError::Profile);
        }
        if super::owner_provider::credential_reference(&config.profile.codex_home)
            .map_err(|_| RuntimeError::Profile)?
            != config.credential_ref
        {
            return Err(RuntimeError::Profile);
        }
        let device = console
            .authorized_device()
            .await
            .map_err(|_| RuntimeError::Authorization)?;
        device.bearer().map_err(|_| RuntimeError::Authorization)?;
        let reply = console
            .owner_api(
                cookie,
                OwnerOperation::Bindings {
                    agent: config.agent_id.clone(),
                },
            )
            .await
            .map_err(|_| RuntimeError::Authorization)?;
        if reply.owner != device.owner_mxid()
            || reply.origin != device.origin()
            || reply.issuer != device.issuer()
            || reply.subject != device.subject()
        {
            return Err(RuntimeError::Authorization);
        }
        let bindings: Vec<Binding> = serde_json::from_value(reply.value["bindings"].clone())
            .map_err(|_| RuntimeError::Profile)?;
        let binding = bindings
            .into_iter()
            .find(|b| {
                b.agent_id == config.agent_id && b.id == config.binding_id && b.state == "active"
            })
            .ok_or(RuntimeError::Profile)?;
        let root = console
            .0
            .server_login
            .state_directory()
            .ok_or(RuntimeError::Profile)?;
        let path = ledger_path(
            &root,
            device.origin(),
            device.issuer(),
            device.subject(),
            device.owner_mxid(),
        )
        .map_err(|_| RuntimeError::Profile)?;
        // Paths are owner-specific, and must be initialized by the owner. A
        // caller cannot silently repurpose ~/.codex or another owner's login.
        let directory = path
            .parent()
            .ok_or(RuntimeError::Profile)?
            .canonicalize()
            .map_err(|_| RuntimeError::Profile)?;
        if config.profile.codex_home != directory.join("codex-home")
            || config.profile.home != directory.join("provider-home")
        {
            return Err(RuntimeError::Profile);
        }
        let mut ledger = local(Ledger::open_scoped(
            &path,
            device.owner_mxid(),
            &profile_identity(
                device.origin(),
                device.issuer(),
                device.subject(),
                device.owner_mxid(),
            )
            .map_err(|_| RuntimeError::Authorization)?,
        ))?;
        let scope = Scope {
            agent: config.agent_id.clone(),
            binding: binding.id,
            room: binding.room_id,
            requester: device.owner_mxid().into(),
            thread: "runtime-profile-validation".into(),
        };
        local(ledger.register_binding(device.owner_mxid(), &scope))?;
        match local(ledger.model_profile(&scope))? {
            Some(p)
                if p.model == config.profile.model
                    && p.workspace_root == config.profile.cwd.to_string_lossy()
                    && p.credential_ref == config.credential_ref
                    && p.credential_ref.starts_with("keychain:") => {}
            _ => return Err(RuntimeError::Profile),
        }
        let key = identity(&device, &config.agent_id);
        let mut entries = self.entries_for(&device).await?;
        if let Some(entry) = entries
            .get_mut(&key)
            .filter(|entry| !entry.task.is_finished() && !*entry.cancel.borrow())
        {
            if entry.device != DevicePin::from(&device) {
                return Err(RuntimeError::Authorization);
            }
            if entry.binding_active(&config.binding_id) {
                return Err(RuntimeError::AlreadyActive);
            }
            let id = config.binding_id.clone();
            let (work, handle, status) = binding_work(config);
            entry
                .commands
                .send(work)
                .map_err(|_| RuntimeError::Stopped)?;
            entry.bindings.insert(id, handle);
            return Ok(status);
        }
        let process_lock = lock_agent(&path, &config.agent_id)?;
        let agent = config.agent_id.clone();
        let binding = config.binding_id.clone();
        let (initial, handle, status) = binding_work(config);
        let (commands, command_rx) = mpsc::unbounded_channel();
        let (cancel, receiver) = watch::channel(false);
        let (sender, status_receiver) =
            watch::channel(RuntimeStatus::new(&agent, "checking_provider", None));
        let cancellation = cancel.clone();
        let device_pin = DevicePin::from(&device);
        let task = tokio::spawn(async move {
            let _process_lock = process_lock;
            let result = run_host(
                console,
                device,
                path,
                initial,
                command_rx,
                receiver,
                cancellation,
                sender.clone(),
            )
            .await;
            let failure = failure_code(&result);
            sender.send_replace(RuntimeStatus::new(&agent, "stopped", failure));
        });
        entries.insert(
            key,
            Entry {
                commands,
                bindings: BTreeMap::from([(binding, handle)]),
                cancel,
                status: status_receiver,
                task,
                device: device_pin,
                approvals: BTreeMap::new(),
            },
        );
        Ok(status)
    }
    async fn offer_approval(
        &self,
        console: &Console,
        expected: &DevicePin,
        pending: codex::PendingApproval,
    ) -> Result<(), RuntimeError> {
        use sha2::{Digest, Sha256};
        let device = console
            .authorized_device()
            .await
            .map_err(|_| RuntimeError::Authorization)?;
        device.bearer().map_err(|_| RuntimeError::Authorization)?;
        if expected != &DevicePin::from(&device) {
            return Err(RuntimeError::Authorization);
        }
        let agent = &pending.proposal.scope.agent;
        let mut entries = self.entries_for(&device).await?;
        let entry = entries
            .get_mut(&identity(&device, agent))
            .ok_or(RuntimeError::Authorization)?;
        entry
            .approvals
            .retain(|_, slot| slot.view.proposal.expires > now() && !slot.pending.is_closed());
        if *entry.cancel.borrow()
            || entry.task.is_finished()
            || entry.device != DevicePin::from(&device)
            || !entry.binding_active(&pending.proposal.scope.binding)
            || pending.proposal.expires <= now()
            || entry.approvals.len() >= 128
        {
            return Err(RuntimeError::Authorization);
        }
        let mut nonce = [0u8; 32];
        getrandom::fill(&mut nonce).map_err(|_| RuntimeError::Profile)?;
        let proposal_id = format!("{:x}", Sha256::digest(nonce));
        let args_digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&pending.proposal).map_err(|_| RuntimeError::Profile)?
            )
        );
        let view = ApprovalView {
            proposal_id: proposal_id.clone(),
            args_digest,
            proposal: pending.proposal.clone(),
        };
        entry.approvals.insert(
            proposal_id,
            ApprovalSlot {
                view,
                pending,
                device: DevicePin::from(&device),
            },
        );
        Ok(())
    }
    pub async fn pending(
        &self,
        console: &Console,
        agent: &str,
    ) -> Result<Vec<ApprovalView>, RuntimeError> {
        let device = console
            .authorized_device()
            .await
            .map_err(|_| RuntimeError::Authorization)?;
        device.bearer().map_err(|_| RuntimeError::Authorization)?;
        let mut entries = self.entries_for(&device).await?;
        let Some(entry) = entries.get_mut(&identity(&device, agent)) else {
            return Ok(vec![]);
        };
        if *entry.cancel.borrow()
            || entry.task.is_finished()
            || entry.device != DevicePin::from(&device)
        {
            return Err(RuntimeError::Authorization);
        }
        entry
            .approvals
            .retain(|_, slot| slot.view.proposal.expires > now() && !slot.pending.is_closed());
        let active: BTreeSet<_> = entry
            .bindings
            .iter()
            .filter(|(_, b)| !*b.cancel.borrow() && b.status.borrow().phase != "stopped")
            .map(|(id, _)| id.clone())
            .collect();
        entry
            .approvals
            .retain(|_, slot| active.contains(&slot.view.proposal.scope.binding));
        Ok(entry
            .approvals
            .values()
            .map(|slot| slot.view.clone())
            .collect())
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn decide(
        &self,
        console: &Console,
        cookie: &str,
        agent: &str,
        proposal_id: &str,
        args_digest: &str,
        approved: bool,
    ) -> Result<(), RuntimeError> {
        let device = console
            .authorized_device()
            .await
            .map_err(|_| RuntimeError::Authorization)?;
        device.bearer().map_err(|_| RuntimeError::Authorization)?;
        let owner = console
            .owner_api(
                cookie,
                OwnerOperation::Bindings {
                    agent: agent.into(),
                },
            )
            .await
            .map_err(|_| RuntimeError::Authorization)?;
        if owner.owner != device.owner_mxid()
            || owner.origin != device.origin()
            || owner.issuer != device.issuer()
            || owner.subject != device.subject()
        {
            return Err(RuntimeError::Authorization);
        }
        let bindings: Vec<Binding> = serde_json::from_value(owner.value["bindings"].clone())
            .map_err(|_| RuntimeError::Profile)?;
        device.bearer().map_err(|_| RuntimeError::Authorization)?;
        let mut entries = self.entries_for(&device).await?;
        let entry = entries
            .get_mut(&identity(&device, agent))
            .ok_or(RuntimeError::Authorization)?;
        if *entry.cancel.borrow()
            || entry.task.is_finished()
            || entry.device != DevicePin::from(&device)
        {
            return Err(RuntimeError::Authorization);
        }
        let pin = DevicePin::from(&device);
        let pending =
            entry.take_approval(&pin, agent, proposal_id, args_digest, &bindings, now())?;
        pending.decide(approved);
        Ok(())
    }

    pub async fn status(
        &self,
        console: &Console,
        agent: &str,
    ) -> Result<Option<RuntimeStatus>, RuntimeError> {
        let device = console
            .authorized_device()
            .await
            .map_err(|_| RuntimeError::Authorization)?;
        let entries = self.entries.lock().await;
        device.bearer().map_err(|_| RuntimeError::Authorization)?;
        if entries
            .get(&identity(&device, agent))
            .is_some_and(|entry| entry.device != DevicePin::from(&device))
        {
            return Err(RuntimeError::Authorization);
        }
        Ok(entries.get(&identity(&device, agent)).map(|e| {
            let mut status = e.status.borrow().clone();
            status.bindings = e
                .bindings
                .values()
                .map(|b| {
                    let mut child = b.status.borrow().clone();
                    if status.phase == "stopped" {
                        child.phase = "stopped";
                        child.last_error = child.last_error.or(status.last_error);
                    }
                    child
                })
                .collect();
            status
        }))
    }
    pub async fn stop(
        &self,
        console: &Console,
        agent: &str,
        binding: &str,
    ) -> Result<(), RuntimeError> {
        let device = console
            .authorized_device()
            .await
            .map_err(|_| RuntimeError::Authorization)?;
        device.bearer().map_err(|_| RuntimeError::Authorization)?;
        let mut receiver = {
            let mut entries = self.entries.lock().await;
            let Some(entry) = entries.get_mut(&identity(&device, agent)) else {
                return Ok(());
            };
            if entry.device != DevicePin::from(&device) {
                return Err(RuntimeError::Authorization);
            }
            let Some(handle) = entry.bindings.get(binding) else {
                return Ok(());
            };
            handle.cancel.send_replace(true);
            let receiver = handle.status.clone();
            entry
                .approvals
                .retain(|_, slot| slot.view.proposal.scope.binding != binding);
            if !entry.bindings.keys().any(|id| entry.binding_active(id)) {
                entry.cancel.send_replace(true);
            }
            receiver
        };
        while receiver.borrow().phase != "stopped" {
            if receiver.changed().await.is_err() {
                break;
            }
        }
        Ok(())
    }
    /// Console shutdown/logout must invoke this even when authorization is gone.
    pub(super) fn request_stop_all(&self) {
        if let Ok(entries) = self.entries.try_lock() {
            for entry in entries.values() {
                entry.cancel.send_replace(true);
            }
        } else if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let manager = self.clone();
            runtime.spawn(async move {
                manager.stop_all().await;
            });
        }
        // retire revokes the shared device atomic before invoking this. If a
        // lock is busy without a Tokio caller, the heartbeat still fails closed.
    }
    pub(super) async fn stop_profile(&self, device: &AuthorizedDevice) -> Result<(), RuntimeError> {
        let pin = DevicePin::from(device);
        let mut entries = self.entries_for(device).await?;
        let keys: Vec<_> = entries
            .iter()
            .filter(|(_, e)| {
                e.device.origin == pin.origin
                    && e.device.issuer == pin.issuer
                    && e.device.subject == pin.subject
                    && e.device.owner == pin.owner
                    && e.device.user == pin.user
            })
            .map(|(key, _)| key.clone())
            .collect();
        let removed: Vec<_> = keys
            .into_iter()
            .filter_map(|key| entries.remove(&key))
            .collect();
        for entry in &removed {
            entry.cancel.send_replace(true);
        }
        drop(entries);
        for entry in removed {
            let _ = entry.task.await;
        }
        Ok(())
    }
    pub async fn stop_all(&self) {
        let entries = std::mem::take(&mut *self.entries.lock().await);
        for entry in entries.values() {
            entry.cancel.send_replace(true);
        }
        for (_, entry) in entries {
            let _ = entry.task.await;
        }
    }
}
async fn canceled(receiver: &mut watch::Receiver<bool>) {
    loop {
        if *receiver.borrow_and_update() {
            return;
        }
        if receiver.changed().await.is_err() {
            return;
        }
    }
}
trait Transport {
    fn approval_host(&self) -> Option<PinnedConsole> {
        None
    }
    fn execute(
        &self,
        op: Op,
    ) -> impl std::future::Future<Output = Result<Response, RuntimeError>> + Send;
}
#[derive(Clone)]
struct PinnedConsole {
    console: Console,
    pin: DevicePin,
}
impl Transport for PinnedConsole {
    fn approval_host(&self) -> Option<PinnedConsole> {
        Some(self.clone())
    }
    async fn execute(&self, op: Op) -> Result<Response, RuntimeError> {
        self.console
            .execution_api_pinned(op, &self.pin)
            .await
            .map_err(|e| {
                if e.code == "execution_history_changed" || e.code == "ledger_recovery_required" {
                    RuntimeError::LedgerRecovery
                } else if matches!(e.status, 401 | 403 | 409) {
                    RuntimeError::Authorization
                } else {
                    RuntimeError::Transport
                }
            })
    }
}
async fn operation<T: Transport>(
    console: &T,
    receiver: &mut watch::Receiver<bool>,
    op: Op,
) -> Result<Response, RuntimeError> {
    tokio::select! { biased; _=canceled(receiver)=>Err(RuntimeError::Stopped), response=console.execute(op)=>response }
}
fn deadline(lease: &Lease) -> Result<Instant, RuntimeError> {
    let remaining = i128::from(lease.expires_at_ms) - i128::from(wall_ms());
    if remaining <= 0 || remaining > 31_000 {
        return Err(RuntimeError::Authorization);
    }
    Ok(Instant::now() + Duration::from_millis(remaining as u64))
}
async fn heartbeat(
    console: Console,
    mut device: AuthorizedDevice,
    mut lease: Lease,
    stop: watch::Sender<bool>,
    mut receiver: watch::Receiver<bool>,
) {
    let mut next = Instant::now() + Duration::from_secs(5);
    while let Ok(expiry) = deadline(&lease) {
        if device.bearer().is_err() {
            break;
        }
        if Instant::now() >= next {
            let pin = DevicePin::from(&device);
            let renewed = tokio::select! { biased; _=canceled(&mut receiver)=>return, _=tokio::time::sleep_until(expiry.into())=>break, result=console.execution_api_pinned(Op::Renew{lease:lease.reference(),ttl_ms:30_000},&pin)=>result };
            let Ok(Response::Lease(value)) = renewed else {
                break;
            };
            lease = value;
            // Snapshot expiry can be refreshed; immutable device identity cannot.
            let fresh = tokio::select! {biased; _=canceled(&mut receiver)=>return, _=tokio::time::sleep_until(expiry.into())=>break, result=console.authorized_device()=>result};
            let Ok(fresh) = fresh else {
                break;
            };
            if fresh.origin() != device.origin()
                || fresh.issuer() != device.issuer()
                || fresh.subject() != device.subject()
                || fresh.user_id() != device.user_id()
                || fresh.owner_mxid() != device.owner_mxid()
                || fresh.device_id() != device.device_id()
                || fresh.generation() != device.generation()
            {
                break;
            }
            device = fresh;
            next = Instant::now() + Duration::from_secs(5);
        }
        tokio::select! { biased; _=canceled(&mut receiver)=>return, _=tokio::time::sleep(Duration::from_millis(200))=>{} }
    }
    stop.send_replace(true);
}
fn failure_code(result: &Result<(), RuntimeError>) -> Option<&'static str> {
    match result {
        Ok(()) | Err(RuntimeError::Stopped) => None,
        Err(RuntimeError::Authorization) => Some("authorization_or_lease_lost"),
        Err(RuntimeError::Profile) => Some("profile_or_local_state_unavailable"),
        Err(RuntimeError::Provider) => Some("provider_authorization_lost"),
        Err(RuntimeError::LedgerRecovery) => Some("ledger_recovery_required"),
        _ => Some("device_transport_unavailable"),
    }
}
/// Consume stable metadata pages without retaining unbounded message/history payloads.
async fn history_coverage<T: Transport>(
    transport: &T,
    ledger: &mut Ledger,
    owner: &str,
    agent: &str,
    cancel: &mut watch::Receiver<bool>,
) -> Result<(HistorySnapshot, bool), RuntimeError> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"hagency-started-executions-v1\n");
    let mut cursor = None;
    let mut snapshot = None;
    let mut seen = 0u64;
    let mut covered = true;
    loop {
        let Response::History(page) = operation(
            transport,
            cancel,
            Op::History {
                agent_id: agent.into(),
                cursor: cursor.clone(),
                snapshot: snapshot.clone(),
            },
        )
        .await?
        else {
            return Err(RuntimeError::LedgerRecovery);
        };
        if page.agent_id != agent || snapshot.as_ref().is_some_and(|s| s != &page.snapshot) {
            return Err(RuntimeError::LedgerRecovery);
        }
        let mut previous = cursor.as_deref();
        for entry in &page.executions {
            hasher.update(
                serde_json::to_vec(&(&entry.dispatch_id, &entry.execution_id))
                    .map_err(|_| RuntimeError::LedgerRecovery)?,
            );
            hasher.update(b"\n");
            if previous.is_some_and(|p| p >= entry.dispatch_id.as_str()) {
                return Err(RuntimeError::LedgerRecovery);
            }
            previous = Some(&entry.dispatch_id);
        }
        if page
            .next_cursor
            .as_ref()
            .is_some_and(|c| Some(c.as_str()) != previous || page.executions.is_empty())
        {
            return Err(RuntimeError::LedgerRecovery);
        }
        seen = seen
            .checked_add(page.executions.len() as u64)
            .ok_or(RuntimeError::LedgerRecovery)?;
        if seen > page.snapshot.count || page.executions.len() > 128 {
            return Err(RuntimeError::LedgerRecovery);
        }
        covered &= local(ledger.covers_execution_history_page(
            owner,
            agent,
            &page.executions,
            page.next_cursor.is_none(),
        ))?;
        snapshot = Some(page.snapshot);
        cursor = page.next_cursor;
        if cursor.is_none() {
            let snapshot = snapshot.ok_or(RuntimeError::LedgerRecovery)?;
            if seen != snapshot.count || format!("{:x}", hasher.finalize()) != snapshot.digest {
                return Err(RuntimeError::LedgerRecovery);
            }
            return Ok((snapshot, covered));
        }
    }
}
async fn acquire_checked<T: Transport>(
    transport: &T,
    ledger: &mut Ledger,
    owner: &str,
    config: &StartConfig,
    cancel: &mut watch::Receiver<bool>,
) -> Result<(Lease, HistorySnapshot, bool), RuntimeError> {
    let (snapshot, covered) =
        history_coverage(transport, ledger, owner, &config.agent_id, cancel).await?;
    if !covered {
        let known = local(ledger.has_known_reply(owner, &config.agent_id, &config.binding_id))?;
        if !known || !matches!(config.takeover, Takeover::OwnerRequested) {
            return Err(RuntimeError::LedgerRecovery);
        }
    }
    let Response::Lease(lease) = operation(
        transport,
        cancel,
        Op::Acquire {
            agent_id: config.agent_id.clone(),
            ttl_ms: 30_000,
            takeover: config.takeover,
            history_snapshot: snapshot.clone(),
        },
    )
    .await?
    else {
        return Err(RuntimeError::Transport);
    };
    Ok((lease, snapshot, covered))
}
async fn post_acquire_coverage<T: Transport>(
    transport: &T,
    ledger: &mut Ledger,
    owner: &str,
    agent: &str,
    expected: &HistorySnapshot,
    cancel: &mut watch::Receiver<bool>,
) -> Result<bool, RuntimeError> {
    let (after, covered) = history_coverage(transport, ledger, owner, agent, cancel).await?;
    if &after != expected {
        return Err(RuntimeError::LedgerRecovery);
    }
    Ok(covered)
}
#[allow(clippy::too_many_arguments)]
async fn run_host(
    console: Console,
    device: AuthorizedDevice,
    path: PathBuf,
    initial: BindingWork,
    commands: mpsc::UnboundedReceiver<BindingWork>,
    mut cancel: watch::Receiver<bool>,
    stop: watch::Sender<bool>,
    status: watch::Sender<RuntimeStatus>,
) -> Result<(), RuntimeError> {
    let profile_id = profile_identity(
        device.origin(),
        device.issuer(),
        device.subject(),
        device.owner_mxid(),
    )
    .map_err(|_| RuntimeError::Authorization)?;
    let transport = PinnedConsole {
        console: console.clone(),
        pin: DevicePin::from(&device),
    };
    let owner = device.owner_mxid().to_owned();
    let agent = initial.config.agent_id.clone();
    // Verify the owner's dedicated provider before any acquisition or explicit
    // takeover can disrupt an existing device. This is account/config only.
    let mut provider = tokio::select! {biased;_=canceled(&mut cancel)=>return Err(RuntimeError::Stopped),p=initial.config.profile.spawn()=>p.map_err(|_|RuntimeError::Provider)?};
    let checked = tokio::select! {biased;_=canceled(&mut cancel)=>Err(RuntimeError::Stopped),r=async {provider.session.initialize().await?;provider.session.verify_host_environment().await?;provider.session.require_local_account().await}=>r.map_err(|_|RuntimeError::Provider)};
    let _ = provider.stop().await;
    checked?;
    let mut ledger = local(Ledger::open_scoped(&path, &owner, &profile_id))?;
    let (lease, history_snapshot, pre_covered) = acquire_checked(
        &transport,
        &mut ledger,
        &owner,
        &initial.config,
        &mut cancel,
    )
    .await?;
    drop(ledger);
    let reference = lease.reference();
    let monitor = tokio::spawn(heartbeat(
        console.clone(),
        device,
        lease,
        stop.clone(),
        cancel.clone(),
    ));
    let _heartbeat_guard = AbortHeartbeat(monitor.abort_handle());
    let result=async {
        let mut ledger=local(Ledger::open_scoped(&path,&owner,&profile_id))?;
        let post_covered=post_acquire_coverage(&transport,&mut ledger,&owner,&agent,&history_snapshot,&mut cancel).await?;
        let models_allowed=pre_covered && post_covered;
        local(ledger.recover_agent_executions(&owner,&agent))?;
        drop(ledger);
        status.send_replace(RuntimeStatus::new(&agent,if models_allowed{"online"}else{"ledger_recovery_required"},if models_allowed{None}else{Some("ledger_recovery_required")}));
        supervise_bindings(initial,commands,&mut cancel,|work| {
            let transport=transport.clone();let owner=owner.clone();let profile_id=profile_id.clone();let path=path.clone();let reference=reference.clone();let global_stop=stop.clone();
            tokio::spawn(async move {
                let mut work=work;
                let result=async {
                    let mut provider=tokio::select! {biased;_=canceled(&mut work.receiver)=>return Err(RuntimeError::Stopped),p=work.config.profile.spawn()=>p.map_err(|_|RuntimeError::Profile)?};
                    let checked=tokio::select! {biased;_=canceled(&mut work.receiver)=>Err(RuntimeError::Stopped),r=async {provider.session.initialize().await?;provider.session.verify_host_environment().await?;provider.session.require_local_account().await}=>r.map_err(|_|RuntimeError::Provider)};
                    let _=provider.stop().await;checked?;
                    let mut ledger=local(Ledger::open_scoped(&path,&owner,&profile_id))?;
                    let mut ready=RuntimeStatus::new(&work.config.agent_id,if models_allowed{"online_chat_only"}else{"ledger_recovery_required"},if models_allowed{None}else{Some("ledger_recovery_required")}).with_host_files(work.config.host_files);ready.binding_id=Some(work.config.binding_id.clone());work.status.send_replace(ready);
                    execution_loop_with_gate(&transport,&owner,&mut ledger,&work.config,&reference,&mut work.receiver,&work.status,models_allowed).await
                }.await;
                if matches!(result,Err(RuntimeError::Provider)) {global_stop.send_replace(true);}
                let mut final_status=RuntimeStatus::new(&work.config.agent_id,"stopped",failure_code(&result)).with_host_files(work.config.host_files);
                final_status.binding_id=Some(work.config.binding_id.clone());work.status.send_replace(final_status);
            })
        }).await
    }.await;
    monitor.abort();
    let _ = monitor.await;
    let _ = transport.execute(Op::Release { lease: reference }).await;
    result
}
/// One supervisor owns the lease. Workers can never acquire, renew or release it.
/// All cancellation paths join workers before the supervisor releases the lease.
async fn supervise_bindings<F>(
    initial: BindingWork,
    mut commands: mpsc::UnboundedReceiver<BindingWork>,
    cancel: &mut watch::Receiver<bool>,
    mut spawn: F,
) -> Result<(), RuntimeError>
where
    F: FnMut(BindingWork) -> tokio::task::JoinHandle<()>,
{
    let mut workers: BTreeMap<String, (watch::Sender<bool>, tokio::task::JoinHandle<()>)> =
        BTreeMap::new();
    let id = initial.config.binding_id.clone();
    let stop = initial.cancel.clone();
    workers.insert(id, (stop, spawn(initial)));
    loop {
        if *cancel.borrow() {
            break;
        }
        let finished: Vec<_> = workers
            .iter()
            .filter(|(_, (_, task))| task.is_finished())
            .map(|(id, _)| id.clone())
            .collect();
        for id in finished {
            if let Some((_, task)) = workers.remove(&id) {
                let _ = task.await;
            }
        }
        if workers.is_empty() {
            // A start accepted before the last worker ended must still run.
            if let Ok(work) = commands.try_recv() {
                let id = work.config.binding_id.clone();
                let stop = work.cancel.clone();
                workers.insert(id, (stop, spawn(work)));
                continue;
            }
            // Close before the final drain: concurrent senders either fail or
            // have their accepted work receive a terminal status.
            commands.close();
            finish_queued_starts(&mut commands);
            return Ok(());
        }
        tokio::select! {biased;
            _=canceled(cancel)=>break,
            work=commands.recv()=>{let Some(work)=work else {break;};let id=work.config.binding_id.clone();
                if let Some((old_stop,old))=workers.remove(&id) {old_stop.send_replace(true);let _=old.await;}
                let stop=work.cancel.clone();workers.insert(id,(stop,spawn(work)));
            },
            _=tokio::time::sleep(Duration::from_millis(100))=>{}
        }
    }
    for (stop, _) in workers.values() {
        stop.send_replace(true);
    }
    for (_, (_, task)) in workers {
        let _ = task.await;
    }
    commands.close();
    finish_queued_starts(&mut commands);
    Err(RuntimeError::Stopped)
}
fn finish_queued_starts(commands: &mut mpsc::UnboundedReceiver<BindingWork>) {
    while let Ok(work) = commands.try_recv() {
        let mut status = RuntimeStatus::new(&work.config.agent_id, "stopped", None);
        status.binding_id = Some(work.config.binding_id);
        work.status.send_replace(status);
    }
}
fn same_lease(event: &inbox::Dispatch, lease: &LeaseRef) -> bool {
    event.agent_id == lease.agent_id && event.dispatch_epoch == Some(lease.epoch)
}
#[cfg(test)]
async fn execution_loop<T: Transport>(
    console: &T,
    owner: &str,
    ledger: &mut Ledger,
    config: &StartConfig,
    lease: &LeaseRef,
    cancel: &mut watch::Receiver<bool>,
    status: &watch::Sender<RuntimeStatus>,
) -> Result<(), RuntimeError> {
    execution_loop_with_gate(console, owner, ledger, config, lease, cancel, status, true).await
}
#[allow(clippy::too_many_arguments)]
async fn execution_loop_with_gate<T: Transport>(
    console: &T,
    owner: &str,
    ledger: &mut Ledger,
    config: &StartConfig,
    lease: &LeaseRef,
    cancel: &mut watch::Receiver<bool>,
    status: &watch::Sender<RuntimeStatus>,
    models_allowed: bool,
) -> Result<(), RuntimeError> {
    let mut reported = BTreeSet::new();
    loop {
        if *cancel.borrow() {
            return Err(RuntimeError::Stopped);
        }
        // Original durable output survives restart; never regenerate it.
        for record in local(ledger.inbox_for_binding(
            owner,
            &config.agent_id,
            &config.binding_id,
            State::ReplyReady,
            1000,
        ))? {
            if record.dispatch.binding_id != config.binding_id
                || record.dispatch.agent_id != lease.agent_id
                || record
                    .dispatch
                    .dispatch_epoch
                    .is_none_or(|epoch| epoch > lease.epoch)
            {
                continue;
            }
            send_reply(console, owner, ledger, lease, cancel, &record).await?;
        }
        for record in local(ledger.inbox_for_binding(
            owner,
            &config.agent_id,
            &config.binding_id,
            State::Unknown,
            1000,
        ))? {
            if record.dispatch.binding_id != config.binding_id
                || !same_lease(&record.dispatch, lease)
                || reported.contains(&record.dispatch.id)
            {
                continue;
            }
            if let Some(execution) = record.execution_id {
                operation(
                    console,
                    cancel,
                    Op::Finish {
                        lease: lease.clone(),
                        dispatch_id: record.dispatch.id.clone(),
                        execution_id: execution,
                        outcome: Outcome::Unknown,
                    },
                )
                .await?;
                reported.insert(record.dispatch.id);
            }
        }
        if !models_allowed {
            let mut blocked = RuntimeStatus::new(
                &config.agent_id,
                "ledger_recovery_required",
                Some("ledger_recovery_required"),
            )
            .with_host_files(config.host_files);
            blocked.binding_id = Some(config.binding_id.clone());
            status.send_replace(blocked);
            tokio::select! {biased;_=canceled(cancel)=>return Err(RuntimeError::Stopped),_=tokio::time::sleep(Duration::from_millis(250))=>{}}
            continue;
        }
        let Response::Events(events) = operation(
            console,
            cancel,
            Op::Poll {
                lease: lease.clone(),
                binding_id: config.binding_id.clone(),
                limit: 1,
            },
        )
        .await?
        else {
            return Err(RuntimeError::Transport);
        };
        for event in events {
            let event = convert(event)?;
            if event.binding_id != config.binding_id {
                return Err(RuntimeError::Transport);
            }
            let scope = event.scope();
            // Poll transport has verified current owner/device/Agent and lease.
            local(ledger.register_binding(owner, &scope))?;
            local(ledger.receive_dispatch(owner, &event, inbox::Limits::default(), now()))?;
            operation(
                console,
                cancel,
                Op::Ack {
                    lease: lease.clone(),
                    dispatch_id: event.id.clone(),
                },
            )
            .await?;
            local(ledger.acknowledge_dispatch(owner, &event.id))?;
        }
        for record in local(ledger.inbox_for_binding(
            owner,
            &config.agent_id,
            &config.binding_id,
            State::Acknowledged,
            1000,
        ))? {
            if record.dispatch.binding_id != config.binding_id
                || !same_lease(&record.dispatch, lease)
            {
                continue;
            }
            let scope = record.dispatch.scope();
            // Profile changes require another explicit owner start.
            let profile = local(ledger.model_profile(&scope))?.ok_or(RuntimeError::Profile)?;
            if profile.model != config.profile.model
                || profile.credential_ref != config.credential_ref
                || profile.workspace_root != config.profile.cwd.to_string_lossy()
            {
                return Err(RuntimeError::Profile);
            }
            if local(ledger.inbox_available_bytes(owner))? < 65_536 {
                return Err(RuntimeError::Profile);
            }
            let prepared = local(ledger.prepare_execution(owner, &record.dispatch.id))?;
            let start = operation(
                console,
                cancel,
                Op::Start {
                    lease: lease.clone(),
                    dispatch_id: prepared.dispatch.id.clone(),
                    execution_id: prepared.execution_id.clone(),
                },
            )
            .await;
            let result = match start {
                Ok(Response::Started(start)) => {
                    let response = inbox::ServerStart {
                        dispatch: convert(start.dispatch)?,
                        newly_started: start.newly_started,
                    };
                    let Some(permit) = local(ledger.confirm_execution_start(owner, &response))?
                    else {
                        continue;
                    };
                    run_turn(
                        console,
                        lease,
                        ledger,
                        owner,
                        config,
                        permit.into_prepared(),
                        cancel,
                        status,
                    )
                    .await
                }
                _ => Err(RuntimeError::Transport),
            };
            let provider_lost = matches!(&result, Err(RuntimeError::Provider));
            let authorization_lost = matches!(&result, Err(RuntimeError::Authorization));
            if let Ok(Some(finish)) = result {
                local(ledger.finish_local_execution(
                    owner,
                    &prepared.dispatch.id,
                    &prepared.execution_id,
                    finish,
                ))?;
                let outcome = match finish {
                    Finish::Rejected => Outcome::Rejected,
                    Finish::Failed => Outcome::Failed,
                    Finish::Unknown => Outcome::Unknown,
                };
                operation(
                    console,
                    cancel,
                    Op::Finish {
                        lease: lease.clone(),
                        dispatch_id: prepared.dispatch.id.clone(),
                        execution_id: prepared.execution_id.clone(),
                        outcome,
                    },
                )
                .await?;
            } else if result.is_err() {
                // Network lost after start, process cancellation or storage failure
                // cannot authorize a retry of provider side effects.
                local(ledger.finish_local_execution(
                    owner,
                    &prepared.dispatch.id,
                    &prepared.execution_id,
                    Finish::Unknown,
                ))?;
                if *cancel.borrow() {
                    return Err(RuntimeError::Stopped);
                }
                let reported_outcome = operation(
                    console,
                    cancel,
                    Op::Finish {
                        lease: lease.clone(),
                        dispatch_id: prepared.dispatch.id.clone(),
                        execution_id: prepared.execution_id.clone(),
                        outcome: Outcome::Unknown,
                    },
                )
                .await;
                if provider_lost {
                    return Err(RuntimeError::Provider);
                }
                if authorization_lost {
                    return Err(RuntimeError::Authorization);
                }
                reported_outcome?;
                reported.insert(prepared.dispatch.id);
            } else {
                let record = local(ledger.inbox_record(owner, &prepared.dispatch.id))?;
                send_reply(console, owner, ledger, lease, cancel, &record).await?;
            }
            let mut ready = RuntimeStatus::new(&config.agent_id, "online_chat_only", None)
                .with_host_files(config.host_files);
            ready.binding_id = Some(config.binding_id.clone());
            status.send_replace(ready);
        }
        tokio::select! {biased; _=canceled(cancel)=>return Err(RuntimeError::Stopped),_=tokio::time::sleep(Duration::from_secs(1))=>{}}
    }
}
async fn fresh_running<T: Transport>(
    transport: &T,
    lease: &LeaseRef,
    expected: &inbox::Dispatch,
    execution: &str,
    cancel: &mut watch::Receiver<bool>,
) -> bool {
    let response = operation(
        transport,
        cancel,
        Op::AuthorizeTool {
            lease: lease.clone(),
            dispatch_id: expected.id.clone(),
            execution_id: execution.into(),
        },
    )
    .await;
    let Ok(Response::ToolAuthorized(event)) = response else {
        return false;
    };
    let Ok(event) = convert(event) else {
        return false;
    };
    !*cancel.borrow()
        && event.id == expected.id
        && event.scope() == expected.scope()
        && event.event_id == expected.event_id
        && event.body == expected.body
        && event.binding_generation == expected.binding_generation
        && event.dispatch_epoch == Some(lease.epoch)
        && event.dispatch_device_id == expected.dispatch_device_id
        && event.execution_id.as_deref() == Some(execution)
        && event.state == "running"
}
#[cfg(unix)]
struct LiveToolGate {
    console: PinnedConsole,
    lease: LeaseRef,
    expected: inbox::Dispatch,
    execution: String,
    cancel: watch::Receiver<bool>,
}
#[cfg(unix)]
impl codex::HostToolGate for LiveToolGate {
    fn authorize<'a>(
        &'a self,
        dispatch: &'a str,
        execution: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            if *self.cancel.borrow() || dispatch != self.expected.id || execution != self.execution
            {
                return false;
            }
            let mut receiver = self.cancel.clone();
            let response = operation(
                &self.console,
                &mut receiver,
                Op::AuthorizeTool {
                    lease: self.lease.clone(),
                    dispatch_id: dispatch.into(),
                    execution_id: execution.into(),
                },
            )
            .await;
            let Ok(Response::ToolAuthorized(event)) = response else {
                return false;
            };
            let Ok(event) = convert(event) else {
                return false;
            };
            !*self.cancel.borrow()
                && event.id == self.expected.id
                && event.scope() == self.expected.scope()
                && event.event_id == self.expected.event_id
                && event.body == self.expected.body
                && event.binding_generation == self.expected.binding_generation
                && event.dispatch_epoch == Some(self.lease.epoch)
                && event.dispatch_device_id == self.expected.dispatch_device_id
                && event.execution_id.as_deref() == Some(execution)
                && event.state == "running"
        })
    }
}
/// Best-effort cancellation: 5s sampling plus at most 5s for the proof RPC.
/// An unprovable current running scope cancels only this binding's turn.
async fn monitor_running<T: Transport>(
    transport: &T,
    lease: &LeaseRef,
    prepared: &inbox::Prepared,
    mut cancel: watch::Receiver<bool>,
) -> RuntimeError {
    loop {
        tokio::select! {biased;_=canceled(&mut cancel)=>return RuntimeError::Stopped,_=tokio::time::sleep(Duration::from_secs(5))=>{}}
        if !matches!(
            tokio::time::timeout(
                Duration::from_secs(5),
                fresh_running(
                    transport,
                    lease,
                    &prepared.dispatch,
                    &prepared.execution_id,
                    &mut cancel
                )
            )
            .await,
            Ok(true)
        ) {
            return RuntimeError::Authorization;
        }
    }
}
#[allow(clippy::too_many_arguments)]
async fn review_request<T: Transport>(
    transport: &T,
    lease: &LeaseRef,
    ledger: &mut Ledger,
    owner: &str,
    prepared: &inbox::Prepared,
    cwd: &std::path::Path,
    queue: &codex::ApprovalQueue,
    cancel: &mut watch::Receiver<bool>,
) -> Result<bool, RuntimeError> {
    let scope = prepared.dispatch.scope();

    use sha2::{Digest, Sha256};
    let revisions = local(ledger.policy_snapshot(&scope))?.map(|v| v.revision);
    let proposal = hagency_agent_local::ToolProposal {
        scope: scope.clone(),
        dispatch: prepared.dispatch.id.clone(),
        tool: "model.request".into(),
        arguments: serde_json::json!({"executionId":prepared.execution_id,"inputSha256":format!("{:x}",Sha256::digest(prepared.dispatch.body.as_bytes())),"inputPreview":prepared.dispatch.body.chars().take(2048).collect::<String>()}),
        canonical_directory: cwd.to_string_lossy().into(),
        risk: "model_request".into(),
        policy_revision: revisions,
        expires: now() + 300,
    };
    let (pending, decision) = codex::pending_approval(proposal.clone());
    let accepted = queue.offer(pending)
        && tokio::select! {biased;_=canceled(cancel)=>false,r=tokio::time::timeout(Duration::from_secs(300),decision)=>matches!(r,Ok(Ok(true)))};
    Ok(accepted
        && fresh_running(
            transport,
            lease,
            &prepared.dispatch,
            &prepared.execution_id,
            cancel,
        )
        .await
        && ledger
            .approve_request_exact(
                owner,
                &scope,
                &prepared.dispatch.id,
                revisions,
                proposal.expires,
                now(),
            )
            .is_ok())
}
#[allow(clippy::too_many_arguments)]
async fn run_turn<T: Transport>(
    transport: &T,
    lease: &LeaseRef,
    ledger: &mut Ledger,
    owner: &str,
    config: &StartConfig,
    prepared: inbox::Prepared,
    cancel: &mut watch::Receiver<bool>,
    status: &watch::Sender<RuntimeStatus>,
) -> Result<Option<Finish>, RuntimeError> {
    let scope = prepared.dispatch.scope();
    let mut profile = config.profile.clone();
    #[cfg(unix)]
    let workspace = if config.host_files {
        let directory = profile.codex_home.parent().ok_or(RuntimeError::Profile)?;
        let workspace = hagency_agent_local::room_files::Workspace::open(directory, owner, &scope)
            .map_err(|_| RuntimeError::Profile)?;
        profile.cwd = workspace.canonical_directory().to_path_buf();
        Some(workspace)
    } else {
        None
    };
    let mut provider = tokio::select! {biased; _=canceled(cancel)=>return Err(RuntimeError::Stopped), p=profile.spawn()=>p.map_err(|_|RuntimeError::Profile)?};
    let (queue, mut receiver) = codex::approval_queue(1).map_err(|_| RuntimeError::Profile)?;
    let host = transport.approval_host();
    if config.host_files {
        #[cfg(unix)]
        {
            let console = host.as_ref().ok_or(RuntimeError::Profile)?;
            let gate = Arc::new(LiveToolGate {
                console: console.clone(),
                lease: lease.clone(),
                expected: prepared.dispatch.clone(),
                execution: prepared.execution_id.clone(),
                cancel: cancel.clone(),
            });
            provider
                .session
                .enable_host_files(workspace.ok_or(RuntimeError::Profile)?, gate)
                .map_err(|_| RuntimeError::Profile)?;
        }
        #[cfg(not(unix))]
        {
            return Err(RuntimeError::Profile);
        }
    }
    let has_approval_host = host.is_some();
    let bridge = host.map(|console| {
        tokio::spawn(async move {
            while let Some(pending) = receiver.recv().await {
                if console
                    .console
                    .0
                    .owned_runtime
                    .offer_approval(&console.console, &console.pin, pending)
                    .await
                    .is_err()
                {
                    break;
                }
            }
        })
    });
    let _approval_bridge = bridge.map(|task| AbortHeartbeat(task.abort_handle()));
    let disposition = local(ledger.request_disposition(&scope, &prepared.dispatch.id))?;
    let allow = match disposition {
        hagency_agent_local::RequestPolicy::Allow => true,
        hagency_agent_local::RequestPolicy::Deny => false,
        hagency_agent_local::RequestPolicy::AskOwner if !has_approval_host => false,
        hagency_agent_local::RequestPolicy::AskOwner => {
            review_request(
                transport,
                lease,
                ledger,
                owner,
                &prepared,
                &profile.cwd,
                &queue,
                cancel,
            )
            .await?
        }
    };
    if !allow {
        let _ = provider.stop().await;
        return if *cancel.borrow() {
            Err(RuntimeError::Stopped)
        } else {
            Ok(Some(Finish::Rejected))
        };
    }
    let mut executing = RuntimeStatus::new(
        &config.agent_id,
        if config.host_files {
            "executing_host_files"
        } else {
            "executing_chat_only"
        },
        None,
    )
    .with_host_files(config.host_files);
    executing.binding_id = Some(config.binding_id.clone());
    status.send_replace(executing);
    let mut pre_model_cancel = cancel.clone();
    let permission_cancel = cancel.clone();
    let result = tokio::select! {biased; _=canceled(cancel)=>Err(RuntimeError::Stopped), error=monitor_running(transport,lease,&prepared,permission_cancel)=>Err(error), r=async {
        provider.session.initialize().await?;
        provider.session.require_local_account().await?;
        provider.session.open_context(ledger,&scope,&profile).await?;
        if !fresh_running(transport,lease,&prepared.dispatch,&prepared.execution_id,&mut pre_model_cancel).await {return Err(codex::Error::Protocol("running dispatch authorization lost"));}
        provider.session.run(ledger,owner,&scope,&prepared.execution_id,&prepared.dispatch.id,&prepared.dispatch.body,config.mode.clone(),&queue).await
    }=>match r {
        Ok(completed)=>Ok(Some(completed)),
        Err(codex::Error::Ledger(hagency_agent_local::Error::Denied|hagency_agent_local::Error::Budget|hagency_agent_local::Error::Unknown))=>Ok(None),
        Err(codex::Error::Profile("dedicated owner provider login required"))=>Err(RuntimeError::Provider),
        Err(codex::Error::Failed)=>{let _=provider.stop().await;return Ok(Some(Finish::Failed));},
        Err(_)=>Err(RuntimeError::Profile),
    }};
    // Dropping a running turn first seals its reservation; then kill the child.
    let _ = provider.stop().await;
    let Some(completed) = result? else {
        return Ok(Some(Finish::Rejected));
    };
    if completed.text.is_empty() || completed.text.len() > 65_536 {
        return Err(RuntimeError::Profile);
    }
    local(ledger.persist_execution_reply(
        owner,
        &prepared.dispatch.id,
        &prepared.execution_id,
        &completed.text,
    ))?;
    Ok(None)
}
async fn send_reply<T: Transport>(
    console: &T,
    owner: &str,
    ledger: &mut Ledger,
    lease: &LeaseRef,
    cancel: &mut watch::Receiver<bool>,
    record: &inbox::Record,
) -> Result<(), RuntimeError> {
    let execution = record.execution_id.clone().ok_or(RuntimeError::Profile)?;
    let body = record.reply.clone().ok_or(RuntimeError::Profile)?;
    let receipt = loop {
        match operation(
            console,
            cancel,
            if record.dispatch.dispatch_epoch == Some(lease.epoch) {
                Op::Reply {
                    lease: lease.clone(),
                    dispatch_id: record.dispatch.id.clone(),
                    execution_id: execution.clone(),
                    body: body.clone(),
                }
            } else {
                Op::ReconcileKnownReply {
                    lease: lease.clone(),
                    known: super::device_execution::KnownReply {
                        dispatch_id: record.dispatch.id.clone(),
                        execution_id: execution.clone(),
                        body: body.clone(),
                        binding_id: record.dispatch.binding_id.clone(),
                        room_id: record.dispatch.room_id.clone(),
                        requester_mxid: record.dispatch.requester_mxid.clone(),
                        thread_root: record.dispatch.thread_root.clone(),
                        binding_generation: record.dispatch.binding_generation,
                        original_epoch: record
                            .dispatch
                            .dispatch_epoch
                            .ok_or(RuntimeError::Profile)?,
                    },
                }
            },
        )
        .await
        {
            Ok(Response::ReplyQueued(receipt)) => break receipt,
            Err(RuntimeError::Transport) => {
                tokio::select! {biased;_=canceled(cancel)=>return Err(RuntimeError::Stopped),_=tokio::time::sleep(Duration::from_secs(1))=>{}}
            }
            Err(error) => return Err(error),
            _ => return Err(RuntimeError::Transport),
        }
    };
    if receipt.binding_id != record.dispatch.binding_id
        || receipt.room_id != record.dispatch.room_id
        || receipt.binding_generation != record.dispatch.binding_generation
    {
        return Err(RuntimeError::Transport);
    }
    // Durable server acceptance can be cancelled after device lease expiry.
    // Keep the immutable body retryable until a Matrix event proves delivery.
    if receipt.state == "sent"
        && receipt
            .matrix_event_id
            .as_deref()
            .is_some_and(|id| !id.is_empty())
    {
        local(ledger.confirm_matrix_delivery(owner, &record.dispatch.id, &execution))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::device_execution::{ExecutionStart, ReplyReceipt};
    use super::*;
    use std::sync::Mutex as StdMutex;
    const OWNER: &str = "@alice:test";
    #[test]
    fn two_process_like_hosts_cannot_share_one_agent_context() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("ledger.db");
        let first = lock_agent(&path, "agent-a").unwrap();
        assert!(matches!(
            lock_agent(&path, "agent-a"),
            Err(RuntimeError::AlreadyActive)
        ));
        assert!(lock_agent(&path, "agent-b").is_ok());
        drop(first);
        assert!(lock_agent(&path, "agent-a").is_ok());
    }
    #[tokio::test]
    async fn multi_profile_revoked_inflight_registration_cannot_reenter_cleared_registry() {
        let runtime = OwnedRuntime::default();
        let (old, revoked) = super::super::server_login::fixture_device(
            "https://server.example/",
            "subject-a",
            OWNER,
        );
        let stale_logout = old.clone();
        let guard = runtime.entries.lock().await;
        let worker = runtime.clone();
        let queued = tokio::spawn(async move {
            let _guard = worker.entries_for(&old).await?;
            Ok::<_, RuntimeError>(())
        });
        tokio::task::yield_now().await;
        revoked.store(true, std::sync::atomic::Ordering::Release);
        drop(guard);
        runtime.stop_all().await;
        assert!(matches!(
            queued.await.unwrap(),
            Err(RuntimeError::Authorization)
        ));
        assert!(runtime.entries.lock().await.is_empty());
        let (fresh, _) = super::super::server_login::fixture_device(
            "https://server.example/",
            "subject-b",
            OWNER,
        );
        assert!(runtime.entries_for(&fresh).await.is_ok());
        // A -> B -> A: the new A session must survive delayed old-A logout.
        let (returned_a, _) = super::super::server_login::fixture_device(
            "https://server.example/",
            "subject-a",
            OWNER,
        );
        let (cancel, mut receiver) = watch::channel(false);
        let (_, status) = watch::channel(RuntimeStatus::new("agent-a", "test", None));
        let task = tokio::spawn(async move { canceled(&mut receiver).await });
        runtime.entries_for(&returned_a).await.unwrap().insert(
            "returned-a".into(),
            Entry {
                commands: mpsc::unbounded_channel().0,
                bindings: BTreeMap::new(),
                cancel,
                status,
                task,
                device: DevicePin::from(&returned_a),
                approvals: BTreeMap::new(),
            },
        );
        assert!(matches!(
            runtime.stop_profile(&stale_logout).await,
            Err(RuntimeError::Authorization)
        ));
        let entries = runtime.entries.lock().await;
        assert!(entries.contains_key("returned-a"));
        assert!(!*entries["returned-a"].cancel.borrow());
        drop(entries);
        runtime.stop_profile(&returned_a).await.unwrap();
        assert!(runtime.entries.lock().await.is_empty());
    }
    #[tokio::test]
    async fn retire_while_registry_locked_cancels_worker_without_blocking() {
        let runtime = OwnedRuntime::default();
        let (cancel, mut receiver) = watch::channel(false);
        let (status, status_receiver) = watch::channel(RuntimeStatus::new("agent-a", "test", None));
        let (done, finished) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            canceled(&mut receiver).await;
            let _ = done.send(());
            drop(status);
        });
        let mut guard = runtime.entries.lock().await;
        guard.insert(
            "test".into(),
            Entry {
                commands: mpsc::unbounded_channel().0,
                bindings: BTreeMap::new(),
                cancel,
                status: status_receiver,
                task,
                device: DevicePin {
                    origin: "test".into(),
                    issuer: "test-issuer".into(),
                    subject: "test-subject".into(),
                    owner: "test".into(),
                    user: "test".into(),
                    id: "test-device".into(),
                    generation: 1,
                },
                approvals: BTreeMap::new(),
            },
        );
        runtime.request_stop_all();
        drop(guard);
        tokio::time::timeout(Duration::from_secs(2), finished)
            .await
            .unwrap()
            .unwrap();
        runtime.stop_all().await;
        assert!(runtime.entries.lock().await.is_empty());
    }
    fn event() -> inbox::Dispatch {
        inbox::Dispatch {
            id: "dispatch-a".into(),
            binding_id: "binding-a".into(),
            agent_id: "agent-a".into(),
            event_id: "$event-a:test".into(),
            room_id: "!room-a:test".into(),
            requester_mxid: "@bob:test".into(),
            thread_root: "$root:test".into(),
            body: "request".into(),
            state: "offered".into(),
            binding_generation: 1,
            dispatch_epoch: Some(1),
            dispatch_device_id: Some("device-a".into()),
            execution_id: None,
            outcome: None,
        }
    }
    fn wire(event: &inbox::Dispatch) -> super::super::device_execution::Dispatch {
        serde_json::from_value(serde_json::to_value(event).unwrap()).unwrap()
    }
    fn lease() -> LeaseRef {
        LeaseRef {
            agent_id: "agent-a".into(),
            epoch: 1,
        }
    }
    fn config(path: &std::path::Path) -> StartConfig {
        StartConfig {
            agent_id: "agent-a".into(),
            binding_id: "binding-a".into(),
            profile: Profile {
                executable: path.join("must-not-spawn"),
                home: path.into(),
                codex_home: path.into(),
                cwd: path.into(),
                model: "test-model".into(),
                effort: "low".into(),
            },
            mode: BudgetMode::Estimated { reservation: 10 },
            credential_ref: "keychain:test".into(),
            takeover: Takeover::Never,
            host_files: false,
        }
    }
    fn setup(path: &std::path::Path, root: &std::path::Path) -> Ledger {
        let mut ledger = Ledger::open(path, OWNER).unwrap();
        ledger.register_binding(OWNER, &event().scope()).unwrap();
        ledger
            .set_model_profile(
                OWNER,
                &event().scope(),
                &hagency_agent_local::ModelProfile {
                    model: "test-model".into(),
                    workspace_root: root.to_string_lossy().into(),
                    credential_ref: "keychain:test".into(),
                },
            )
            .unwrap();
        ledger
    }
    #[derive(Clone, Copy)]
    enum Mode {
        LostStart,
        ExistingStart,
        ReplyRetry,
        CancelModel,
        RevokedModel,
        IdleOtherRoom,
        CrossEpochReply,
        AcceptedOnly,
    }
    struct Fake {
        path: PathBuf,
        mode: Mode,
        calls: StdMutex<Vec<&'static str>>,
        stop: watch::Sender<bool>,
    }
    impl Transport for Fake {
        async fn execute(&self, op: Op) -> Result<Response, RuntimeError> {
            let recovering = matches!(op, Op::ReconcileKnownReply { .. });
            let delivery_epoch = if recovering { 2 } else { 1 };
            let op = match op {
                Op::ReconcileKnownReply { lease, known } => {
                    assert_eq!(known.original_epoch, 1);
                    assert_eq!(known.binding_generation, 1);
                    assert_eq!(known.binding_id, "binding-a");
                    assert_eq!(known.room_id, "!room-a:test");
                    assert_eq!(known.requester_mxid, "@bob:test");
                    assert_eq!(known.thread_root, "$root:test");
                    Op::Reply {
                        lease,
                        dispatch_id: known.dispatch_id,
                        execution_id: known.execution_id,
                        body: known.body,
                    }
                }
                op => op,
            };
            let mut calls = self.calls.lock().unwrap();
            let ledger = Ledger::open(&self.path, OWNER).unwrap();
            match op {
                Op::Poll { binding_id, .. } => {
                    assert_eq!(
                        binding_id,
                        if matches!(self.mode, Mode::IdleOtherRoom) {
                            "binding-b"
                        } else {
                            "binding-a"
                        }
                    );
                    calls.push("poll");
                    if matches!(self.mode, Mode::IdleOtherRoom) {
                        return Ok(Response::Events(vec![]));
                    }
                    if matches!(self.mode, Mode::ReplyRetry | Mode::CrossEpochReply) {
                        assert_eq!(
                            ledger.inbox_record(OWNER, "dispatch-a").unwrap().state,
                            State::Replied
                        );
                        self.stop.send_replace(true);
                        return Ok(Response::Events(vec![]));
                    }
                    if calls.iter().filter(|c| **c == "poll").count() > 1 {
                        return Ok(Response::Events(vec![]));
                    }
                    Ok(Response::Events(vec![wire(&event())]))
                }
                Op::Ack { dispatch_id, .. } => {
                    assert_eq!(
                        ledger.inbox_record(OWNER, &dispatch_id).unwrap().state,
                        State::Received
                    );
                    calls.push("ack");
                    Ok(Response::Acknowledged)
                }
                Op::Start {
                    dispatch_id,
                    execution_id,
                    ..
                } => {
                    let record = ledger.inbox_record(OWNER, &dispatch_id).unwrap();
                    assert_eq!(record.state, State::Prepared);
                    assert_eq!(record.execution_id.as_ref(), Some(&execution_id));
                    calls.push("start");
                    if matches!(self.mode, Mode::LostStart) {
                        return Err(RuntimeError::Transport);
                    }
                    let mut dispatch = event();
                    dispatch.execution_id = Some(execution_id);
                    dispatch.state = "running".into();
                    Ok(Response::Started(ExecutionStart {
                        dispatch: wire(&dispatch),
                        newly_started: matches!(self.mode, Mode::CancelModel | Mode::RevokedModel),
                    }))
                }
                Op::AuthorizeTool {
                    dispatch_id,
                    execution_id,
                    ..
                } => {
                    calls.push("authorize");
                    if matches!(self.mode, Mode::RevokedModel)
                        && calls.iter().filter(|c| **c == "authorize").count() > 1
                    {
                        return Err(RuntimeError::Authorization);
                    }
                    let mut active = event();
                    assert_eq!(dispatch_id, active.id);
                    active.state = "running".into();
                    active.execution_id = Some(execution_id);
                    Ok(Response::ToolAuthorized(
                        serde_json::from_value(serde_json::to_value(active).unwrap()).unwrap(),
                    ))
                }
                Op::Finish {
                    dispatch_id,
                    outcome,
                    ..
                } => {
                    assert!(matches!(outcome, Outcome::Unknown));
                    assert_eq!(
                        ledger.inbox_record(OWNER, &dispatch_id).unwrap().state,
                        State::Unknown
                    );
                    calls.push("unknown");
                    if !matches!(self.mode, Mode::RevokedModel) {
                        self.stop.send_replace(true);
                    }
                    Ok(Response::Finished)
                }
                Op::Reply {
                    dispatch_id,
                    execution_id,
                    body,
                    ..
                } => {
                    assert_eq!(body, "original response");
                    let record = ledger.inbox_record(OWNER, &dispatch_id).unwrap();
                    assert_eq!(record.state, State::ReplyReady);
                    assert_eq!(record.reply.as_deref(), Some("original response"));
                    let label = if recovering { "reconcile" } else { "reply" };
                    calls.push(label);
                    if !matches!(self.mode, Mode::AcceptedOnly)
                        && calls.iter().filter(|c| **c == label).count() == 1
                    {
                        return Err(RuntimeError::Transport);
                    }
                    Ok(Response::ReplyQueued(ReplyReceipt {
                        id: "reply-a".into(),
                        owner_event_id: dispatch_id.clone(),
                        agent_id: "agent-a".into(),
                        binding_id: "binding-a".into(),
                        owner_user_id: "user-a".into(),
                        room_id: "!room-a:test".into(),
                        puppet_mxid: "@agent:test".into(),
                        binding_generation: 1,
                        dispatch_epoch: 1,
                        delivery_epoch,
                        requester_mxid: "@bob:test".into(),
                        thread_root: "$root:test".into(),
                        body: body.clone(),
                        payload_digest: {
                            use sha2::{Digest, Sha256};
                            format!("{:x}",Sha256::digest(format!("{{\"dispatchId\":\"{dispatch_id}\",\"executionId\":\"{execution_id}\",\"body\":\"{body}\"}}").as_bytes()))
                        },
                        matrix_txn_id: {
                            use sha2::{Digest, Sha256};
                            format!("hagency_{:x}", Sha256::digest(dispatch_id.as_bytes()))
                        },
                        state: if matches!(self.mode, Mode::AcceptedOnly) {
                            "pending"
                        } else {
                            "sent"
                        }
                        .into(),
                        matrix_event_id: if matches!(self.mode, Mode::AcceptedOnly) {
                            None
                        } else {
                            Some("$sent:test".into())
                        },
                    }))
                }
                _ => panic!("unexpected transport call"),
            }
        }
    }
    async fn fixture(mode: Mode) -> (Vec<&'static str>, State) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("ledger.db");
        let mut ledger = setup(&path, tmp.path());
        if matches!(mode, Mode::ReplyRetry | Mode::CrossEpochReply) {
            ledger
                .receive_dispatch(OWNER, &event(), inbox::Limits::default(), now())
                .unwrap();
            ledger.acknowledge_dispatch(OWNER, "dispatch-a").unwrap();
            let prepared = ledger.prepare_execution(OWNER, "dispatch-a").unwrap();
            let mut event = prepared.dispatch.clone();
            event.state = "running".into();
            event.execution_id = Some(prepared.execution_id.clone());
            ledger
                .confirm_execution_start(
                    OWNER,
                    &inbox::ServerStart {
                        dispatch: event,
                        newly_started: true,
                    },
                )
                .unwrap()
                .unwrap();
            ledger
                .persist_execution_reply(
                    OWNER,
                    "dispatch-a",
                    &prepared.execution_id,
                    "original response",
                )
                .unwrap();
            drop(ledger);
            ledger = Ledger::open(&path, OWNER).unwrap();
            ledger.recover_agent_executions(OWNER, "agent-a").unwrap();
        }
        let (stop, mut receiver) = watch::channel(false);
        let fake = Fake {
            path,
            mode,
            calls: StdMutex::new(vec![]),
            stop,
        };
        let (status, _rx) = watch::channel(RuntimeStatus::new("agent-a", "test", None));
        assert!(matches!(
            tokio::time::timeout(
                Duration::from_secs(5),
                execution_loop(
                    &fake,
                    OWNER,
                    &mut ledger,
                    &config(tmp.path()),
                    &LeaseRef {
                        agent_id: "agent-a".into(),
                        epoch: if matches!(mode, Mode::CrossEpochReply) {
                            2
                        } else {
                            1
                        }
                    },
                    &mut receiver,
                    &status
                )
            )
            .await
            .unwrap(),
            Err(RuntimeError::Stopped)
        ));
        let calls = fake.calls.lock().unwrap().clone();
        (
            calls,
            ledger.inbox_record(OWNER, "dispatch-a").unwrap().state,
        )
    }
    #[tokio::test]
    async fn last_worker_completion_consumes_already_accepted_new_binding_start() {
        let tmp = tempfile::tempdir().unwrap();
        let first = config(tmp.path());
        let mut next = first.clone();
        next.binding_id = "binding-b".into();
        let (initial, _, _) = binding_work(first);
        let (work, second, _) = binding_work(next);
        let (commands, rx) = mpsc::unbounded_channel();
        assert!(commands.send(work).is_ok());
        let completed = tokio::spawn(async {});
        while !completed.is_finished() {
            tokio::task::yield_now().await;
        }
        let mut completed = Some(completed);
        let (stop, mut cancel) = watch::channel(false);
        let (began, mut started) = mpsc::unbounded_channel();
        let supervisor = tokio::spawn(async move {
            supervise_bindings(initial, rx, &mut cancel, |mut work| {
                if let Some(done) = completed.take() {
                    return done;
                }
                assert_eq!(work.config.binding_id, "binding-b");
                let _ = began.send(());
                tokio::spawn(async move {
                    canceled(&mut work.receiver).await;
                    let mut status = RuntimeStatus::new("agent-a", "stopped", None);
                    status.binding_id = Some(work.config.binding_id);
                    work.status.send_replace(status);
                })
            })
            .await
        });
        tokio::time::timeout(Duration::from_secs(2), started.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(!supervisor.is_finished());
        stop.send_replace(true);
        assert!(matches!(
            supervisor.await.unwrap(),
            Err(RuntimeError::Stopped)
        ));
        assert_eq!(second.status.borrow().phase, "stopped");
        assert!(commands.is_closed());
    }
    #[tokio::test]
    async fn closing_supervisor_seals_queued_binding_start_instead_of_leaving_starting() {
        let tmp = tempfile::tempdir().unwrap();
        let first = config(tmp.path());
        let mut next = first.clone();
        next.binding_id = "binding-b".into();
        let (initial, _, _) = binding_work(first);
        let (work, second, _) = binding_work(next);
        let (commands, rx) = mpsc::unbounded_channel();
        assert!(commands.send(work).is_ok());
        let (_stop, mut cancel) = watch::channel(true);
        assert!(matches!(
            supervise_bindings(initial, rx, &mut cancel, |mut work| tokio::spawn(
                async move {
                    canceled(&mut work.receiver).await;
                }
            ))
            .await,
            Err(RuntimeError::Stopped)
        ));
        assert_eq!(second.status.borrow().phase, "stopped");
        assert!(commands.is_closed());
        assert!(commands.send(binding_work(config(tmp.path())).0).is_err());
    }
    #[derive(Clone)]
    struct SharedLeaseTransport {
        polls: Arc<StdMutex<Vec<(String, i64)>>>,
        fenced: Arc<StdMutex<BTreeSet<String>>>,
    }
    impl Transport for SharedLeaseTransport {
        async fn execute(&self, op: Op) -> Result<Response, RuntimeError> {
            let Op::Poll {
                lease, binding_id, ..
            } = op
            else {
                panic!(
                    "Room workers cannot acquire/renew/release shared lease or start unoffered work"
                )
            };
            assert_eq!(lease.agent_id, "agent-a");
            assert_eq!(lease.epoch, 77);
            self.polls
                .lock()
                .unwrap()
                .push((binding_id.clone(), lease.epoch));
            if self.fenced.lock().unwrap().contains(&binding_id) {
                Err(RuntimeError::Authorization)
            } else {
                Ok(Response::Events(vec![]))
            }
        }
    }
    async fn wait_polls(transport: &SharedLeaseTransport, binding: &str, minimum: usize) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if transport
                    .polls
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|(id, _)| id == binding)
                    .count()
                    >= minimum
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
    async fn multi_binding_fixture(fence: bool) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let path = root.join("ledger");
        drop(setup(&path, &root));
        let transport = SharedLeaseTransport {
            polls: Arc::new(StdMutex::new(vec![])),
            fenced: Arc::new(StdMutex::new(BTreeSet::new())),
        };
        let a = config(&root);
        let mut b = a.clone();
        b.binding_id = "binding-b".into();
        b.mode = BudgetMode::Estimated { reservation: 91 };
        b.host_files = true;
        b.profile.effort = "high".into();
        let (initial, first, _) = binding_work(a);
        let (next, second, _) = binding_work(b);
        let (commands, rx) = mpsc::unbounded_channel();
        assert!(commands.send(next).is_ok());
        let (stop, mut cancel) = watch::channel(false);
        let wire = transport.clone();
        let seen = Arc::new(StdMutex::new(vec![]));
        let captured = seen.clone();
        let supervisor = tokio::spawn(async move {
            supervise_bindings(initial, rx, &mut cancel, |mut work| {
                captured.lock().unwrap().push((
                    work.config.binding_id.clone(),
                    work.config.host_files,
                    work.config.profile.effort.clone(),
                    match work.config.mode {
                        BudgetMode::Estimated { reservation } => reservation,
                        _ => 0,
                    },
                ));
                let wire = wire.clone();
                let path = path.clone();
                tokio::spawn(async move {
                    let mut ledger = Ledger::open(&path, OWNER).unwrap();
                    let result = execution_loop(
                        &wire,
                        OWNER,
                        &mut ledger,
                        &work.config,
                        &LeaseRef {
                            agent_id: "agent-a".into(),
                            epoch: 77,
                        },
                        &mut work.receiver,
                        &work.status,
                    )
                    .await;
                    let mut status =
                        RuntimeStatus::new(&work.config.agent_id, "stopped", failure_code(&result));
                    status.binding_id = Some(work.config.binding_id);
                    work.status.send_replace(status);
                })
            })
            .await
        });
        wait_polls(&transport, "binding-a", 1).await;
        wait_polls(&transport, "binding-b", 1).await;
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                ("binding-a".into(), false, "low".into(), 10),
                ("binding-b".into(), true, "high".into(), 91)
            ]
        );
        let before_b = transport
            .polls
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, _)| id == "binding-b")
            .count();
        if fence {
            transport.fenced.lock().unwrap().insert("binding-a".into());
        } else {
            first.cancel.send_replace(true);
        }
        let mut astatus = first.status.clone();
        tokio::time::timeout(Duration::from_secs(5), async {
            while astatus.borrow().phase != "stopped" {
                astatus.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        assert_eq!(
            astatus.borrow().last_error,
            if fence {
                Some("authorization_or_lease_lost")
            } else {
                None
            }
        );
        let a_polls = transport
            .polls
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, _)| id == "binding-a")
            .count();
        wait_polls(&transport, "binding-b", before_b + 2).await;
        assert_eq!(
            transport
                .polls
                .lock()
                .unwrap()
                .iter()
                .filter(|(id, _)| id == "binding-a")
                .count(),
            a_polls
        );
        assert!(!supervisor.is_finished());
        assert!(!*second.cancel.borrow());
        assert!(!*stop.borrow());
        // Shared owner/device cancellation joins all remaining workers before release.
        stop.send_replace(true);
        assert!(matches!(
            supervisor.await.unwrap(),
            Err(RuntimeError::Stopped)
        ));
        assert_eq!(second.status.borrow().phase, "stopped");
        assert!(
            transport
                .polls
                .lock()
                .unwrap()
                .iter()
                .all(|(_, epoch)| *epoch == 77)
        );
    }
    #[tokio::test]
    async fn simultaneous_rooms_share_one_epoch_without_worker_acquire_and_stop_independently() {
        multi_binding_fixture(false).await;
    }
    #[tokio::test]
    async fn room_permission_fence_stops_only_that_binding_and_owner_stop_joins_rest() {
        multi_binding_fixture(true).await;
    }
    #[tokio::test]
    async fn selected_binding_never_runs_or_recovers_other_room_inbox_records() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("ledger.db");
        let mut ledger = setup(&path, tmp.path());
        for (id, target) in [
            ("other-acked", State::Acknowledged),
            ("other-reply", State::ReplyReady),
            ("other-unknown", State::Unknown),
        ] {
            let mut other = event();
            other.id = id.into();
            other.event_id = format!("${id}:test");
            other.binding_id = "binding-b".into();
            other.room_id = "!room-b:test".into();
            ledger.register_binding(OWNER, &other.scope()).unwrap();
            ledger
                .receive_dispatch(OWNER, &other, inbox::Limits::default(), 1)
                .unwrap();
            ledger.acknowledge_dispatch(OWNER, id).unwrap();
            if target != State::Acknowledged {
                let prepared = ledger.prepare_execution(OWNER, id).unwrap();
                other.execution_id = Some(prepared.execution_id.clone());
                other.state = "running".into();
                ledger
                    .confirm_execution_start(
                        OWNER,
                        &inbox::ServerStart {
                            dispatch: other,
                            newly_started: true,
                        },
                    )
                    .unwrap()
                    .unwrap();
                if target == State::ReplyReady {
                    ledger
                        .persist_execution_reply(
                            OWNER,
                            id,
                            &prepared.execution_id,
                            "other original response",
                        )
                        .unwrap();
                } else {
                    ledger
                        .finish_local_execution(OWNER, id, &prepared.execution_id, Finish::Unknown)
                        .unwrap();
                }
            }
        }
        let (stop, mut receiver) = watch::channel(false);
        let fake = Fake {
            path,
            mode: Mode::LostStart,
            calls: StdMutex::new(vec![]),
            stop,
        };
        let (status, _) = watch::channel(RuntimeStatus::new("agent-a", "test", None));
        assert!(matches!(
            execution_loop(
                &fake,
                OWNER,
                &mut ledger,
                &config(tmp.path()),
                &lease(),
                &mut receiver,
                &status
            )
            .await,
            Err(RuntimeError::Stopped)
        ));
        assert_eq!(
            *fake.calls.lock().unwrap(),
            vec!["poll", "ack", "start", "unknown"]
        );
        assert_eq!(
            ledger.inbox_record(OWNER, "other-acked").unwrap().state,
            State::Acknowledged
        );
        assert_eq!(
            ledger.inbox_record(OWNER, "other-reply").unwrap().state,
            State::ReplyReady
        );
        assert_eq!(
            ledger.inbox_record(OWNER, "other-unknown").unwrap().state,
            State::Unknown
        );
        assert_eq!(
            ledger
                .inbox_record(OWNER, "other-reply")
                .unwrap()
                .reply
                .as_deref(),
            Some("other original response")
        );
    }
    struct WrongBinding;
    impl Transport for WrongBinding {
        async fn execute(&self, op: Op) -> Result<Response, RuntimeError> {
            let Op::Poll { binding_id, .. } = op else {
                panic!("foreign binding must never be acknowledged or executed")
            };
            assert_eq!(binding_id, "binding-a");
            let mut other = event();
            other.binding_id = "binding-b".into();
            other.room_id = "!room-b:test".into();
            Ok(Response::Events(vec![wire(&other)]))
        }
    }
    #[tokio::test]
    async fn foreign_binding_poll_response_is_rejected_before_inbox_ack_or_model() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ledger = setup(&tmp.path().join("ledger"), tmp.path());
        let (_stop, mut receiver) = watch::channel(false);
        let (status, _) = watch::channel(RuntimeStatus::new("agent-a", "test", None));
        assert!(matches!(
            execution_loop(
                &WrongBinding,
                OWNER,
                &mut ledger,
                &config(tmp.path()),
                &lease(),
                &mut receiver,
                &status
            )
            .await,
            Err(RuntimeError::Transport)
        ));
        assert!(ledger.inbox_record(OWNER, "dispatch-a").is_err());
    }
    #[tokio::test]
    async fn lost_start_response_seals_unknown_without_spawning_provider() {
        let (calls, state) = fixture(Mode::LostStart).await;
        assert_eq!(calls, vec!["poll", "ack", "start", "unknown"]);
        assert_eq!(state, State::Unknown);
    }
    #[tokio::test]
    async fn server_existing_start_never_replays_model() {
        let (calls, state) = fixture(Mode::ExistingStart).await;
        assert_eq!(calls, vec!["poll", "ack", "start", "unknown"]);
        assert_eq!(state, State::Unknown);
    }
    #[tokio::test]
    async fn original_reply_retries_after_restart_and_lost_transport_response() {
        let (calls, state) = fixture(Mode::ReplyRetry).await;
        assert_eq!(calls, vec!["reply", "reply", "poll"]);
        assert_eq!(state, State::Replied);
    }
    #[cfg(unix)]
    fn install_long_turn_provider(script: &std::path::Path, marker: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        let source=r##"#!/usr/bin/python3
import sys,json,time,os
marker=MARKER
for raw in sys.stdin:
    frame=json.loads(raw); method=frame.get('method'); ident=frame.get('id')
    def reply(result):
        print(json.dumps({'id':ident,'result':result}),flush=True)
    if method=='initialize': reply({})
    elif method=='initialized': pass
    elif method=='thread/name/set': reply({})
    elif method=='config/read':
        config={'project_doc_max_bytes':0,'skills':{'include_instructions':False,'bundled':{'enabled':False}},'include_apps_instructions':False,'include_environment_context':False,'developer_instructions':'','web_search':'disabled','mcp_servers':{},'features':{}}
        for name in ['shell_tool','unified_exec','code_mode_host','code_mode','hooks','plugins','multi_agent','multi_agent_v2','skill_search','skill_mcp_dependency_install','shell_snapshot','view_image','image_generation','apps','tool_search','tool_suggest','web_search','web_search_cached','web_search_request','standalone_web_search','memory_tool']: config['features'][name]=False
        reply({'config':config})
    elif method=='account/read': reply({'requiresOpenaiAuth':True,'account':{'type':'chatgpt'}})
    elif method=='thread/start':
        p=frame['params'];reply({'thread':{'id':'thread-a','cwd':p['cwd']},'cwd':p['cwd'],'model':p['model'],'approvalPolicy':'untrusted','approvalsReviewer':'user','sandbox':{'type':'readOnly','networkAccess':False}})
    elif method=='turn/start':
        open(marker,'w').write(str(os.getpid()))
        reply({'turn':{'id':'turn-a','status':'inProgress'}})
        while True: time.sleep(10)
    else: raise Exception(method)
"##.replace("MARKER",&serde_json::to_string(marker.to_str().unwrap()).unwrap());
        std::fs::write(script, source).unwrap();
        std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    impl Transport for Arc<Fake> {
        async fn execute(&self, op: Op) -> Result<Response, RuntimeError> {
            self.as_ref().execute(op).await
        }
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn pure_model_room_revoke_kills_provider_seals_unknown_hold_and_keeps_other_room_running()
    {
        use hagency_agent_local::{Layer, Period};
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        for name in ["home", "codex", "workspace"] {
            std::fs::create_dir(root.join(name)).unwrap();
            std::fs::set_permissions(root.join(name), std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let script = root.join("provider");
        let marker = root.join("model-pid");
        install_long_turn_provider(&script, &marker);
        let path = root.join("ledger");
        let mut configured = setup(&path, &root.join("workspace"));
        configured
            .set_policy(
                OWNER,
                &event().scope(),
                Layer::Room,
                0,
                &hagency_agent_local::Policy {
                    budget: hagency_agent_local::Budget {
                        limit: hagency_agent_local::Limit::Tokens(100),
                        period: Period::Lifetime,
                    },
                    requests: hagency_agent_local::RequestPolicy::Allow,
                    high_risk: hagency_agent_local::ToolPolicy::Deny,
                },
            )
            .unwrap();
        drop(configured);
        let mut a = config(&root.join("workspace"));
        a.profile.executable = script;
        a.profile.home = root.join("home");
        a.profile.codex_home = root.join("codex");
        let mut b = a.clone();
        b.binding_id = "binding-b".into();
        let (initial, first, _) = binding_work(a);
        let (work, second, _) = binding_work(b);
        let (commands, rx) = mpsc::unbounded_channel();
        assert!(commands.send(work).is_ok());
        let (global_stop, mut receiver) = watch::channel(false);
        let fa = Arc::new(Fake {
            path: path.clone(),
            mode: Mode::RevokedModel,
            calls: StdMutex::new(vec![]),
            stop: first.cancel.clone(),
        });
        let fb = Arc::new(Fake {
            path: path.clone(),
            mode: Mode::IdleOtherRoom,
            calls: StdMutex::new(vec![]),
            stop: second.cancel.clone(),
        });
        let wire_a = fa.clone();
        let wire_b = fb.clone();
        let worker_path = path.clone();
        let supervisor = tokio::spawn(async move {
            supervise_bindings(initial, rx, &mut receiver, |mut work| {
                let wire = if work.config.binding_id == "binding-a" {
                    wire_a.clone()
                } else {
                    wire_b.clone()
                };
                let path = worker_path.clone();
                tokio::spawn(async move {
                    let mut ledger = Ledger::open(path, OWNER).unwrap();
                    let result = execution_loop(
                        &wire,
                        OWNER,
                        &mut ledger,
                        &work.config,
                        &lease(),
                        &mut work.receiver,
                        &work.status,
                    )
                    .await;
                    let mut status =
                        RuntimeStatus::new("agent-a", "stopped", failure_code(&result));
                    status.binding_id = Some(work.config.binding_id);
                    work.status.send_replace(status);
                })
            })
            .await
        });
        let mut ast = first.status.clone();
        tokio::time::timeout(Duration::from_secs(15), async {
            while ast.borrow().phase != "stopped" {
                ast.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        assert_eq!(ast.borrow().last_error, Some("authorization_or_lease_lost"));
        assert!(marker.exists());
        let pid = std::fs::read_to_string(&marker).unwrap();
        assert!(
            !std::process::Command::new("/bin/kill")
                .args(["-0", pid.trim()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap()
                .success(),
            "revoked model child must be terminated"
        );
        let ledger = Ledger::open(&path, OWNER).unwrap();
        assert_eq!(
            ledger.inbox_record(OWNER, "dispatch-a").unwrap().state,
            State::Unknown
        );
        assert_eq!(
            ledger
                .account(&event().scope(), Layer::Room, Period::Lifetime, now())
                .unwrap(),
            (0, 10)
        );
        let before = fb.calls.lock().unwrap().len();
        tokio::time::timeout(Duration::from_secs(3), async {
            while fb.calls.lock().unwrap().len() <= before {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert!(!supervisor.is_finished());
        assert!(!*second.cancel.borrow());
        assert!(!*global_stop.borrow());
        assert_eq!(
            fa.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|c| **c == "start")
                .count(),
            1
        );
        global_stop.send_replace(true);
        assert!(matches!(
            supervisor.await.unwrap(),
            Err(RuntimeError::Stopped)
        ));
        assert_eq!(second.status.borrow().phase, "stopped");
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn cancellation_kills_provider_and_preserves_unknown_reserved_charge() {
        use hagency_agent_local::{
            Budget, Layer, Limit, Period, Policy, RequestPolicy, ToolPolicy,
        };
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        for name in ["home", "codex", "workspace"] {
            std::fs::create_dir(root.join(name)).unwrap();
            std::fs::set_permissions(root.join(name), std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let script = root.join("codex-fixture");
        let marker = root.join("turn-started");
        install_long_turn_provider(&script, &marker);
        let path = root.join("ledger.db");
        let workspace = root.join("workspace");
        let mut ledger = setup(&path, &workspace);
        ledger
            .set_policy(
                OWNER,
                &event().scope(),
                Layer::Room,
                0,
                &Policy {
                    budget: Budget {
                        limit: Limit::Tokens(100),
                        period: Period::Lifetime,
                    },
                    requests: RequestPolicy::Allow,
                    high_risk: ToolPolicy::Deny,
                },
            )
            .unwrap();
        let mut config = config(&workspace);
        config.profile.executable = script;
        config.profile.home = root.join("home");
        config.profile.codex_home = root.join("codex");
        let (stop, mut receiver) = watch::channel(false);
        let fake = Fake {
            path,
            mode: Mode::CancelModel,
            calls: StdMutex::new(vec![]),
            stop: stop.clone(),
        };
        let (status, _) = watch::channel(RuntimeStatus::new("agent-a", "test", None));
        let revoke = async {
            tokio::time::timeout(Duration::from_secs(5), async {
                while !marker.exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            stop.send_replace(true);
        };
        let reference = lease();
        let (result, ()) = tokio::join!(
            execution_loop(
                &fake,
                OWNER,
                &mut ledger,
                &config,
                &reference,
                &mut receiver,
                &status
            ),
            revoke
        );
        assert!(matches!(result, Err(RuntimeError::Stopped)));
        assert_eq!(
            *fake.calls.lock().unwrap(),
            vec!["poll", "ack", "start", "authorize"]
        );
        assert_eq!(
            ledger.inbox_record(OWNER, "dispatch-a").unwrap().state,
            State::Unknown
        );
        assert_eq!(
            ledger
                .account(&event().scope(), Layer::Room, Period::Lifetime, now())
                .unwrap(),
            (0, 10)
        );
        assert_eq!(ledger.outstanding_calls(OWNER).unwrap().len(), 1);
    }
    #[tokio::test]
    async fn known_original_reply_reconciles_after_new_epoch_without_model_replay() {
        let (calls, state) = fixture(Mode::CrossEpochReply).await;
        assert_eq!(calls, vec!["reconcile", "reconcile", "poll"]);
        assert_eq!(state, State::Replied);
    }
    #[tokio::test]
    async fn server_accepted_unsent_reply_survives_restart_and_new_lease_reconciliation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("ledger.db");
        let mut ledger = setup(&path, tmp.path());
        ledger
            .receive_dispatch(OWNER, &event(), inbox::Limits::default(), now())
            .unwrap();
        ledger.acknowledge_dispatch(OWNER, "dispatch-a").unwrap();
        let prepared = ledger.prepare_execution(OWNER, "dispatch-a").unwrap();
        let mut envelope = prepared.dispatch.clone();
        envelope.state = "running".into();
        envelope.execution_id = Some(prepared.execution_id.clone());
        ledger
            .confirm_execution_start(
                OWNER,
                &inbox::ServerStart {
                    dispatch: envelope,
                    newly_started: true,
                },
            )
            .unwrap()
            .unwrap();
        ledger
            .persist_execution_reply(
                OWNER,
                "dispatch-a",
                &prepared.execution_id,
                "original response",
            )
            .unwrap();
        let (stop, mut receiver) = watch::channel(false);
        let accepted = Fake {
            path: path.clone(),
            mode: Mode::AcceptedOnly,
            calls: StdMutex::new(vec![]),
            stop: stop.clone(),
        };
        let record = ledger.inbox_record(OWNER, "dispatch-a").unwrap();
        send_reply(
            &accepted,
            OWNER,
            &mut ledger,
            &lease(),
            &mut receiver,
            &record,
        )
        .await
        .unwrap();
        assert_eq!(*accepted.calls.lock().unwrap(), vec!["reply"]);
        assert_eq!(
            ledger.inbox_record(OWNER, "dispatch-a").unwrap().state,
            State::ReplyReady
        );
        drop(ledger);
        let mut ledger = Ledger::open(&path, OWNER).unwrap();
        assert_eq!(
            ledger.recover_agent_executions(OWNER, "agent-a").unwrap(),
            0
        );
        // The server may have cancelled the old epoch's accepted outbox. The
        // only subsequent action is reconciliation of that exact known body.
        let newlease = LeaseRef {
            agent_id: "agent-a".into(),
            epoch: 2,
        };
        let restored = Fake {
            path,
            mode: Mode::CrossEpochReply,
            calls: StdMutex::new(vec![]),
            stop,
        };
        let record = ledger.inbox_record(OWNER, "dispatch-a").unwrap();
        send_reply(
            &restored,
            OWNER,
            &mut ledger,
            &newlease,
            &mut receiver,
            &record,
        )
        .await
        .unwrap();
        assert_eq!(
            *restored.calls.lock().unwrap(),
            vec!["reconcile", "reconcile"]
        );
        assert_eq!(
            ledger.inbox_record(OWNER, "dispatch-a").unwrap().state,
            State::Replied
        );
        assert_eq!(
            ledger
                .inbox_record(OWNER, "dispatch-a")
                .unwrap()
                .reply
                .as_deref(),
            None
        );
    }
    #[tokio::test]
    async fn approval_registry_fences_owner_device_expiry_digest_and_single_consumption() {
        let pin = DevicePin {
            origin: "https://server.test".into(),
            issuer: "https://server.test/_pasion/".into(),
            subject: "subject-a".into(),
            owner: OWNER.into(),
            user: "subject-a".into(),
            id: "device-a".into(),
            generation: 4,
        };
        let proposal = hagency_agent_local::ToolProposal {
            scope: event().scope(),
            dispatch: "dispatch-a".into(),
            tool: "model.request".into(),
            arguments: serde_json::json!({"executionId":"execution-a"}),
            canonical_directory: "/tmp/room".into(),
            risk: "model_request".into(),
            policy_revision: [1, 1, 1],
            expires: 100,
        };
        let (pending, decision) = codex::pending_approval(proposal.clone());
        let (cancel, _receiver) = watch::channel(false);
        let (_status, status) = watch::channel(RuntimeStatus::new("agent-a", "test", None));
        let mut entry = Entry {
            commands: mpsc::unbounded_channel().0,
            bindings: BTreeMap::from([(
                "binding-a".into(),
                BindingHandle {
                    cancel: watch::channel(false).0,
                    status: status.clone(),
                },
            )]),
            cancel,
            status,
            task: tokio::spawn(std::future::pending()),
            device: pin.clone(),
            approvals: BTreeMap::from([(
                "nonce".into(),
                ApprovalSlot {
                    view: ApprovalView {
                        proposal_id: "nonce".into(),
                        args_digest: "exact-proposal-digest".into(),
                        proposal,
                    },
                    pending,
                    device: pin.clone(),
                },
            )]),
        };
        let bindings = vec![Binding {
            id: "binding-a".into(),
            agent_id: "agent-a".into(),
            room_id: "!room-a:test".into(),
            state: "active".into(),
        }];
        for changed in 0..7 {
            let mut wrong = pin.clone();
            match changed {
                0 => wrong.owner = "@other:test".into(),
                1 => wrong.origin = "https://other.test".into(),
                2 => wrong.user = "subject-b".into(),
                3 => wrong.id = "device-b".into(),
                4 => wrong.generation += 1,
                5 => wrong.issuer = "https://other.test/_pasion/".into(),
                _ => wrong.subject = "different-oauth-subject".into(),
            }
            assert!(matches!(
                entry.take_approval(
                    &wrong,
                    "agent-a",
                    "nonce",
                    "exact-proposal-digest",
                    &bindings,
                    10
                ),
                Err(RuntimeError::Authorization)
            ));
        }
        assert!(matches!(
            entry.take_approval(&pin, "agent-a", "nonce", "changed-digest", &bindings, 10),
            Err(RuntimeError::Approval)
        ));
        assert!(matches!(
            entry.take_approval(
                &pin,
                "agent-a",
                "wrong-nonce",
                "exact-proposal-digest",
                &bindings,
                10
            ),
            Err(RuntimeError::Approval)
        ));
        assert!(matches!(
            entry.take_approval(
                &pin,
                "agent-a",
                "nonce",
                "exact-proposal-digest",
                &bindings,
                100
            ),
            Err(RuntimeError::Approval)
        ));
        assert!(matches!(
            entry.take_approval(&pin, "agent-a", "nonce", "exact-proposal-digest", &[], 10),
            Err(RuntimeError::Approval)
        ));
        entry.cancel.send_replace(true);
        assert!(matches!(
            entry.take_approval(
                &pin,
                "agent-a",
                "nonce",
                "exact-proposal-digest",
                &bindings,
                10
            ),
            Err(RuntimeError::Authorization)
        ));
        entry.cancel.send_replace(false);
        let entry = Arc::new(Mutex::new(entry));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let entry = entry.clone();
            let pin = pin.clone();
            tasks.push(tokio::spawn(async move {
                let bindings = vec![Binding {
                    id: "binding-a".into(),
                    agent_id: "agent-a".into(),
                    room_id: "!room-a:test".into(),
                    state: "active".into(),
                }];
                let mut entry = entry.lock().await;
                if let Ok(pending) = entry.take_approval(
                    &pin,
                    "agent-a",
                    "nonce",
                    "exact-proposal-digest",
                    &bindings,
                    10,
                ) {
                    pending.decide(true);
                    true
                } else {
                    false
                }
            }));
        }
        let mut count = 0;
        for task in tasks {
            count += usize::from(task.await.unwrap());
        }
        assert_eq!(count, 1);
        assert!(decision.await.unwrap());
        entry.lock().await.task.abort();
    }
    struct RequestFacts {
        expected: inbox::Dispatch,
        changed: bool,
    }
    impl Transport for RequestFacts {
        async fn execute(&self, op: Op) -> Result<Response, RuntimeError> {
            let Op::AuthorizeTool {
                dispatch_id,
                execution_id,
                ..
            } = op
            else {
                panic!("review must not start a model")
            };
            assert_eq!(dispatch_id, self.expected.id);
            let mut event = self.expected.clone();
            event.state = "running".into();
            event.execution_id = Some(execution_id);
            if self.changed {
                event.requester_mxid = "@different:test".into();
            }
            Ok(Response::ToolAuthorized(
                serde_json::from_value(serde_json::to_value(event).unwrap()).unwrap(),
            ))
        }
    }
    #[tokio::test]
    async fn model_request_review_binds_original_request_and_rechecks_remote_and_policy() {
        use hagency_agent_local::{
            Budget, Layer, Limit, Period, Policy, RequestPolicy, ToolPolicy,
        };
        for scenario in 0..3 {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("ledger");
            let mut ledger = setup(&path, temp.path());
            let mut policy = Policy {
                budget: Budget {
                    limit: Limit::Tokens(100),
                    period: Period::Lifetime,
                },
                requests: RequestPolicy::AskOwner,
                high_risk: ToolPolicy::Deny,
            };
            ledger
                .set_policy(OWNER, &event().scope(), Layer::Room, 0, &policy)
                .unwrap();
            let prepared = inbox::Prepared {
                dispatch: event(),
                execution_id: "execution-a".into(),
            };
            let facts = RequestFacts {
                expected: prepared.dispatch.clone(),
                changed: scenario == 1,
            };
            let (queue, mut receiver) = codex::approval_queue(1).unwrap();
            let edit_path = path.clone();
            let decision = tokio::spawn(async move {
                let pending = receiver.recv().await.unwrap();
                assert_eq!(pending.proposal.tool, "model.request");
                assert_eq!(pending.proposal.risk, "model_request");
                assert_eq!(pending.proposal.arguments["executionId"], "execution-a");
                if scenario == 2 {
                    let mut edit = Ledger::open(edit_path, OWNER).unwrap();
                    policy.requests = RequestPolicy::Allow;
                    edit.set_policy(OWNER, &event().scope(), Layer::Room, 1, &policy)
                        .unwrap();
                }
                pending.decide(true);
            });
            let (_stop, mut cancel) = watch::channel(false);
            let allowed = review_request(
                &facts,
                &lease(),
                &mut ledger,
                OWNER,
                &prepared,
                temp.path(),
                &queue,
                &mut cancel,
            )
            .await
            .unwrap();
            decision.await.unwrap();
            assert_eq!(allowed, scenario == 0);
            if allowed {
                ledger
                    .reserve(&event().scope(), "execution-a", "dispatch-a", 10, now())
                    .unwrap();
            } else {
                assert!(ledger.outstanding_calls(OWNER).unwrap().is_empty());
            }
        }
    }
    struct HistoryFake {
        pages: StdMutex<std::collections::VecDeque<Result<Response, RuntimeError>>>,
    }
    impl Transport for HistoryFake {
        async fn execute(&self, op: Op) -> Result<Response, RuntimeError> {
            match op {
                Op::History { .. } => {}
                Op::Acquire {
                    history_snapshot, ..
                } => {
                    assert!(history_snapshot.count <= 2);
                    assert_eq!(history_snapshot.digest.len(), 64);
                }
                _ => panic!("unexpected history operation"),
            }
            self.pages
                .lock()
                .unwrap()
                .pop_front()
                .expect("bounded page requests")
        }
    }
    fn history_page(
        digest: &str,
        entries: Vec<inbox::HistoryExecution>,
        count: u64,
        next: Option<&str>,
    ) -> Result<Response, RuntimeError> {
        Ok(Response::History(
            super::super::device_execution::ExecutionHistory {
                agent_id: "agent-a".into(),
                snapshot: HistorySnapshot {
                    count,
                    digest: if count == entries.len() as u64 {
                        use sha2::{Digest, Sha256};
                        let mut h = Sha256::new();
                        h.update(b"hagency-started-executions-v1\n");
                        for entry in &entries {
                            h.update(
                                serde_json::to_vec(&(&entry.dispatch_id, &entry.execution_id))
                                    .unwrap(),
                            );
                            h.update(b"\n");
                        }
                        format!("{:x}", h.finalize())
                    } else {
                        digest.repeat(64)
                    },
                },
                executions: entries,
                next_cursor: next.map(str::to_owned),
            },
        ))
    }
    fn history_identity(p: &inbox::Prepared) -> inbox::HistoryExecution {
        let mut value = serde_json::to_value(&p.dispatch).unwrap();
        let immutable = serde_json::to_vec(&serde_json::json!([
            p.dispatch.id,
            p.dispatch.binding_id,
            p.dispatch.agent_id,
            p.dispatch.event_id,
            p.dispatch.room_id,
            p.dispatch.requester_mxid,
            p.dispatch.thread_root,
            p.dispatch.body,
            p.dispatch.binding_generation
        ]))
        .unwrap();
        use sha2::{Digest, Sha256};
        let o = value.as_object_mut().unwrap();
        let id = o.remove("id").unwrap();
        o.insert("dispatchId".into(), id);
        o.insert("executionId".into(), p.execution_id.clone().into());
        o.insert(
            "immutableDigest".into(),
            format!("{:x}", Sha256::digest(immutable)).into(),
        );
        for key in ["body", "state", "outcome"] {
            o.remove(key);
        }
        serde_json::from_value(value).unwrap()
    }
    #[tokio::test]
    async fn history_fresh_zero_full_settled_and_missing_cost_gate_real_local_ledger() {
        let t = tempfile::tempdir().unwrap();
        let mut l = setup(&t.path().join("ledger.db"), t.path());
        let (_stop, mut cancel) = watch::channel(false);
        let fake = HistoryFake {
            pages: StdMutex::new([history_page("a", vec![], 0, None)].into()),
        };
        assert!(
            history_coverage(&fake, &mut l, OWNER, "agent-a", &mut cancel)
                .await
                .unwrap()
                .1
        );
        l.receive_dispatch(OWNER, &event(), inbox::Limits::default(), now())
            .unwrap();
        l.acknowledge_dispatch(OWNER, "dispatch-a").unwrap();
        let p = l.prepare_execution(OWNER, "dispatch-a").unwrap();
        let history = history_identity(&p);
        let fake = HistoryFake {
            pages: StdMutex::new([history_page("b", vec![history.clone()], 1, None)].into()),
        };
        assert!(
            !history_coverage(&fake, &mut l, OWNER, "agent-a", &mut cancel)
                .await
                .unwrap()
                .1
        );
        let policy = hagency_agent_local::Policy {
            budget: hagency_agent_local::Budget {
                limit: hagency_agent_local::Limit::Tokens(100),
                period: hagency_agent_local::Period::Lifetime,
            },
            requests: hagency_agent_local::RequestPolicy::Allow,
            high_risk: hagency_agent_local::ToolPolicy::Deny,
        };
        for layer in [
            hagency_agent_local::Layer::Agent,
            hagency_agent_local::Layer::Room,
            hagency_agent_local::Layer::Requester,
        ] {
            l.set_policy(OWNER, &event().scope(), layer, 0, &policy)
                .unwrap();
        }
        l.reserve(&event().scope(), &p.execution_id, "dispatch-a", 50, now())
            .unwrap();
        l.settle(
            &event().scope(),
            &p.execution_id,
            &hagency_agent_local::Usage {
                input: 10,
                output: 2,
                cached_input: 0,
                reasoning_output: 0,
                accounting_version: "fixture".into(),
            },
        )
        .unwrap();
        let fake = HistoryFake {
            pages: StdMutex::new([history_page("b", vec![history], 1, None)].into()),
        };
        assert!(
            history_coverage(&fake, &mut l, OWNER, "agent-a", &mut cancel)
                .await
                .unwrap()
                .1
        );
    }
    #[tokio::test]
    async fn history_changed_between_pages_after_acquire_and_missing_page_are_never_empty() {
        let t = tempfile::tempdir().unwrap();
        let mut l = setup(&t.path().join("ledger.db"), t.path());
        let (_stop, mut cancel) = watch::channel(false);
        let expected = HistorySnapshot {
            count: 0,
            digest: "a".repeat(64),
        };
        let fake = HistoryFake {
            pages: StdMutex::new([history_page("b", vec![], 0, None)].into()),
        };
        assert!(matches!(
            post_acquire_coverage(&fake, &mut l, OWNER, "agent-a", &expected, &mut cancel).await,
            Err(RuntimeError::LedgerRecovery)
        ));
        let fake = HistoryFake {
            pages: StdMutex::new([history_page("a", vec![], 1, None)].into()),
        };
        assert!(matches!(
            history_coverage(&fake, &mut l, OWNER, "agent-a", &mut cancel).await,
            Err(RuntimeError::LedgerRecovery)
        ));
        let fake = HistoryFake {
            pages: StdMutex::new([Err(RuntimeError::Transport)].into()),
        };
        assert!(
            history_coverage(&fake, &mut l, OWNER, "agent-a", &mut cancel)
                .await
                .is_err()
        );
        l.receive_dispatch(OWNER, &event(), inbox::Limits::default(), now())
            .unwrap();
        l.acknowledge_dispatch(OWNER, "dispatch-a").unwrap();
        let p = l.prepare_execution(OWNER, "dispatch-a").unwrap();
        let h = history_identity(&p);
        let fake = HistoryFake {
            pages: StdMutex::new(
                [
                    history_page("a", vec![h], 2, Some("dispatch-a")),
                    history_page("b", vec![], 2, None),
                ]
                .into(),
            ),
        };
        assert!(matches!(
            history_coverage(&fake, &mut l, OWNER, "agent-a", &mut cancel).await,
            Err(RuntimeError::LedgerRecovery)
        ));
    }
    #[tokio::test]
    async fn recovery_required_reconciles_known_reply_without_poll_ack_or_model() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("ledger.db");
        let mut l = setup(&path, t.path());
        l.receive_dispatch(OWNER, &event(), inbox::Limits::default(), now())
            .unwrap();
        l.acknowledge_dispatch(OWNER, "dispatch-a").unwrap();
        let p = l.prepare_execution(OWNER, "dispatch-a").unwrap();
        let mut d = p.dispatch.clone();
        d.state = "running".into();
        d.execution_id = Some(p.execution_id.clone());
        l.confirm_execution_start(
            OWNER,
            &inbox::ServerStart {
                dispatch: d,
                newly_started: true,
            },
        )
        .unwrap();
        l.persist_execution_reply(OWNER, "dispatch-a", &p.execution_id, "original response")
            .unwrap();
        let (stop, mut cancel) = watch::channel(false);
        let fake = Fake {
            path,
            mode: Mode::CrossEpochReply,
            calls: StdMutex::new(vec![]),
            stop: stop.clone(),
        };
        let (status, observed) = watch::channel(RuntimeStatus::new("agent-a", "starting", None));
        let cfg = config(t.path());
        let reference = LeaseRef {
            agent_id: "agent-a".into(),
            epoch: 2,
        };
        let future = execution_loop_with_gate(
            &fake,
            OWNER,
            &mut l,
            &cfg,
            &reference,
            &mut cancel,
            &status,
            false,
        );
        let result = tokio::select! {r=future=>r,_=tokio::time::sleep(Duration::from_millis(1400))=>{stop.send_replace(true);Err(RuntimeError::Stopped)}};
        assert!(matches!(result, Err(RuntimeError::Stopped)));
        assert_eq!(observed.borrow().phase, "ledger_recovery_required");
        let calls = fake.calls.lock().unwrap();
        assert!(!calls.contains(&"poll"));
        assert!(!calls.contains(&"ack"));
        assert_eq!(
            l.inbox_record(OWNER, "dispatch-a").unwrap().state,
            State::Replied
        );
    }
    #[tokio::test]
    async fn incomplete_ledger_never_acquires_without_explicit_known_reply_recovery() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("ledger.db");
        let mut l = setup(&path, t.path());
        l.receive_dispatch(OWNER, &event(), inbox::Limits::default(), now())
            .unwrap();
        l.acknowledge_dispatch(OWNER, "dispatch-a").unwrap();
        let p = l.prepare_execution(OWNER, "dispatch-a").unwrap();
        let mut h = history_identity(&p);
        h.dispatch_id = "dispatch-z".into();
        h.execution_id = "execution-z".into();
        h.binding_id = "binding-z".into();
        let history = vec![history_identity(&p), h];
        let (_stop, mut cancel) = watch::channel(false);
        let mut cfg = config(t.path());
        cfg.takeover = Takeover::OwnerRequested;
        let fake = HistoryFake {
            pages: StdMutex::new([history_page("b", history.clone(), 2, None)].into()),
        };
        assert!(matches!(
            acquire_checked(&fake, &mut l, OWNER, &cfg, &mut cancel).await,
            Err(RuntimeError::LedgerRecovery)
        ));
        assert!(
            fake.pages.lock().unwrap().is_empty(),
            "only history requested; no Acquire"
        );
        let mut d = p.dispatch.clone();
        d.state = "running".into();
        d.execution_id = Some(p.execution_id.clone());
        l.confirm_execution_start(
            OWNER,
            &inbox::ServerStart {
                dispatch: d,
                newly_started: true,
            },
        )
        .unwrap();
        l.persist_execution_reply(OWNER, "dispatch-a", &p.execution_id, "original response")
            .unwrap();
        cfg.takeover = Takeover::Never;
        let fake = HistoryFake {
            pages: StdMutex::new([history_page("b", history.clone(), 2, None)].into()),
        };
        assert!(matches!(
            acquire_checked(&fake, &mut l, OWNER, &cfg, &mut cancel).await,
            Err(RuntimeError::LedgerRecovery)
        ));
        cfg.takeover = Takeover::OwnerRequested;
        let response = Response::Lease(Lease {
            agent_id: "agent-a".into(),
            owner_user_id: "owner-a".into(),
            device_id: "device-a".into(),
            device_generation: 1,
            epoch: 2,
            expires_at_ms: wall_ms() + 30000,
        });
        let fake = HistoryFake {
            pages: StdMutex::new([history_page("b", history, 2, None), Ok(response)].into()),
        };
        let (_, snapshot, covered) = acquire_checked(&fake, &mut l, OWNER, &cfg, &mut cancel)
            .await
            .unwrap();
        assert!(!covered);
        assert_eq!(snapshot.count, 2);
        assert!(fake.pages.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn fresh_history_acquire_sends_snapshot_and_post_check_keeps_models_allowed() {
        let t = tempfile::tempdir().unwrap();
        let mut l = setup(&t.path().join("ledger.db"), t.path());
        let (_stop, mut cancel) = watch::channel(false);
        let lease = Response::Lease(Lease {
            agent_id: "agent-a".into(),
            owner_user_id: "owner-a".into(),
            device_id: "device-a".into(),
            device_generation: 1,
            epoch: 1,
            expires_at_ms: wall_ms() + 30000,
        });
        let fake = HistoryFake {
            pages: StdMutex::new(
                [
                    history_page("a", vec![], 0, None),
                    Ok(lease),
                    history_page("a", vec![], 0, None),
                ]
                .into(),
            ),
        };
        let (_, snapshot, pre) =
            acquire_checked(&fake, &mut l, OWNER, &config(t.path()), &mut cancel)
                .await
                .unwrap();
        assert!(pre);
        assert!(
            post_acquire_coverage(&fake, &mut l, OWNER, "agent-a", &snapshot, &mut cancel)
                .await
                .unwrap()
        );
        assert!(fake.pages.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn history_wrong_witness_digest_is_not_an_empty_or_complete_ledger() {
        let t = tempfile::tempdir().unwrap();
        let mut l = setup(&t.path().join("ledger.db"), t.path());
        let (_stop, mut cancel) = watch::channel(false);
        let mut page = history_page("a", vec![], 0, None).unwrap();
        if let Response::History(page) = &mut page {
            page.snapshot.digest = "0".repeat(64);
        }
        let fake = HistoryFake {
            pages: StdMutex::new([Ok(page)].into()),
        };
        assert!(matches!(
            history_coverage(&fake, &mut l, OWNER, "agent-a", &mut cancel).await,
            Err(RuntimeError::LedgerRecovery)
        ));
    }
}
