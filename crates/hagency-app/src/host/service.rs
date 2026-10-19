//! Where Rinx's assistant comes from (ADR 0007).
//!
//! | Deployment | Selected by | Runtime owner |
//! | --- | --- | --- |
//! | Hosted | the OctoSense shell creating the native module ([`install_hosted`]) | OctoSense |
//! | Standalone local | Rinx's AI settings ([`use_local`]) | Rinx |
//! | Standalone remote | an explicit server in Rinx's AI settings ([`use_remote`]) | the server |
//!
//! Every mode hands Rinx the same scoped [`OctosAppService`]: Rinx's peer
//! (owned by the OctoSense system agent when hosted, by Rinx's own root
//! session otherwise) with one request context per mini-app instance. Hosted
//! mode never starts a kernel, holds no AI credentials and cannot switch to
//! local or remote from Rinx's settings; a hosted Rinx without an injected
//! service simply has no assistant. An unconfigured assistant is a normal
//! state: Matrix and the editors keep working.

use std::sync::{Arc, Mutex};

use octosense_app_peers::{Availability, Deployment, OctosAppService, SettingsEntry};

/// The standalone assistant mode, persisted (non-secret) in Rinx's data dir.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum StandaloneMode {
    /// No assistant.
    #[default]
    Off,
    /// Rinx's own local runtime.
    Local,
    /// An explicit remote server. The access token is never written to disk:
    /// it is asked for again after a restart.
    Remote { url: String, profile: String },
}

#[derive(Default)]
struct State {
    /// `Some` once the deployment is fixed; hosted is fixed for the process.
    hosted: bool,
    service: Option<Arc<dyn OctosAppService>>,
    mode: Option<StandaloneMode>,
}

static STATE: Mutex<State> = Mutex::new(State { hosted: false, service: None, mode: None });

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// The OctoSense shell created Rinx as a native module and injected
/// `service` (or none: the host granted no assistant). Hosted is final.
pub fn install_hosted(service: Option<Arc<dyn OctosAppService>>) {
    let previous = {
        let mut st = state();
        st.hosted = true;
        st.mode = None;
        std::mem::replace(&mut st.service, service)
    };
    if let Some(previous) = previous {
        previous.release();
    }
    sync_account();
}

/// Whether the shell hosts Rinx's assistant.
pub fn is_hosted() -> bool {
    state().hosted
}

/// The current service, building the standalone one from its saved mode on
/// first use. `Err` says why there is no assistant.
pub fn service() -> Result<Arc<dyn OctosAppService>, String> {
    let (hosted, current, mode) = {
        let st = state();
        (st.hosted, st.service.clone(), st.mode.clone())
    };
    if let Some(current) = current {
        return Ok(current);
    }
    if hosted {
        return Err("OctoSense has not given Rinx assistant access. \
                    Check Settings → Accounts → AI providers."
            .into());
    }
    let mode = mode.unwrap_or_else(load_mode);
    match mode {
        StandaloneMode::Off => Err("The assistant is off. Choose this device or a server \
                                     in the assistant settings."
            .into()),
        StandaloneMode::Local => {
            #[cfg(feature = "octos-local")]
            {
                install_standalone(local::service()?, StandaloneMode::Local);
                service()
            }
            #[cfg(not(feature = "octos-local"))]
            Err("This build has no local assistant runtime".into())
        }
        StandaloneMode::Remote { .. } => {
            Err("Enter the server's access token again to reconnect".into())
        }
    }
}

/// What to show about the assistant.
pub fn status() -> String {
    let service = service();
    let service = match service {
        Ok(service) => service,
        Err(reason) => return reason,
    };
    let owner = match service.deployment() {
        Deployment::Hosted => "OctoSense",
        Deployment::StandaloneLocal => "this device",
        Deployment::StandaloneRemote => "your server",
    };
    let model = service
        .model()
        .map(|m| match (m.provider, m.model) {
            (Some(p), Some(model)) => format!(" · {p}/{model}"),
            _ => format!(" · {}", m.lane),
        })
        .unwrap_or_default();
    let state = match service.availability() {
        Availability::Ready => "ready".to_owned(),
        Availability::Idle => "starts on first use".to_owned(),
        Availability::Unavailable(why) => why,
        Availability::Failed(why) => format!("failed: {why}"),
    };
    let settings = match service.settings_entry() {
        SettingsEntry::Host => " · AI settings: OctoSense Settings → Accounts → AI providers",
        SettingsEntry::AppLocal | SettingsEntry::AppRemote => "",
    };
    format!("Assistant ({owner}){model}: {state}{settings}")
}

