//! Mini-app Octos services over Rinx's assistant (ADR 0007).
//!
//! Each running mini app gets its own request context of Rinx's peer
//! (`crate::octos_service`): a separate transcript, workspace and memory
//! namespace in the kernel. Never an ordinary privileged session, a kernel of
//! its own or a raw connection.
use octosense_app_peers::{ContextEvent, ContextOp, ContextSpec, OctosContext};
use rinx_miniapp_core::{InstanceId, Lease, OctosProvider, ServiceEvent};
use serde_json::{Value, json};
use std::sync::{Arc, mpsc::SyncSender};

/// One mini-app instance's Octos adapter.
pub struct ContextProvider {
    context: Arc<dyn OctosContext>,
}

impl ContextProvider {
    /// Open the instance's request context. `Err` names why the assistant is
    /// unavailable; the mini app's other services keep working.
    pub fn open(lease: &Lease) -> Result<Arc<Self>, String> {
        let service = crate::octos_service::service()?;
        let identity = lease.identity();
        let services = lease
            .services()
            .iter()
            .filter(|s| octosense_app_peers::OCTOS_SERVICES.contains(&s.as_str()))
            .cloned()
            .collect();
        let context = service.open_context(ContextSpec {
            account: identity.account.clone(),
            instance: format!("{}-g{}", identity.app, identity.generation),
            services,
        })?;
        Ok(Arc::new(Self { context }))
    }
}

fn valid(lease: &Lease) -> Result<(), String> {
    let account = crate::sliding_sync::current_user_id().ok_or("Not logged in")?;
    lease.check(account.as_str())
}

/// Turn a context event into the shape mini apps bind to:
/// `{"text", "event": {"kind", …}}`.
fn mini_app_value(data: &Value) -> Value {
    let method = data["method"].as_str().unwrap_or("");
    let params = &data["params"];
    let event = match method {
        "approval/requested" => json!({
            "kind": "approval_requested",
            "approval_id": params["approval_id"],
            "title": params["title"],
            "body": params["body"],
        }),
        other => json!({"kind": other.replace('/', "_")}),
    };
    json!({"text": data.get("text").cloned().unwrap_or(Value::Null), "event": event})
}

fn completed_reply(result: Result<Value, String>) -> ServiceEvent {
    // The broker already returns each service's public result. Wrapping all
    // results in `text` hides session_id/messages from history callbacks.
    ServiceEvent::Complete(result.and_then(rinx_miniapp_core::bounded_reply))
}

fn forward(lease: Lease, reply: SyncSender<ServiceEvent>) -> octosense_app_peers::EventSink {
    let reply = std::sync::Mutex::new(reply);
    Arc::new(move |event| {
        // The lease is checked again before anything reaches the app.
        if valid(&lease).is_err() {
            return;
        }
        let event = match event {
            ContextEvent::Data(data) => {
                // Streamed text and approval requests only.
                if data.get("text").is_none() && data["method"] != "approval/requested" {
                    return;
                }
                match rinx_miniapp_core::bounded_reply(mini_app_value(&data)) {
                    Ok(value) => ServiceEvent::Data(value),
                    Err(_) => return,
                }
            }
            ContextEvent::Complete(result) => completed_reply(result),
        };
        if let Ok(reply) = reply.lock() {
            let _ = reply.try_send(event);
        }
        makepad_widgets::SignalToUI::set_ui_signal();
    })
}

impl OctosProvider for ContextProvider {
    fn request(
        &self,
        lease: Lease,
        service: &str,
        args: Value,
        reply: SyncSender<ServiceEvent>,
    ) -> Result<(), String> {
        valid(&lease)?;
        lease.authorize(&lease.identity().account, service, None)?;
        // Apps supply input text, never session/profile/workspace identity,
        // approval decisions, arbitrary RPC methods or filesystem paths.
        let allowed: &[&str] = if service == "octos.turn.start" { &["text"] } else { &[] };
        if args
            .as_object()
            .is_none_or(|o| o.keys().any(|k| !allowed.contains(&k.as_str())))
        {
            return Err("Unsupported Octos arguments".into());
        }
        let op = match service {
            "octos.session.open" => ContextOp::Open,
            "octos.session.history" => ContextOp::History,
            "octos.turn.start" => ContextOp::Turn {
                text: args["text"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty() && s.len() <= 32 * 1024)
                    .ok_or("Provide text (at most 32 KiB)")?
                    .to_owned(),
            },
            "octos.turn.interrupt" => ContextOp::Interrupt,
            _ => return Err("Unknown Octos mini-app service".into()),
        };
        self.context.call(op, forward(lease, reply))
    }

    fn decide(
        &self,
        lease: Lease,
        approval: &str,
        approve: bool,
        reply: SyncSender<ServiceEvent>,
    ) -> Result<(), String> {
        valid(&lease)?;
        self.context.call(
            ContextOp::Approval { id: approval.to_owned(), approve },
            forward(lease, reply),
        )
    }

    fn close(&self, _identity: &InstanceId) {
        self.context.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_callbacks_keep_session_history_and_turn_fields() {
        for result in [
            json!({"open": true, "model": {"lane": "primary"}}),
            json!({"session_id": "context-a", "messages": [{"role": "assistant", "content": "hello"}]}),
            json!({"turn_id": "turn-a", "text": "hello"}),
        ] {
            let ServiceEvent::Complete(Ok(reply)) = completed_reply(Ok(result.clone())) else {
                panic!("expected a successful service callback");
            };
            assert_eq!(reply, result);
        }
        assert!(matches!(completed_reply(Err("revoked".into())), ServiceEvent::Complete(Err(e)) if e == "revoked"));
        let too_large = json!({"messages": ["x".repeat(rinx_miniapp_core::MAX_REPLY_BYTES)]});
        assert!(matches!(completed_reply(Ok(too_large)), ServiceEvent::Complete(Err(_))));
    }

    #[test]
    fn approval_requests_keep_the_shape_the_native_prompt_reads() {
        let value = mini_app_value(&json!({
            "method": "approval/requested",
            "params": {"approval_id": "a1", "title": "Run shell", "body": "ls"},
        }));
        assert_eq!(value["event"]["kind"], "approval_requested");
        assert_eq!(value["event"]["approval_id"], "a1");
        let streamed = mini_app_value(&json!({"method": "projection/envelope", "params": {}, "text": "Hel"}));
        assert_eq!(streamed["text"], "Hel");
        assert_eq!(streamed["event"]["kind"], "projection_envelope");
    }
}
