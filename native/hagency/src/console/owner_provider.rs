//! Dedicated owner provider onboarding. Codex owns OAuth tokens in its OS
//! keyring; this host only exposes a login URL and verified account status.
use super::{
    Console, body, console, cookie, current, owned_agents::ledger_path, recheck,
    server_login::OwnerOperation,
};
use salvo::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
    sync::Mutex,
};

#[derive(Debug, thiserror::Error)]
#[error("{code}")]
pub(crate) struct ProviderError {
    pub status: u16,
    pub code: &'static str,
}
fn failure(status: u16, code: &'static str) -> ProviderError {
    ProviderError { status, code }
}
fn private_directory(path: &Path) -> Result<PathBuf, ProviderError> {
    hagency_store::private::directory(path)
        .map_err(|_| failure(503, "provider_directory_unavailable"))?;
    let canonical = path
        .canonicalize()
        .map_err(|_| failure(503, "provider_directory_unavailable"))?;
    if canonical != path {
        return Err(failure(503, "provider_directory_not_canonical"));
    }
    Ok(canonical)
}
/// Locator of Codex's keyring entry (Codex Auth / cli|<path hash>). This
/// neither reads credentials nor treats the locator as proof of login.
pub(crate) fn credential_reference(home: &Path) -> Result<String, ProviderError> {
    private_directory(home)?;
    if std::fs::symlink_metadata(home.join("auth.json")).is_ok()
        || std::fs::symlink_metadata(home.join("config.toml")).is_ok()
    {
        return Err(failure(409, "provider_file_credentials_or_config_rejected"));
    }
    let digest = Sha256::digest(home.to_string_lossy().as_bytes());
    Ok(format!(
        "keychain:codex-home:{}",
        &format!("{digest:x}")[..16]
    ))
}
pub(crate) fn executable() -> Result<PathBuf, ProviderError> {
    let candidates = if let Some(configured) = std::env::var_os("HAGENCY_CODEX_BINARY") {
        vec![PathBuf::from(configured)]
    } else {
        let mut paths = vec![PathBuf::from(
            "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
        )];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path).map(|p| p.join("codex")));
        }
        paths
    };
    candidates
        .into_iter()
        .find_map(|p| {
            p.is_absolute()
                .then(|| p.canonicalize().ok())
                .flatten()
                .filter(|p| p.is_file())
        })
        .ok_or(failure(503, "codex_binary_unavailable"))
}
#[derive(Clone)]
pub(crate) struct ProviderPaths {
    pub home: PathBuf,
    pub codex_home: PathBuf,
    pub credential_ref: String,
}
pub(crate) fn paths(
    root: &Path,
    origin: &str,
    issuer: &str,
    subject: &str,
    owner: &str,
) -> Result<ProviderPaths, ProviderError> {
    let ledger = ledger_path(root, origin, issuer, subject, owner)
        .map_err(|_| failure(503, "provider_directory_unavailable"))?;
    let directory = ledger
        .parent()
        .ok_or(failure(503, "provider_directory_unavailable"))?
        .canonicalize()
        .map_err(|_| failure(503, "provider_directory_unavailable"))?;
    let home = private_directory(&directory.join("provider-home"))?;
    let codex_home = private_directory(&directory.join("codex-home"))?;
    let credential_ref = credential_reference(&codex_home)?;
    Ok(ProviderPaths {
        home,
        codex_home,
        credential_ref,
    })
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderStatus {
    pub state: &'static str,
    pub authenticated: bool,
    pub credential_ref: String,
    pub credential_store: &'static str,
    pub auth_url: Option<String>,
    pub strict_token_cap: bool,
    pub native_tools: bool,
}
struct Process {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next: u64,
    login_id: Option<String>,
    auth_url: Option<String>,
    until: Option<Instant>,
    reference: String,
}
impl Process {
    async fn spawn(paths: &ProviderPaths, executable: &Path) -> Result<Self, ProviderError> {
        credential_reference(&paths.codex_home)?;
        let mut command = tokio::process::Command::new(executable);
        command
            .args([
                "app-server",
                "--listen",
                "stdio://",
                "-c",
                "cli_auth_credentials_store=\"keyring\"",
            ])
            .args([
                "-c",
                "project_doc_max_bytes=0",
                "-c",
                "skills.include_instructions=false",
                "-c",
                "skills.bundled.enabled=false",
                "-c",
                "web_search=\"disabled\"",
                "-c",
                "developer_instructions=\"\"",
                "-c",
                "include_apps_instructions=false",
                "-c",
                "include_environment_context=false",
            ])
            .env_clear()
            .env("HOME", &paths.home)
            .env("CODEX_HOME", &paths.codex_home)
            .current_dir(&paths.home)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        if let Some(path) = std::env::var_os("PATH") {
            command.env("PATH", path);
        }
        for feature in [
            "shell_tool",
            "unified_exec",
            "code_mode_host",
            "code_mode",
            "hooks",
            "plugins",
            "multi_agent",
            "skill_search",
            "skill_mcp_dependency_install",
            "shell_snapshot",
        ] {
            command.arg("--disable").arg(feature);
        }
        let mut child = command
            .spawn()
            .map_err(|_| failure(503, "codex_provider_unavailable"))?;
        let input = child
            .stdin
            .take()
            .ok_or(failure(503, "codex_provider_unavailable"))?;
        let output = BufReader::new(
            child
                .stdout
                .take()
                .ok_or(failure(503, "codex_provider_unavailable"))?,
        );
        let mut p = Self {
            child,
            input,
            output,
            next: 1,
            login_id: None,
            auth_url: None,
            until: None,
            reference: paths.credential_ref.clone(),
        };
        p.rpc("initialize",json!({"clientInfo":{"name":"hagency-client","title":"Hagency Client","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":false}})).await?;
        p.write(json!({"method":"initialized"})).await?;
        Ok(p)
    }
    async fn write(&mut self, value: Value) -> Result<(), ProviderError> {
        let mut line =
            serde_json::to_vec(&value).map_err(|_| failure(503, "codex_protocol_mismatch"))?;
        line.push(b'\n');
        self.input
            .write_all(&line)
            .await
            .map_err(|_| failure(503, "codex_provider_unavailable"))?;
        self.input
            .flush()
            .await
            .map_err(|_| failure(503, "codex_provider_unavailable"))
    }
    async fn frame(&mut self) -> Result<Value, ProviderError> {
        let mut frame = Vec::new();
        loop {
            let available = self
                .output
                .fill_buf()
                .await
                .map_err(|_| failure(503, "codex_provider_unavailable"))?;
            if available.is_empty() {
                return Err(failure(503, "codex_provider_unavailable"));
            }
            let end = available.iter().position(|b| *b == b'\n').map(|n| n + 1);
            let n = end.unwrap_or(available.len());
            if frame.len() + n > 262144 {
                return Err(failure(502, "codex_protocol_mismatch"));
            }
            frame.extend_from_slice(&available[..n]);
            self.output.consume(n);
            if end.is_some() {
                return serde_json::from_slice(&frame)
                    .map_err(|_| failure(502, "codex_protocol_mismatch"));
            }
        }
    }
    async fn rpc(&mut self, method: &str, params: Value) -> Result<Value, ProviderError> {
        let id = self.next;
        self.next += 1;
        self.write(json!({"id":id,"method":method,"params":params}))
            .await?;
        tokio::time::timeout(Duration::from_secs(20),async{
            for _ in 0..128{
                let frame=self.frame().await?;
                if frame["id"]==id && frame.get("method").is_none(){
                    if frame.get("error").is_some(){return Err(failure(502,"codex_provider_request_rejected"));}
                    return frame.get("result").cloned().ok_or(failure(502,"codex_protocol_mismatch"));
                }
                if frame.get("id").is_some(){self.write(json!({"id":frame["id"],"error":{"code":-32601,"message":"Provider onboarding does not execute tools"}})).await?;}
                if frame["method"]=="account/login/completed" && frame["params"]["loginId"].as_str()==self.login_id.as_deref(){self.login_id=None;self.auth_url=None;self.until=None;}
            }Err(failure(502,"codex_protocol_mismatch"))
        }).await.map_err(|_|failure(504,"codex_provider_timeout"))?
    }
    async fn cancel(&mut self) -> Result<(), ProviderError> {
        if let Some(id) = self.login_id.take() {
            self.rpc("account/login/cancel", json!({"loginId":id}))
                .await?;
        }
        self.auth_url = None;
        self.until = None;
        Ok(())
    }
    async fn status(&mut self, paths: &ProviderPaths) -> Result<ProviderStatus, ProviderError> {
        credential_reference(&paths.codex_home)?;
        if self.until.is_some_and(|until| until <= Instant::now()) {
            self.cancel().await?;
        }
        let value = self
            .rpc("account/read", json!({"refreshToken":false}))
            .await?;
        let account = value["account"]["type"].as_str();
        let authenticated = value["requiresOpenaiAuth"] == true && account == Some("chatgpt");
        if value["account"].is_object() && !authenticated {
            return Err(failure(409, "provider_login_method_rejected"));
        }
        if authenticated {
            self.login_id = None;
            self.auth_url = None;
            self.until = None;
        }
        // A managed policy forcing file storage must not silently defeat keyring-only onboarding.
        credential_reference(&paths.codex_home)?;
        Ok(ProviderStatus {
            state: if authenticated {
                "authenticated"
            } else if self.login_id.is_some() {
                "signing_in"
            } else {
                "signed_out"
            },
            authenticated,
            credential_ref: self.reference.clone(),
            credential_store: "codex_os_keyring",
            auth_url: self.auth_url.clone(),
            strict_token_cap: false,
            native_tools: false,
        })
    }
    async fn start(&mut self, paths: &ProviderPaths) -> Result<ProviderStatus, ProviderError> {
        let status = self.status(paths).await?;
        if status.authenticated || self.login_id.is_some() {
            return Ok(status);
        }
        let value = self
            .rpc(
                "account/login/start",
                json!({"type":"chatgpt","useHostedLoginSuccessPage":true,"appBrand":"codex"}),
            )
            .await?;
        if value["type"] != "chatgpt" {
            return Err(failure(502, "codex_protocol_mismatch"));
        }
        let url = value["authUrl"]
            .as_str()
            .ok_or(failure(502, "codex_protocol_mismatch"))?;
        let parsed =
            reqwest::Url::parse(url).map_err(|_| failure(502, "codex_protocol_mismatch"))?;
        if parsed.scheme() != "https"
            || !matches!(parsed.host_str(), Some("auth.openai.com" | "chatgpt.com"))
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || url.len() > 8192
        {
            return Err(failure(502, "provider_authorization_url_rejected"));
        }
        self.login_id = Some(
            value["loginId"]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 128)
                .ok_or(failure(502, "codex_protocol_mismatch"))?
                .to_owned(),
        );
        self.auth_url = Some(url.to_owned());
        self.until = Some(Instant::now() + Duration::from_secs(600));
        self.status(paths).await
    }
}
struct Entry {
    process: Mutex<Process>,
    paths: ProviderPaths,
}
#[derive(Default, Clone)]
pub(super) struct OwnerProvider {
    entries: Arc<Mutex<BTreeMap<String, Arc<Entry>>>>,
}
#[derive(Clone, Copy)]
pub(super) enum Operation {
    Status,
    Login,
    Cancel,
    Logout,
}
impl OwnerProvider {
    pub async fn call(
        &self,
        console: &Console,
        cookie: &str,
        operation: Operation,
    ) -> Result<ProviderStatus, ProviderError> {
        let reply = console
            .owner_api(cookie, OwnerOperation::Agents)
            .await
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        let device = console
            .authorized_device()
            .await
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        if device.origin() != reply.origin
            || device.issuer() != reply.issuer
            || device.subject() != reply.subject
            || device.owner_mxid() != reply.owner
        {
            return Err(failure(401, "owner_authorization_required"));
        }
        device
            .bearer()
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        let root = console
            .0
            .server_login
            .state_directory()
            .ok_or(failure(503, "provider_directory_unavailable"))?;
        let paths = paths(
            &root,
            &reply.origin,
            &reply.issuer,
            &reply.subject,
            &reply.owner,
        )?;
        if matches!(operation, Operation::Logout) {
            console
                .0
                .owned_runtime
                .stop_profile(&device)
                .await
                .map_err(|_| failure(401, "owner_authorization_required"))?;
        }
        let key = paths.credential_ref.clone();
        let entry = {
            let mut entries = self.entries.lock().await;
            device
                .bearer()
                .map_err(|_| failure(401, "owner_authorization_required"))?;
            if let Some(entry) = entries.get(&key) {
                entry.clone()
            } else {
                let mut process = Process::spawn(&paths, &executable()?).await?;
                if device.bearer().is_err() {
                    let _ = process.child.kill().await;
                    return Err(failure(401, "owner_authorization_required"));
                }
                let entry = Arc::new(Entry {
                    process: Mutex::new(process),
                    paths,
                });
                entries.insert(key.clone(), entry.clone());
                entry
            }
        };
        let mut process = entry.process.lock().await;
        device
            .bearer()
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        let result = match operation {
            Operation::Status => process.status(&entry.paths).await,
            Operation::Login => process.start(&entry.paths).await,
            Operation::Cancel => {
                process.cancel().await?;
                process.status(&entry.paths).await
            }
            Operation::Logout => {
                process.cancel().await?;
                process.rpc("account/logout", json!({})).await?;
                process.status(&entry.paths).await
            }
        };
        // Recheck local/server capability before delivering account metadata or login URL.
        device
            .bearer()
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        let verified = console
            .owner_api(cookie, OwnerOperation::Agents)
            .await
            .map_err(|_| failure(401, "owner_authorization_required"))?;
        if verified.owner != reply.owner
            || verified.origin != reply.origin
            || verified.issuer != reply.issuer
            || verified.subject != reply.subject
        {
            return Err(failure(401, "owner_authorization_required"));
        }
        if result.is_err() {
            let _ = process.child.kill().await;
            drop(process);
            let mut entries = self.entries.lock().await;
            if entries
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(current, &entry))
            {
                entries.remove(&key);
            }
        }
        result
    }
    pub fn request_stop_all(&self) {
        if let Ok(entries) = self.entries.try_lock() {
            for entry in entries.values() {
                if let Ok(mut process) = entry.process.try_lock() {
                    let _ = process.child.start_kill();
                }
            }
        }
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let manager = self.clone();
            runtime.spawn(async move {
                manager.stop_all().await;
            });
        }
    }
    pub async fn stop_all(&self) {
        let entries = std::mem::take(&mut *self.entries.lock().await);
        for entry in entries.values() {
            let mut process = entry.process.lock().await;
            let _ = process.cancel().await;
            let _ = process.child.kill().await;
        }
    }
}
pub(super) fn router() -> Router {
    Router::with_path("owner-provider")
        .goal(dispatch)
        .push(Router::with_path("{action}").goal(dispatch))
}
#[handler]
async fn dispatch(req: &mut Request, depot: &Depot, res: &mut Response) {
    let result = async {
        let _guard = current(req, depot).map_err(|_| failure(401, "sign_in_required"))?;
        if req.uri().query().is_some() {
            return Err(failure(400, "invalid_arguments"));
        }
        let action = req.param::<String>("action").unwrap_or_default();
        let operation = match (req.method(), action.as_str()) {
            (&salvo::http::Method::GET, "") => Operation::Status,
            (&salvo::http::Method::POST, "login") => Operation::Login,
            (&salvo::http::Method::POST, "cancel") => Operation::Cancel,
            (&salvo::http::Method::POST, "logout") => Operation::Logout,
            _ => return Err(failure(404, "not_found")),
        };
        if !body(req, 1)
            .await
            .map_err(|_| failure(400, "invalid_arguments"))?
            .is_empty()
        {
            return Err(failure(400, "invalid_arguments"));
        }
        let console = console(depot).map_err(|_| failure(503, "local_state_unavailable"))?;
        let value = console
            .0
            .owner_provider
            .call(
                console,
                cookie(req).map_err(|_| failure(401, "sign_in_required"))?,
                operation,
            )
            .await?;
        recheck(depot).map_err(|_| failure(401, "sign_in_required"))?;
        Ok(value)
    }
    .await;
    match result {
        Ok(value) => res.render(Json(value)),
        Err(error) => {
            res.status_code(
                StatusCode::from_u16(error.status).unwrap_or(StatusCode::SERVICE_UNAVAILABLE),
            );
            res.render(Json(json!({"code":error.code})));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dedicated_provider_paths_are_private_canonical_and_owner_scoped() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap().join("state");
        let first = paths(
            &root,
            "https://server.example/",
            "https://server.example/_pasion/",
            "test-subject",
            "@alice:example",
        )
        .unwrap();
        let second = paths(
            &root,
            "https://server.example/",
            "https://server.example/_pasion/",
            "test-subject",
            "@bob:example",
        )
        .unwrap();
        let changed_subject = paths(
            &root,
            "https://server.example/",
            "https://server.example/_pasion/",
            "different-subject",
            "@alice:example",
        )
        .unwrap();
        assert_ne!(first.credential_ref, changed_subject.credential_ref);
        assert_ne!(first.codex_home, changed_subject.codex_home);
        assert_ne!(first.credential_ref, second.credential_ref);
        assert_ne!(first.codex_home, second.codex_home);
        let hash = format!(
            "{:x}",
            Sha256::digest(first.codex_home.to_string_lossy().as_bytes())
        );
        assert_eq!(
            first.credential_ref,
            format!("keychain:codex-home:{}", &hash[..16])
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, symlink};
            assert_eq!(
                std::fs::metadata(&first.codex_home).unwrap().mode() & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(&first.home).unwrap().mode() & 0o777,
                0o700
            );
            let link = first.home.join("linked-login");
            symlink(&first.codex_home, &link).unwrap();
            assert!(credential_reference(&link).is_err());
        }
        std::fs::write(first.codex_home.join("auth.json"), "{}").unwrap();
        assert_eq!(
            credential_reference(&first.codex_home).unwrap_err().code,
            "provider_file_credentials_or_config_rejected"
        );
        std::fs::remove_file(first.codex_home.join("auth.json")).unwrap();
        std::fs::write(first.codex_home.join("config.toml"), "").unwrap();
        assert!(credential_reference(&first.codex_home).is_err());
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn multi_profile_switch_stops_pending_provider_children_without_deleting_account_homes() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let exe = root.join("fake-codex");
        std::fs::write(&exe,r#"#!/usr/bin/env python3
import json,sys
for line in sys.stdin:
 r=json.loads(line);i=r.get('id');m=r.get('method')
 if i is None:continue
 if m=='account/read':v={'requiresOpenaiAuth':True,'account':None}
 elif m=='account/login/start':v={'type':'chatgpt','loginId':'pending','authUrl':'https://auth.openai.com/authorize?state=test'}
 elif m in ['initialize','account/login/cancel']:v={}
 else:raise Exception('no model or account credential mutation permitted')
 print(json.dumps({'id':i,'result':v}),flush=True)
"#).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        let manager = OwnerProvider::default();
        let mut held = vec![];
        for sub in ["subject-a", "subject-b"] {
            let paths = paths(
                &root.join("state"),
                "https://server.example/",
                "https://server.example/_pasion/",
                sub,
                "@alice:example",
            )
            .unwrap();
            std::fs::write(paths.home.join("retain-account-data"), sub).unwrap();
            let mut process = Process::spawn(&paths, &exe).await.unwrap();
            assert_eq!(process.start(&paths).await.unwrap().state, "signing_in");
            let entry = Arc::new(Entry {
                process: Mutex::new(process),
                paths,
            });
            manager
                .entries
                .lock()
                .await
                .insert(entry.paths.credential_ref.clone(), entry.clone());
            held.push(entry);
        }
        manager.stop_all().await;
        assert!(manager.entries.lock().await.is_empty());
        for entry in held {
            let mut p = entry.process.lock().await;
            assert!(p.child.try_wait().unwrap().is_some());
            assert!(p.login_id.is_none());
            assert!(entry.paths.home.join("retain-account-data").is_file());
            assert!(entry.paths.codex_home.is_dir());
        }
    }
    #[tokio::test]
    async fn onboarding_managed_keyring_login_status_and_logout_never_execute_turns() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().canonicalize().unwrap();
        let paths = paths(
            &directory.join("state"),
            "https://server.example/",
            "https://server.example/_pasion/",
            "test-subject",
            "@owner:example",
        )
        .unwrap();
        let executable = directory.join("fake-codex");
        std::fs::write(&executable,r#"#!/usr/bin/env python3
import json,os,sys,pathlib
home=pathlib.Path(os.environ['HOME']); marker=home/'signed-in'
(home/'process-metadata.json').write_text(json.dumps({'home':os.environ['HOME'],'codex_home':os.environ['CODEX_HOME'],'args':sys.argv[1:],'keys':list(os.environ.keys())}))
for line in sys.stdin:
 req=json.loads(line); method=req.get('method'); rid=req.get('id')
 if rid is None:continue
 if method=='initialize':result={}
 elif method=='account/read':result={'requiresOpenaiAuth':True,'account':({'type':'chatgpt','email':'private@example','planType':'plus'} if marker.exists() else None)}
 elif method=='account/login/start':
  assert req['params']['type']=='chatgpt'
  result={'type':'chatgpt','loginId':'login-1','authUrl':'https://auth.openai.com/authorize?state=test'}
 elif method=='account/login/cancel':result={}
 elif method=='account/logout':
  marker.unlink(missing_ok=True);result={}
 else:raise Exception('onboarding must not start inference')
 print(json.dumps({'id':rid,'result':result}),flush=True)
"#).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut process = Process::spawn(&paths, &executable).await.unwrap();
        assert!(!process.status(&paths).await.unwrap().authenticated);
        let pending = process.start(&paths).await.unwrap();
        assert_eq!(pending.state, "signing_in");
        assert!(pending.auth_url.is_some());
        process.cancel().await.unwrap();
        assert_eq!(process.status(&paths).await.unwrap().state, "signed_out");
        process.start(&paths).await.unwrap();
        std::fs::write(paths.home.join("signed-in"), "").unwrap();
        let status = process.status(&paths).await.unwrap();
        assert!(status.authenticated);
        assert!(status.auth_url.is_none());
        let public = serde_json::to_string(&status).unwrap();
        assert!(!public.contains("private@example"));
        assert!(!public.contains("token"));
        process.rpc("account/logout", json!({})).await.unwrap();
        assert!(!process.status(&paths).await.unwrap().authenticated);
        let metadata: Value = serde_json::from_slice(
            &std::fs::read(paths.home.join("process-metadata.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(metadata["home"], paths.home.to_string_lossy().as_ref());
        assert_eq!(
            metadata["codex_home"],
            paths.codex_home.to_string_lossy().as_ref()
        );
        assert!(
            metadata["args"]
                .as_array()
                .unwrap()
                .contains(&json!("cli_auth_credentials_store=\"keyring\""))
        );
        for flag in [
            "project_doc_max_bytes=0",
            "skills.include_instructions=false",
            "skills.bundled.enabled=false",
            "web_search=\"disabled\"",
            "developer_instructions=\"\"",
            "include_apps_instructions=false",
            "include_environment_context=false",
        ] {
            assert!(metadata["args"].as_array().unwrap().contains(&json!(flag)));
        }
        assert!(
            !metadata["keys"]
                .as_array()
                .unwrap()
                .iter()
                .any(|key| matches!(
                    key.as_str(),
                    Some("OPENAI_API_KEY" | "CODEX_ACCESS_TOKEN" | "HAGENCY_DEVICE_TOKEN")
                ))
        );
        assert!(!paths.codex_home.join("auth.json").exists());
        let _ = process.child.kill().await;
    }
    #[tokio::test]
    #[ignore = "requires installed Codex 0.160 and OS keyring; no login or inference"]
    async fn installed_codex_reads_only_its_dedicated_empty_owner_keyring() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap().join("state");
        let paths = paths(
            &root,
            "https://server.example/",
            "https://server.example/_pasion/",
            "test-subject",
            "@fresh-owner:example",
        )
        .unwrap();
        let mut process = Process::spawn(&paths, &executable().unwrap())
            .await
            .unwrap();
        let status = process.status(&paths).await.unwrap();
        assert!(!status.authenticated);
        assert_eq!(status.state, "signed_out");
        assert!(!paths.codex_home.join("auth.json").exists());
        assert!(!paths.codex_home.join("config.toml").exists());
        for ambient in ["skills", "AGENTS.md", "AGENTS.override.md"] {
            assert!(!paths.codex_home.join(ambient).exists());
        }
        let _ = process.child.kill().await;
        let mut reopened = Process::spawn(&paths, &executable().unwrap())
            .await
            .unwrap();
        assert!(!reopened.status(&paths).await.unwrap().authenticated);
        assert!(!paths.codex_home.join("skills").exists());
        let _ = reopened.child.kill().await;
    }
}
