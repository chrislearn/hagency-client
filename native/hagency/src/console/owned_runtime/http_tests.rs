//! Isolated real TCP/typed-transport runtime test. Provider is an explicit fake
//! subprocess; no production provider credential or Matrix room is used.
use super::super::native::{MatrixAccessToken, MatrixTokenSource, NativeError, NativeOwner};
use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex as StdMutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
const OWNER: &str = "@alice:test";
struct Source;
impl MatrixTokenSource for Source {
    fn access_token(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<MatrixAccessToken, NativeError>> + Send + '_>,
    > {
        Box::pin(async {
            Ok(MatrixAccessToken {
                access_token: "fixture-sdk-only".into(),
                client_id: "sdk-client".into(),
            })
        })
    }
}
fn hash(value: &Value) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}
fn event(id: &str, epoch: i64) -> Value {
    json!({"id":id,"bindingId":"binding-a","agentId":"agent-a","eventId":format!("${id}:test"),"roomId":"!room-a:test","requesterMxid":"@bob:test","threadRoot":"$root:test","body":"isolated fixture request","state":"offered","bindingGeneration":1,"dispatchEpoch":epoch,"dispatchDeviceId":"device-a","executionId":null,"outcome":null})
}
#[derive(Default)]
struct WireState {
    epoch: i64,
    events: Vec<Value>,
    paths: Vec<String>,
    replies: usize,
    first_reply: Option<Value>,
}
impl WireState {
    fn history(&self) -> Value {
        let mut entries = vec![];
        let mut witness = Sha256::new();
        witness.update(b"hagency-started-executions-v1\n");
        for d in self.events.iter().filter(|d| d["executionId"].is_string()) {
            witness.update(serde_json::to_vec(&json!([d["id"], d["executionId"]])).unwrap());
            witness.update(b"\n");
            entries.push(json!({"dispatchId":d["id"],"executionId":d["executionId"],"bindingId":d["bindingId"],"agentId":d["agentId"],"eventId":d["eventId"],"roomId":d["roomId"],"requesterMxid":d["requesterMxid"],"threadRoot":d["threadRoot"],"bindingGeneration":1,"dispatchEpoch":d["dispatchEpoch"],"dispatchDeviceId":"device-a","immutableDigest":hash(&json!([d["id"],d["bindingId"],d["agentId"],d["eventId"],d["roomId"],d["requesterMxid"],d["threadRoot"],d["body"],1]))}));
        }
        json!({"history":{"agentId":"agent-a","snapshot":{"count":entries.len(),"digest":format!("{:x}",witness.finalize())},"executions":entries,"nextCursor":null}})
    }
    fn respond(&mut self, path: &str, input: &Value, origin: &str, unknown: bool) -> (u16, Value) {
        self.paths.push(path.into());
        let until = wall_ms() + 20_000;
        let lease = |epoch| json!({"lease":{"agentId":"agent-a","ownerUserId":"uid","deviceId":"device-a","deviceGeneration":1,"epoch":epoch,"expiresAtMs":until}});
        match path {
            "/api/hagency/v1/discovery" => (
                200,
                json!({"product":"hagency-server","version":"0.1.0","protocolVersion":3,"capabilities":["pasion-oauth","owner-agent-appservice-v1","global-agent-identity-v2","execution-device-v1","owner-direct-v1"],"homeserver":origin,"issuer":format!("{origin}_pasion/")}),
            ),
            "/api/hagency/v1/sessions/pasion" => (
                200,
                json!({"token":"a".repeat(64),"userId":"uid","mxid":OWNER,"validUntilMs":wall_ms()+30_000}),
            ),
            "/api/hagency/v1/identity" => (
                200,
                json!({"userId":"uid","mxid":OWNER,"subject":"sub","clientId":"sdk-client","issuer":format!("{origin}_pasion/"),"validUntilMs":wall_ms()+30_000}),
            ),
            "/api/hagency/v1/devices" => (
                200,
                json!({"token":"b".repeat(64),"deviceId":"device-a","generation":1,"validUntilMs":wall_ms()+30_000}),
            ),
            "/api/hagency/v1/sessions/current/renew" => {
                (200, json!({"validUntilMs":wall_ms()+30_000}))
            }
            "/api/hagency/v1/sessions/current" => (200, json!({})),
            "/api/hagency/v1/execution/leases/release" => (200, json!({"released":true})),
            "/api/hagency/v1/execution/history" => (200, self.history()),
            "/api/hagency/v1/execution/leases/acquire" => {
                assert_eq!(
                    input["historySnapshot"],
                    self.history()["history"]["snapshot"]
                );
                self.epoch += 1;
                if self.events.is_empty() {
                    self.events.push(event("dispatch-a", self.epoch));
                }
                (200, lease(self.epoch))
            }
            "/api/hagency/v1/execution/leases/renew" => (200, lease(self.epoch)),
            "/api/hagency/v1/execution/events/poll" => {
                assert_eq!(input["bindingId"], "binding-a");
                (
                    200,
                    json!({"events":self.events.iter().filter(|d|d["state"]=="offered").take(1).cloned().collect::<Vec<_>>()}),
                )
            }
            "/api/hagency/v1/execution/events/ack" => (200, json!({"acknowledged":true})),
            "/api/hagency/v1/execution/events/start" => {
                let d = self
                    .events
                    .iter_mut()
                    .find(|d| d["id"] == input["dispatchId"])
                    .unwrap();
                d["executionId"] = input["executionId"].clone();
                d["state"] = "running".into();
                (200, json!({"execution":{"dispatch":d,"newlyStarted":true}}))
            }
            "/api/hagency/v1/execution/events/authorize-tool" => (
                200,
                json!({"dispatch":self.events.iter().find(|d|d["id"]==input["dispatchId"]).unwrap()}),
            ),
            "/api/hagency/v1/execution/events/finish" => {
                let d = self
                    .events
                    .iter_mut()
                    .find(|d| d["id"] == input["dispatchId"])
                    .unwrap();
                d["state"] = "finished".into();
                d["outcome"] = input["outcome"].clone();
                if unknown && self.events.len() == 1 {
                    self.events.push(event("dispatch-b", self.epoch));
                }
                (200, json!({"finished":true}))
            }
            "/api/hagency/v1/execution/replies" => {
                self.replies += 1;
                if self.replies == 1 {
                    self.first_reply = Some(input.clone());
                    return (503, json!({"code":"fixture_reply_response_lost"}));
                }
                assert_eq!(
                    self.first_reply.as_ref(),
                    Some(input),
                    "retry preserves exact reply and execution identity"
                );
                let reply = &input["reply"];
                let d = self
                    .events
                    .iter_mut()
                    .find(|d| d["id"] == reply["dispatchId"])
                    .unwrap();
                d["state"] = "replied".into();
                let digest = format!(
                    "{:x}",
                    Sha256::digest(
                        format!(
                            "{{\"dispatchId\":{},\"executionId\":{},\"body\":{}}}",
                            reply["dispatchId"], reply["executionId"], reply["body"]
                        )
                        .as_bytes()
                    )
                );
                (
                    200,
                    json!({"reply":{"id":"reply-a","ownerEventId":d["id"],"agentId":"agent-a","bindingId":"binding-a","ownerUserId":"uid","roomId":"!room-a:test","puppetMxid":"@agent:test","bindingGeneration":1,"dispatchEpoch":d["dispatchEpoch"],"deliveryEpoch":self.epoch,"requesterMxid":"@bob:test","threadRoot":"$root:test","body":reply["body"],"payloadDigest":digest,"matrixTxnId":format!("hagency_{:x}",Sha256::digest(d["id"].as_str().unwrap().as_bytes())),"state":"sent","matrixEventId":"$fixture-sent:test"}}),
                )
            }
            _ => (404, json!({"code":"fixture_unknown_path"})),
        }
    }
}
async fn server(
    unknown: bool,
) -> (
    String,
    Arc<StdMutex<WireState>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let base = origin.clone();
    let state = Arc::new(StdMutex::new(WireState::default()));
    let records = state.clone();
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let state = records.clone();
            let origin = base.clone();
            tokio::spawn(async move {
                let mut bytes = vec![];
                let mut chunk = [0; 4096];
                let split = loop {
                    let n = stream.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                    if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&bytes[..split]).into_owned();
                let length = headers
                    .lines()
                    .find_map(|s| {
                        s.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|s| s.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < split + length {
                    let n = stream.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                }
                let path = headers
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap();
                let body: Value =
                    serde_json::from_slice(&bytes[split..split + length]).unwrap_or(Value::Null);
                if path.starts_with("/api/hagency/v1/execution/") {
                    assert!(
                        headers
                            .to_ascii_lowercase()
                            .contains(&format!("authorization: bearer {}", "b".repeat(64)))
                    );
                }
                let (code, value) = state.lock().unwrap().respond(path, &body, &origin, unknown);
                let body = value.to_string();
                let _=stream.write_all(format!("HTTP/1.1 {code} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await;
            });
        }
    });
    (origin, state, task)
}
fn provider(
    script: &std::path::Path,
    marker: &std::path::Path,
    ledger: &std::path::Path,
    unknown: bool,
) {
    let source=r##"#!/usr/bin/python3
import sys,json,time,sqlite3
marker=MARKER
ledger=LEDGER
unknown=UNKNOWN
for raw in sys.stdin:
 f=json.loads(raw);m=f.get('method');ident=f.get('id');p=f.get('params',{})
 def reply(v): print(json.dumps({'id':ident,'result':v}),flush=True)
 def emit(m,p): print(json.dumps({'method':m,'params':p}),flush=True)
 if m=='initialize': reply({})
 elif m=='initialized': pass
 elif m=='account/read': reply({'requiresOpenaiAuth':True,'account':{'type':'chatgpt'}})
 elif m=='config/read':
  c={'project_doc_max_bytes':0,'skills':{'include_instructions':False,'bundled':{'enabled':False}},'include_apps_instructions':False,'include_environment_context':False,'developer_instructions':'','web_search':'disabled','mcp_servers':{},'features':{}}
  for n in ['shell_tool','unified_exec','code_mode_host','code_mode','hooks','plugins','multi_agent','multi_agent_v2','skill_search','skill_mcp_dependency_install','shell_snapshot','view_image','image_generation','apps','tool_search','tool_suggest','web_search','web_search_cached','web_search_request','standalone_web_search','memory_tool']:c['features'][n]=False
  reply({'config':c})
 elif m in ['thread/start','thread/resume']:
  reply({'thread':{'id':'thread-a','cwd':p['cwd']},'cwd':p['cwd'],'model':p['model'],'approvalPolicy':'untrusted','approvalsReviewer':'user','sandbox':{'type':'readOnly','networkAccess':False}})
 elif m=='thread/name/set': reply({})
 elif m=='turn/start':
  db=sqlite3.connect('file:'+ledger+'?mode=ro',uri=True)
  assert db.execute("select count(*) from calls where state='pending' and reserved=10").fetchone()[0]==1
  assert db.execute("select count(*) from accounts where held=10").fetchone()[0]==3
  db.close()
  with open(marker,'a') as out:out.write('turn\n')
  reply({'turn':{'id':'turn-a','status':'inProgress'}})
  time.sleep(6)
  if not unknown:emit('thread/tokenUsage/updated',{'threadId':'thread-a','turnId':'turn-a','tokenUsage':{'total':{'inputTokens':5,'outputTokens':0,'cachedInputTokens':0,'reasoningOutputTokens':0,'totalTokens':5}}})
  emit('item/completed',{'threadId':'thread-a','turnId':'turn-a','item':{'id':'message-a','type':'agentMessage','text':'isolated fixture answer'}})
  emit('turn/completed',{'threadId':'thread-a','turn':{'id':'turn-a','status':'completed'}})
 else: raise Exception(m)
"##.replace("MARKER",&serde_json::to_string(marker).unwrap()).replace("LEDGER",&serde_json::to_string(ledger).unwrap()).replace("UNKNOWN",if unknown{"True"}else{"False"});
    std::fs::write(script, source).unwrap();
    std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o700)).unwrap();
}
async fn host_round(
    console: Console,
    config: StartConfig,
    path: PathBuf,
    state: Arc<StdMutex<WireState>>,
    unknown: bool,
    restart: bool,
) {
    let device = console.authorized_device().await.unwrap();
    let (initial, _binding_handle, _) = binding_work(config);
    let local_path = path.clone();
    let device_profile = profile_identity(
        device.origin(),
        device.issuer(),
        device.subject(),
        device.owner_mxid(),
    )
    .unwrap();
    let baseline_polls = state
        .lock()
        .unwrap()
        .paths
        .iter()
        .filter(|p| p.ends_with("/events/poll"))
        .count();
    let (commands, rx) = mpsc::unbounded_channel();
    let (stop, cancel) = watch::channel(false);
    let (status, _) = watch::channel(RuntimeStatus::new("agent-a", "test", None));
    let worker = tokio::spawn(run_host(
        console,
        device,
        path,
        initial,
        rx,
        cancel,
        stop.clone(),
        status,
    ));
    let ready = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let done = {
                let s = state.lock().unwrap();
                if restart {
                    s.paths
                        .iter()
                        .filter(|p| p.ends_with("/leases/acquire"))
                        .count()
                        >= 2
                        && s.paths
                            .iter()
                            .filter(|p| p.ends_with("/events/poll"))
                            .count()
                            >= baseline_polls + 2
                } else if unknown {
                    s.events.len() == 2 && s.events[1]["state"] == "finished"
                } else {
                    s.events.first().is_some_and(|d| d["state"] == "replied")
                        && Ledger::open_scoped(&local_path, OWNER, &device_profile)
                            .unwrap()
                            .inbox_record(OWNER, "dispatch-a")
                            .is_ok_and(|r| r.state == State::Replied)
                }
            };
            if done {
                break true;
            }
            if worker.is_finished() {
                break false;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    stop.send_replace(true);
    drop(commands);
    let result = tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(ready, Ok(true)),
        "runtime ended before completion: {result:?}; binding={:?}; paths={:?}",
        _binding_handle.status.borrow(),
        state.lock().unwrap().paths
    );
    assert!(
        result.is_ok() || matches!(result, Err(RuntimeError::Stopped)),
        "Runtime result: {result:?}; paths={:?}",
        state.lock().unwrap().paths
    );
}
async fn fixture(unknown: bool) {
    use hagency_agent_local::{
        Budget, Layer, Limit, ModelProfile, Period, Policy, RequestPolicy, ToolPolicy,
    };
    let (origin, state, server) = server(unknown).await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let native =
        NativeOwner::open_with_matrix(&root.join("state"), &origin, OWNER, Arc::new(Source))
            .await
            .unwrap();
    let console = native.test_console().clone();
    let device = console.authorized_device().await.unwrap();
    let path = ledger_path(
        &root.join("state"),
        device.origin(),
        device.issuer(),
        device.subject(),
        device.owner_mxid(),
    )
    .unwrap();
    let profile_id = profile_identity(
        device.origin(),
        device.issuer(),
        device.subject(),
        device.owner_mxid(),
    )
    .unwrap();
    let directory = path.parent().unwrap();
    let home = directory.join("provider-home");
    let codex_home = directory.join("codex-home");
    let workspace = root.join("workspace");
    for d in [&home, &codex_home, &workspace] {
        std::fs::create_dir(d).unwrap();
        std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let script = root.join("fake-provider");
    let marker = root.join("turns");
    provider(&script, &marker, &path, unknown);
    let scope = inbox::Dispatch {
        id: "dispatch-a".into(),
        binding_id: "binding-a".into(),
        agent_id: "agent-a".into(),
        event_id: "$dispatch-a:test".into(),
        room_id: "!room-a:test".into(),
        requester_mxid: "@bob:test".into(),
        thread_root: "$root:test".into(),
        body: "isolated fixture request".into(),
        state: "offered".into(),
        binding_generation: 1,
        dispatch_epoch: Some(1),
        dispatch_device_id: Some("device-a".into()),
        execution_id: None,
        outcome: None,
    }
    .scope();
    let mut ledger = Ledger::open_scoped(&path, OWNER, &profile_id).unwrap();
    ledger.register_binding(OWNER, &scope).unwrap();
    ledger
        .set_model_profile(
            OWNER,
            &scope,
            &ModelProfile {
                model: "test-model".into(),
                credential_ref: "keychain:fixture".into(),
                workspace_root: workspace.to_string_lossy().into(),
            },
        )
        .unwrap();
    ledger
        .set_policy(
            OWNER,
            &scope,
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
    drop(ledger);
    let config = || StartConfig {
        agent_id: "agent-a".into(),
        binding_id: "binding-a".into(),
        profile: Profile {
            executable: script.clone(),
            home: home.clone(),
            codex_home: codex_home.clone(),
            cwd: workspace.clone(),
            model: "test-model".into(),
            effort: "low".into(),
            shared_auth: false,
        },
        mode: BudgetMode::Estimated { reservation: 10 },
        credential_ref: "keychain:fixture".into(),
        takeover: Takeover::Never,
        host_files: false,
        owner_direct: false,
    };
    host_round(
        console.clone(),
        config(),
        path.clone(),
        state.clone(),
        unknown,
        false,
    )
    .await;
    let ledger = Ledger::open_scoped(&path, OWNER, &profile_id).unwrap();
    assert_eq!(std::fs::read_to_string(&marker).unwrap().lines().count(), 1);
    let account = ledger
        .account(&scope, Layer::Room, Period::Lifetime, now())
        .unwrap();
    if unknown {
        assert_eq!(account, (0, 10));
        assert_eq!(ledger.outstanding_calls(OWNER).unwrap().len(), 1);
        assert_eq!(
            ledger.inbox_record(OWNER, "dispatch-b").unwrap().state,
            State::Rejected
        );
    } else {
        assert_eq!(account, (5, 0));
        assert_eq!(
            ledger.inbox_record(OWNER, "dispatch-a").unwrap().state,
            State::Replied
        );
        drop(ledger);
        host_round(console, config(), path, state.clone(), false, true).await;
        assert_eq!(std::fs::read_to_string(&marker).unwrap().lines().count(), 1);
    }
    {
        let s = state.lock().unwrap();
        assert!(s.paths.iter().any(|p| p.ends_with("/events/ack")));
        assert!(s.paths.iter().any(|p| p.ends_with("/events/start")));
        assert!(s.paths.iter().any(|p| p.ends_with("/leases/renew")));
        assert!(
            s.paths
                .iter()
                .filter(|p| p.ends_with("/events/authorize-tool"))
                .count()
                >= 2
        );
        assert!(s.paths.iter().any(|p| p.ends_with("/leases/release")));
        if !unknown {
            assert_eq!(s.replies, 2);
        }
    }
    native.shutdown().await;
    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn typed_http_runtime_replies_retries_and_restart_never_repeat_provider_turn() {
    fixture(false).await;
}
#[tokio::test]
async fn typed_http_runtime_missing_usage_keeps_hold_and_blocks_next_provider_turn() {
    fixture(true).await;
}