/// Bind the logged-in Matrix account (called on every account change): the
/// previous account's requests are revoked and never restored under another.
pub fn sync_account() {
    let service = state().service.clone();
    if let Some(service) = service {
        let account = crate::sliding_sync::current_user_id().map(|u| u.to_string());
        service.set_account(account.as_deref());
    }
}

/// Revoke requests immediately, including when login failed but the Matrix
/// client has not been cleared yet. Never rebind that stale client's account.
pub fn revoke_account() {
    if let Some(service) = state().service.clone() {
        service.set_account(None);
    }
}

/// Hosted Rinx is closing: release its leases, leave the shared kernel.
pub fn release() {
    let service = state().service.take();
    if let Some(service) = service {
        service.release();
    }
}

/// Standalone Rinx is exiting: release, and stop only a runtime Rinx owns.
pub fn shutdown() {
    let service = state().service.take();
    if let Some(service) = service {
        service.shutdown();
    }
}

/// Turn the standalone assistant off (stopping an owned runtime).
pub fn turn_off() -> Result<(), String> {
    if is_hosted() {
        return Err("OctoSense manages the assistant here".into());
    }
    shutdown();
    save_mode(&StandaloneMode::Off)?;
    state().mode = Some(StandaloneMode::Off);
    Ok(())
}

#[allow(dead_code)]
fn install_standalone(service: Arc<dyn OctosAppService>, mode: StandaloneMode) {
    let previous = {
        let mut st = state();
        st.mode = Some(mode);
        std::mem::replace(&mut st.service, Some(service))
    };
    // Replacing a provider revokes the old one's requests first.
    if let Some(previous) = previous {
        previous.shutdown();
    }
    sync_account();
}

/// Where Rinx keeps its peers' host tokens (mode 0600 files).
#[cfg_attr(not(any(feature = "octos-local", feature = "octos-remote")), allow(dead_code))]
fn peer_state_dir() -> std::path::PathBuf {
    crate::app_data_dir().join("octos").join("peers")
}

fn settings_path() -> std::path::PathBuf {
    crate::app_data_dir().join("octos").join("assistant.json")
}

fn load_mode() -> StandaloneMode {
    std::fs::read(settings_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_mode(mode: &StandaloneMode) -> Result<(), String> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(mode).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())
}

/// The services Rinx's peer uses: the four the mini-app host adapts.
#[cfg_attr(not(any(feature = "octos-local", feature = "octos-remote")), allow(dead_code))]
fn rinx_services() -> std::collections::BTreeSet<String> {
    octosense_app_peers::OCTOS_SERVICES.iter().map(|s| s.to_string()).collect()
}

/// Rinx's root session in standalone modes: the owner of Rinx's peer. No
/// OctoSense system agent is manufactured; Rinx's agent is the app root.
#[allow(dead_code)]
fn standalone_root(profile: &str) -> String {
    format!("{profile}:api:rinx#root")
}

/// Use Rinx's own local runtime.
#[cfg(feature = "octos-local")]
pub fn use_local() -> Result<(), String> {
    if is_hosted() {
        return Err("OctoSense manages the assistant here".into());
    }
    let service = local::service()?;
    save_mode(&StandaloneMode::Local)?;
    install_standalone(service, StandaloneMode::Local);
    Ok(())
}

/// Save a provider for the local runtime (written into Rinx's own kernel
/// profile; never another installation's). Restarts an owned runtime so it
/// reads the new profile.
#[cfg(feature = "octos-local")]
pub fn configure_local_provider(family: &str, model: &str, base_url: &str, key: &str) -> Result<(), String> {
    if is_hosted() {
        return Err("OctoSense manages the assistant here".into());
    }
    local::write_provider(family, model, base_url, key)?;
    // The next request starts a runtime that reads the new provider.
    shutdown();
    use_local()
}

/// A fresh standalone-local service (Rinx-owned runtime), not installed.
/// For embedders and the process-level tests; the UI uses [`use_local`].
#[cfg(feature = "octos-local")]
pub fn local_service() -> Result<Arc<dyn OctosAppService>, String> {
    local::service()
}

/// Rinx's own local kernel root.
#[cfg(feature = "octos-local")]
pub fn local_core_dir() -> std::path::PathBuf {
    local::core_dir()
}

/// Write the local runtime's provider without switching modes.
#[cfg(feature = "octos-local")]
pub fn write_local_provider(family: &str, model: &str, base_url: &str, key: &str) -> Result<(), String> {
    local::write_provider(family, model, base_url, key)
}

/// Use an explicit remote octos server. The token stays in memory.
#[cfg(feature = "octos-remote")]
pub fn use_remote(url: &str, profile: &str, token: &str) -> Result<(), String> {
    use octosense_app_peers::broker::{Broker, BrokerConfig};
    use octosense_app_peers::connectors::WsConnector;
    if is_hosted() {
        return Err("OctoSense manages the assistant here".into());
    }
    let url = url::Url::parse(url.trim()).map_err(|_| "Enter a complete server URL")?;
    let profile = profile.trim();
    if profile.is_empty() {
        return Err("Enter the server's profile name".into());
    }
    if token.is_empty() {
        return Err("Enter the server's access token".into());
    }
    let connector = WsConnector::new(url.clone(), token, profile)?;
    let mut cfg = BrokerConfig::new(
        Deployment::StandaloneRemote,
        profile,
        standalone_root(profile),
        "rinx",
        "Rinx",
        rinx_services(),
    );
    // The peer's host token (the kernel's credential for controlling Rinx's
    // peer) is kept per server, mode 0600, so a restart resumes the peer.
    cfg.state_dir = Some(peer_state_dir().join(url.host_str().unwrap_or("server")));
    let service = Arc::new(Broker::new(cfg, Arc::new(connector)));
    let mode = StandaloneMode::Remote { url: url.to_string(), profile: profile.to_owned() };
    save_mode(&mode)?;
    install_standalone(service, mode);
    Ok(())
}

/// The saved remote server, to prefill the form.
pub fn saved_remote() -> Option<(String, String)> {
    match load_mode() {
        StandaloneMode::Remote { url, profile } => Some((url, profile)),
        _ => None,
    }
}

#[cfg(feature = "octos-local")]
mod local {
    use super::*;
    use octosense_app_peers::broker::{Broker, BrokerConfig};
    use octosense_app_peers::connectors::CoreConnector;
    use std::path::PathBuf;

    /// Rinx's own kernel root, independent of any OctoSense installation or
    /// desktop octos profile.
    pub(super) fn core_dir() -> PathBuf {
        crate::app_data_dir().join("octos").join(".octos")
    }

    /// The packaged kernel: `RINX_OCTOS_BIN` when a developer sets it
    /// explicitly, else `octos` beside the Rinx executable (inside the app
    /// bundle on macOS). Never looked up on PATH. Android uses the APK's
    /// bundled `liboctos.so` (octosense-octos-core resolves it).
    fn packaged_kernel() -> Option<PathBuf> {
        if let Some(explicit) = std::env::var_os("RINX_OCTOS_BIN") {
            return Some(PathBuf::from(explicit));
        }
        if cfg!(target_os = "android") {
            return None;
        }
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?;
        let name = if cfg!(windows) { "octos.exe" } else { "octos" };
        [dir.join(name), dir.join("../Resources").join(name)]
            .into_iter()
            .find(|p| p.is_file())
    }

    pub(super) fn service() -> Result<Arc<dyn OctosAppService>, String> {
        let mut options = octosense_app_peers::octos_core::Options::default().core_dir(core_dir());
        match packaged_kernel() {
            Some(program) => options = options.program(program),
            None if cfg!(target_os = "android") => {}
            None => {
                return Err("This Rinx build has no packaged assistant runtime \
                            (octos beside the executable)"
                    .into())
            }
        }
        let core = octosense_app_peers::octos_core::Core::new(options);
        core.launch().map_err(|e| e.to_string())?;
        let mut cfg = BrokerConfig::new(
            Deployment::StandaloneLocal,
            "_main",
            standalone_root("_main"),
            "rinx",
            "Rinx",
            rinx_services(),
        );
        cfg.state_dir = Some(peer_state_dir().join("local"));
        Ok(Arc::new(Broker::new(cfg, Arc::new(CoreConnector::owned(core)))))
    }

    /// Write the local kernel's primary provider into Rinx's own profile.
    pub(super) fn write_provider(family: &str, model: &str, base_url: &str, key: &str) -> Result<(), String> {
        let family = family.trim();
        let model = model.trim();
        if family.is_empty() || model.is_empty() {
            return Err("Enter the provider and the model".into());
        }
        if !family.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
            return Err("Unknown provider name".into());
        }
        let dir = core_dir().join("profiles");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let key_env = format!("{}_API_KEY", family.to_ascii_uppercase().replace('-', "_"));
        let mut route = serde_json::json!({ "api_key_env": key_env });
        if !base_url.trim().is_empty() {
            let url = url::Url::parse(base_url.trim()).map_err(|_| "Enter a complete base URL")?;
            route["base_url"] = serde_json::json!(url.as_str());
        }
        let now = chrono::Utc::now().to_rfc3339();
        let mut env = serde_json::Map::new();
        if !key.is_empty() {
            env.insert(key_env.clone(), serde_json::json!(key));
        }
        let profile = serde_json::json!({
            "id": "_main", "name": "Rinx", "enabled": true,
            "created_at": now, "updated_at": now,
            "config": {
                "llm": {"primary": {"family_id": family, "model_id": model, "route": route}},
                "env_vars": env,
            }
        });
        let path = dir.join("_main.json");
        let tmp = dir.join("_main.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&profile).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercise real SDK session restoration and broker account gates without
    /// starting a kernel, contacting a homeserver or writing a user profile.
    #[tokio::test]
    async fn matrix_login_restore_switch_and_logout_bind_and_revoke_assistant_contexts() {
        use octosense_app_peers::{broker::{BoxFuture, Broker, BrokerConfig, Connector, Link}, ContextSpec};
        use crate::sliding_sync::replace_client;

        struct Offline;
        impl Connector for Offline {
            fn available(&self) -> Result<(), String> { Err("offline test".into()) }
            fn connect(&self) -> BoxFuture<'static, Result<Box<dyn Link>, String>> {
                panic!("account binding must not need a kernel connection");
            }
            fn owns_runtime(&self) -> bool { false }
            fn shutdown(&self) {}
        }
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                replace_client(None);
                *state() = State::default();
            }
        }
        let _reset = Reset;
        async fn client(user: &str) -> matrix_sdk::Client {
            let client = matrix_sdk::Client::builder()
                .homeserver_url("http://127.0.0.1:1")
                .server_versions([matrix_sdk::ruma::api::MatrixVersion::V1_0])
                .build().await.unwrap();
            client.matrix_auth().restore_session(
                serde_json::from_value(serde_json::json!({
                    "user_id": user, "device_id": "ACCOUNT_BINDING_TEST",
                    "access_token": "offline-fixture-token"
                })).unwrap(),
                Default::default(),
            ).await.unwrap();
            client
        }
        let alice = client("@alice:example.org").await;
        let bob = client("@bob:example.org").await;
        for deployment in [Deployment::Hosted, Deployment::StandaloneLocal, Deployment::StandaloneRemote] {
            replace_client(None);
            *state() = State::default();
            let service: Arc<dyn OctosAppService> = Arc::new(Broker::new(
                BrokerConfig::new(deployment, "_main", "_main:api:root", "rinx", "Rinx", rinx_services()),
                Arc::new(Offline),
            ));
            match deployment {
                Deployment::Hosted => install_hosted(Some(service.clone())),
                Deployment::StandaloneLocal => install_standalone(service.clone(), StandaloneMode::Local),
                Deployment::StandaloneRemote => install_standalone(service.clone(), StandaloneMode::Remote {
                    url: "ws://127.0.0.1:1".into(), profile: "_main".into(),
                }),
            }
            let open = |account: &str| service.open_context(ContextSpec {
                account: account.into(), instance: "test-mini-app".into(), services: rinx_services(),
            });
            assert!(open("@alice:example.org").is_err());
            replace_client(Some(alice.clone()));
            let first = open("@alice:example.org").expect("login must bind the new account");
            replace_client(Some(alice.clone()));
            assert!(!first.is_open(), "even the same account's old login must be revoked");
            let restored = open("@alice:example.org").expect("restored login must be bound");
            replace_client(Some(bob.clone()));
            assert!(!restored.is_open());
            assert!(open("@alice:example.org").is_err());
            let current = open("@bob:example.org").unwrap();
            crate::octoscript_apps::invalidate_sessions();
            assert!(!current.is_open(), "login failure must revoke before CLIENT is cleared");
            assert!(open("@bob:example.org").is_err());
            replace_client(Some(bob.clone()));
            let relogged = open("@bob:example.org").unwrap();
            replace_client(None);
            assert!(!relogged.is_open());
            assert!(open("@bob:example.org").is_err());
        }
    }

    #[test]
    fn standalone_mode_round_trips_without_secrets() {
        let mode = StandaloneMode::Remote { url: "https://octos.example".into(), profile: "me".into() };
        let text = serde_json::to_string(&mode).unwrap();
        assert!(!text.contains("token"));
        assert_eq!(serde_json::from_str::<StandaloneMode>(&text).unwrap(), mode);
        assert_eq!(serde_json::from_str::<StandaloneMode>(r#"{"mode":"off"}"#).unwrap(), StandaloneMode::Off);
    }

    #[test]
    fn the_standalone_root_is_rinx_own_session() {
        assert_eq!(standalone_root("_main"), "_main:api:rinx#root");
    }
}
