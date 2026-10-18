use anyhow::{Result, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey, Signature};
use rand::RngCore;
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use super::SCHEMA;

pub fn now() -> u64 {
    crate::agent_chat::approval::current_unix_time_millis()
}
pub fn nonce() -> String {
    let mut bytes = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
pub fn canonical(value: &Value) -> Vec<u8> {
    fn sorted(v: &Value) -> Value {
        match v {
            Value::Object(map) => {
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                Value::Object(
                    keys.into_iter()
                        .map(|k| (k.clone(), sorted(&map[k])))
                        .collect(),
                )
            }
            Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
            _ => v.clone(),
        }
    }
    serde_json::to_vec(&sorted(value)).expect("JSON value serialization")
}
pub fn digest(value: &Value) -> String {
    format!("{:x}", Sha256::digest(canonical(value)))
}
pub fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    let s = value[name]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing field: {name}"))?;
    ensure!(
        !s.is_empty() && s.len() <= 8192 && !s.chars().any(char::is_control),
        "Invalid field: {name}"
    );
    Ok(s)
}
pub fn number(value: &Value, name: &str) -> Result<u64> {
    let n = value[name]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid number: {name}"))?;
    ensure!(n <= 9_007_199_254_740_991, "Number outside protocol range");
    Ok(n)
}
pub fn origin(address: &str) -> Result<String> {
    let url = url::Url::parse(address)?;
    ensure!(
        url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "Use an HTTP loopback address with an explicit port"
    );
    Ok(url.origin().ascii_serialization())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub agent: String,
    pub project_room: ruma::OwnedRoomId,
    pub owner_room: ruma::OwnedRoomId,
    pub bridge: ruma::OwnedUserId,
    /// Initial third member, fixed for the session after the encrypted bootstrap.
    #[serde(skip)]
    pub owner_agent: Option<ruma::OwnedUserId>,
    pub origin: String,
    pub fingerprint: String,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            origin(&self.origin)? == self.origin,
            "Use the canonical loopback origin"
        );
        ensure!(
            self.agent.len() <= 255
                && !self.agent.trim().is_empty()
                && !self.agent.chars().any(char::is_control),
            "Invalid agent name"
        );
        ensure!(
            self.project_room != self.owner_room,
            "Choose the separate owner approval room"
        );
        let hash = self
            .fingerprint
            .strip_prefix("sha256:")
            .ok_or_else(|| anyhow::anyhow!("Enter the pinned server fingerprint"))?;
        ensure!(
            URL_SAFE_NO_PAD
                .decode(hash)
                .map_err(|_| anyhow::anyhow!("Invalid server fingerprint"))?
                .len()
                == 32,
            "Invalid server fingerprint"
        );
        Ok(())
    }
}

pub struct ClientKey(SigningKey);
impl ClientKey {
    pub fn generate() -> Self {
        let mut bytes = [0; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(SigningKey::from_bytes(&bytes))
    }
    pub fn jwk(&self) -> Value {
        json!({"kty":"OKP", "crv":"Ed25519", "x":URL_SAFE_NO_PAD.encode(self.0.verifying_key().to_bytes())})
    }
    pub fn sign(&self, material: &Value) -> String {
        URL_SAFE_NO_PAD.encode(self.0.sign(&canonical(material)).to_bytes())
    }
}
pub fn fingerprint(jwk: &Value) -> Result<String> {
    public_key(jwk)?;
    Ok(format!(
        "sha256:{}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(canonical(jwk)))
    ))
}
fn public_key(jwk: &Value) -> Result<VerifyingKey> {
    ensure!(
        jwk.as_object().is_some_and(|o| o.len() == 3)
            && jwk["kty"] == "OKP"
            && jwk["crv"] == "Ed25519",
        "Invalid server key"
    );
    let bytes: [u8; 32] = URL_SAFE_NO_PAD
        .decode(string(jwk, "x")?)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid server key length"))?;
    Ok(VerifyingKey::from_bytes(&bytes)?)
}
pub fn verify(value: &Value, jwk: &Value, expected_fingerprint: &str) -> Result<()> {
    ensure!(
        fingerprint(jwk)? == expected_fingerprint
            && value["server_key_fingerprint"] == expected_fingerprint,
        "Server identity does not match the pinned key"
    );
    let signature =
        Signature::from_slice(&URL_SAFE_NO_PAD.decode(string(value, "server_signature")?)?)?;
    let mut unsigned = value.clone();
    let fields = unsigned
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid signed response"))?;
    fields.remove("server_signature");
    fields.remove("idempotent_request_replay");
    public_key(jwk)?.verify_strict(&canonical(&unsigned), &signature)?;
    Ok(())
}

pub struct Session {
    pub key: ClientKey,
    pub config: Config,
    pub owner: ruma::OwnedUserId,
    pub grant: Value,
    pub session: Value,
    pub snapshot: Option<Value>,
}
impl Session {
    pub fn from_exchange(
        key: ClientKey,
        config: Config,
        owner: ruma::OwnedUserId,
        grant: Value,
        session: Value,
    ) -> Result<Self> {
        verify(&session, &grant["server_public_jwk"], &config.fingerprint)?;
        ensure!(
            session["schema"] == SCHEMA && session["kind"] == "client_session",
            "Unsupported session"
        );
        for field in [
            "scope_id",
            "projection_id",
            "auth_fence_generation",
            "audience",
        ] {
            ensure!(session[field] == grant[field], "Session binding mismatch");
        }
        for field in ["client_session_id", "session_capability", "stream_epoch"] {
            string(&session, field)?;
        }
        let expires = number(&session, "expires_at_unix_ms")?;
        ensure!(
            expires > now() && expires <= now().saturating_add(900_000),
            "Invalid session expiry"
        );
        Ok(Self {
            key,
            config,
            owner,
            grant,
            session,
            snapshot: None,
        })
    }
    pub fn valid(&self) -> Result<()> {
        ensure!(
            number(&self.session, "expires_at_unix_ms")? > now(),
            "Agent session expired; reconnect"
        );
        Ok(())
    }
    pub fn validate_projection(&self, value: &Value) -> Result<()> {
        self.valid()?;
        ensure!(
            value["schema"] == SCHEMA,
            "Unsupported Agent Operations response"
        );
        for field in ["projection_id", "stream_epoch", "auth_fence_generation"] {
            ensure!(
                value[field] == self.session[field],
                "Agent session changed; reconnect"
            );
        }
        Ok(())
    }
    pub fn accept_snapshot(&mut self, value: Value) -> Result<()> {
        self.validate_projection(&value)?;
        let scope = &value["scope"];
        ensure!(
            scope["scope_id"] == self.session["scope_id"]
                && scope["owner_mxid"] == self.owner.as_str()
                && scope["owner_dm_room_id"] == self.config.owner_room.as_str()
                && scope["project_room_id"] == self.config.project_room.as_str(),
            "Agent scope mismatch"
        );
        string(scope, "agent_id")?;
        if let Some(previous) = &self.snapshot {
            ensure!(
                scope == &previous["scope"] && number(&value, "seq")? >= number(previous, "seq")?,
                "Stale Agent Operations snapshot"
            );
        }
        number(&value, "seq")?;
        for section in ["attention", "tasks", "queue", "worktrees"] {
            let rows = value[section]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Invalid snapshot section"))?;
            ensure!(rows.len() <= 2000, "Snapshot exceeds client limit");
            for row in rows {
                if let Some(actions) = row.get("available_actions") {
                    let actions = actions
                        .as_array()
                        .ok_or_else(|| anyhow::anyhow!("Invalid actions"))?;
                    ensure!(actions.len() <= 16, "Too many actions");
                    for action in actions {
                        validate_action(action)?;
                    }
                }
            }
        }
        self.snapshot = Some(value);
        Ok(())
    }
    pub fn validate_inspection(&self, value: &Value, source: &Value) -> Result<()> {
        self.validate_projection(value)?;
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Refresh the panel first"))?;
        ensure!(
            value["scope_id"] == self.session["scope_id"]
                && value["client_session_id"] == self.session["client_session_id"]
                && number(value, "snapshot_seq")? >= number(snapshot, "seq")?
                && value["dispatch_target"]["entity_kind"] == source["target"]["entity_kind"]
                && value["dispatch_target"]["entity_id"] == source["target"]["entity_id"]
                && number(&value["dispatch_target"], "entity_version")?
                    >= number(&source["target"], "entity_version")?,
            "Inspection scope mismatch"
        );
        string(value, "inspection_id")?;
        string(value, "inspection_token")?;
        ensure!(
            number(value, "expires_at_unix_ms")? > now(),
            "Inspection expired; inspect again"
        );
        let action = &value["resolution_action"];
        validate_action(action)?;
        ensure!(
            action["kind"] == "resolve_outcome"
                && action["target"] == value["dispatch_target"]
                && action["resource_precondition"] == source["resource_precondition"],
            "Inspection target mismatch"
        );
        Ok(())
    }
    pub fn proof(&self, method: &str, path: &str, body: &Value, proof_nonce: &str) -> Value {
        json!({"schema":SCHEMA,"kind":"session_request", "client_session_id":self.session["client_session_id"],
            "proof_nonce":proof_nonce,"http_method":method,"request_path":path,"body_sha256":digest(body),"audience":self.config.origin})
    }
    pub fn command(
        &self,
        action: &Value,
        inspection: Option<&Value>,
        resolution: Option<&str>,
        note: &str,
        recovery: &str,
        request_id: &str,
    ) -> Result<(String, Value)> {
        self.valid()?;
        validate_action(action)?;
        ensure!(
            number(action, "expires_at_unix_ms")? > now(),
            "Action expired; refresh the panel"
        );
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Refresh the panel first"))?;
        let kind = string(action, "kind")?;
        let mut body = json!({"schema":SCHEMA,"request_id":request_id,"client_session_id":self.session["client_session_id"],
            "scope_id":self.session["scope_id"],"projection_id":self.session["projection_id"],"stream_epoch":self.session["stream_epoch"],
            "auth_fence_generation":self.session["auth_fence_generation"],"snapshot_seq":snapshot["seq"],
            "action_capability":action["capability"],"target":action["target"]});
        if let Some(precondition) = action.get("resource_precondition") {
            body["resource_precondition"] = precondition.clone();
        }
        if kind == "resolve_outcome" {
            let inspection =
                inspection.ok_or_else(|| anyhow::anyhow!("Inspect the outcome first"))?;
            self.validate_projection(inspection)?;
            ensure!(
                inspection["scope_id"] == self.session["scope_id"]
                    && inspection["client_session_id"] == self.session["client_session_id"]
                    && inspection["resolution_action"] == *action
                    && number(inspection, "snapshot_seq")? >= number(snapshot, "seq")?
                    && number(inspection, "expires_at_unix_ms")? > now(),
                "Inspection expired; inspect again"
            );
            let resolution = resolution.ok_or_else(|| anyhow::anyhow!("Choose a resolution"))?;
            ensure!(
                matches!(resolution, "continue" | "accept_completed" | "keep_blocked")
                    && action["allowed_resolutions"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|v| v == resolution)),
                "Resolution is not available"
            );
            ensure!(
                !note.trim().is_empty() && note.len() <= 2000,
                "Enter an inspection note"
            );
            body["snapshot_seq"] = inspection["snapshot_seq"].clone();
            body["inspection_id"] = inspection["inspection_id"].clone();
            body["inspection_token"] = inspection["inspection_token"].clone();
            body["operator_note"] = note.into();
            body["resolution"] = json!({"kind":resolution});
            if resolution == "continue" {
                ensure!(
                    !recovery.trim().is_empty() && recovery.len() <= 8000,
                    "Enter recovery instructions"
                );
                body["resolution"]["recovery_instruction"] = recovery.into();
            } else {
                ensure!(
                    recovery.is_empty(),
                    "Recovery instructions apply only to Continue"
                );
            }
        } else {
            let offered = ["attention", "tasks", "queue", "worktrees"]
                .into_iter()
                .flat_map(|k| snapshot[k].as_array().into_iter().flatten())
                .flat_map(|row| row["available_actions"].as_array().into_iter().flatten())
                .any(|a| a == action);
            ensure!(offered, "Action is not in the current snapshot");
        }
        Ok((
            format!("/api/agent-ops/v1/commands/{}", kind.replace('_', "-")),
            body,
        ))
    }
}
pub fn validate_action(action: &Value) -> Result<()> {
    ensure!(
        matches!(
            string(action, "kind")?,
            "cancel_dispatch"
                | "mark_resource_inspected"
                | "begin_outcome_inspection"
                | "resolve_outcome"
        ),
        "Unsupported Agent Operations action"
    );
    string(action, "action_id")?;
    string(action, "capability")?;
    number(action, "expires_at_unix_ms")?;
    let target = &action["target"];
    string(target, "entity_id")?;
    ensure!(
        matches!(string(target, "entity_kind")?, "dispatch" | "resource")
            && number(target, "entity_version")? > 0,
        "Invalid action target"
    );
    if let Some(p) = action.get("resource_precondition") {
        string(p, "resource_id")?;
        number(p, "dirty_generation")?;
    }
    Ok(())
}

pub fn validate_grant(grant: &Value, config: &Config, client_nonce: &str) -> Result<()> {
    verify(grant, &grant["server_public_jwk"], &config.fingerprint)?;
    ensure!(
        grant["schema"] == SCHEMA
            && grant["kind"] == "client_session_grant"
            && grant["client_nonce"] == client_nonce
            && grant["audience"] == config.origin
            && grant["exchange_endpoint"]
                == format!("{}/api/agent-ops/v1/session/exchange", config.origin),
        "Agent grant binding mismatch"
    );
    for field in ["grant_jti", "scope_id", "projection_id", "server_challenge"] {
        string(grant, field)?;
    }
    number(grant, "auth_fence_generation")?;
    let expiry = number(grant, "expires_at_unix_ms")?;
    ensure!(
        expiry > now() && expiry <= now().saturating_add(600_000),
        "Agent grant expired"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loopback_and_pins_are_strict() {
        assert!(origin("http://127.0.0.1:8090").is_ok());
        assert!(origin("http://[::1]:8090").is_ok());
        for bad in [
            "http://localhost:8090",
            "http://127.0.0.1:8090/a",
            "https://127.0.0.1:8090",
            "http://example.org:8090",
            "http://u@127.0.0.1:8090",
            "http://127.0.0.1:8090#x",
        ] {
            assert!(origin(bad).is_err(), "{bad}");
        }
        let key = ClientKey::generate();
        let jwk = key.jwk();
        let pin = fingerprint(&jwk).unwrap();
        let mut response = json!({"server_key_fingerprint":pin,"body":"中文"});
        response["server_signature"] = key.sign(&response).into();
        assert!(verify(&response, &jwk, &pin).is_ok());
        response["body"] = "changed".into();
        assert!(verify(&response, &jwk, &pin).is_err());
    }
    #[test]
    fn canonical_hash_matches_backend_fixture() {
        assert_eq!(
            digest(&Value::Null),
            "74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b"
        );
        assert_eq!(
            canonical(&json!({"b":0,"a":{"z":1,"a":2}})),
            br#"{"a":{"a":2,"z":1},"b":0}"#
        );
    }
    fn fixture_session() -> Session {
        let mut grant: Value =
            serde_json::from_str(include_str!("contract/client-session-grant.json")).unwrap();
        let mut session: Value =
            serde_json::from_str(include_str!("contract/client-session.json")).unwrap();
        let signer = ClientKey::generate();
        let jwk = signer.jwk();
        let pin = fingerprint(&jwk).unwrap();
        grant["server_public_jwk"] = jwk;
        grant["server_key_fingerprint"] = pin.clone().into();
        grant["expires_at_unix_ms"] = (now() + 120_000).into();
        grant.as_object_mut().unwrap().remove("server_signature");
        let mut unsigned = grant.clone();
        unsigned
            .as_object_mut()
            .unwrap()
            .remove("idempotent_request_replay");
        grant["server_signature"] = signer.sign(&unsigned).into();
        session["server_key_fingerprint"] = pin.clone().into();
        session["expires_at_unix_ms"] = (now() + 300_000).into();
        session.as_object_mut().unwrap().remove("server_signature");
        let mut unsigned = session.clone();
        unsigned
            .as_object_mut()
            .unwrap()
            .remove("idempotent_request_replay");
        session["server_signature"] = signer.sign(&unsigned).into();
        let config = Config {
            agent: "worker".into(),
            project_room: "!project:example.org".try_into().unwrap(),
            owner_room: "!owner-dm:example.org".try_into().unwrap(),
            bridge: "@bridge:example.org".try_into().unwrap(),
            owner_agent: None,
            origin: grant["audience"].as_str().unwrap().into(),
            fingerprint: pin,
        };
        validate_grant(&grant, &config, grant["client_nonce"].as_str().unwrap()).unwrap();
        let mut result = Session::from_exchange(
            ClientKey::generate(),
            config,
            "@owner:example.org".try_into().unwrap(),
            grant,
            session,
        )
        .unwrap();
        let mut snapshot: Value =
            serde_json::from_str(include_str!("contract/snapshot.json")).unwrap();
        snapshot["attention"][0]["available_actions"][0]["expires_at_unix_ms"] =
            (now() + 120_000).into();
        result.accept_snapshot(snapshot).unwrap();
        result
    }
    #[test]
    fn snapshots_reject_scope_epoch_fence_and_sequence_substitution() {
        let mut s = fixture_session();
        for (field, value) in [
            ("projection_id", json!("foreign")),
            ("stream_epoch", json!("foreign")),
            ("auth_fence_generation", json!(0)),
            ("seq", json!(1)),
        ] {
            let mut snapshot = s.snapshot.clone().unwrap();
            snapshot[field] = value;
            assert!(s.accept_snapshot(snapshot).is_err(), "{field}");
        }
        for field in [
            "scope_id",
            "owner_mxid",
            "owner_dm_room_id",
            "project_room_id",
            "agent_id",
        ] {
            let mut snapshot = s.snapshot.clone().unwrap();
            snapshot["scope"][field] = "foreign".into();
            assert!(s.accept_snapshot(snapshot).is_err(), "{field}");
        }
        assert_eq!(s.snapshot.as_ref().unwrap()["seq"], 42);
    }
    #[test]
    fn only_current_capabilities_can_be_dispatched_and_preconditions_are_preserved() {
        let s = fixture_session();
        let action = &s.snapshot.as_ref().unwrap()["attention"][0]["available_actions"][0];
        let (path, body) = s.command(action, None, None, "", "", "once").unwrap();
        assert!(path.ends_with("begin-outcome-inspection"));
        assert_eq!(
            body["resource_precondition"],
            action["resource_precondition"]
        );
        assert_eq!(body["target"], action["target"]);
        assert_eq!(body["request_id"], "once");
        for field in ["capability", "target", "resource_precondition", "kind"] {
            let mut action = action.clone();
            action[field] = "substituted".into();
            assert!(s.command(&action, None, None, "", "", "once").is_err());
        }
        let mut expired = action.clone();
        expired["expires_at_unix_ms"] = 1.into();
        assert!(s.command(&expired, None, None, "", "", "once").is_err());
    }
    #[test]
    fn resolution_requires_bound_inspection_and_semantically_valid_input() {
        let s = fixture_session();
        let source = &s.snapshot.as_ref().unwrap()["attention"][0]["available_actions"][0];
        let mut inspection: Value =
            serde_json::from_str(include_str!("contract/outcome-inspection.json")).unwrap();
        inspection["expires_at_unix_ms"] = (now() + 120_000).into();
        inspection["resolution_action"]["expires_at_unix_ms"] = (now() + 120_000).into();
        s.validate_inspection(&inspection, source).unwrap();
        let action = &inspection["resolution_action"];
        for kind in ["continue", "accept_completed", "keep_blocked"] {
            let recovery = if kind == "continue" {
                "Checked diff; rerun tests"
            } else {
                ""
            };
            let (_, body) = s
                .command(
                    action,
                    Some(&inspection),
                    Some(kind),
                    "Inspected changes",
                    recovery,
                    "once",
                )
                .unwrap();
            assert_eq!(body["resolution"]["kind"], kind);
            assert_eq!(body["inspection_token"], inspection["inspection_token"]);
            if kind != "continue" {
                assert!(body["resolution"].get("recovery_instruction").is_none());
            }
        }
        assert!(s
            .command(action, None, Some("continue"), "checked", "retry", "once")
            .is_err());
        assert!(s
            .command(
                action,
                Some(&inspection),
                Some("continue"),
                "checked",
                "",
                "once"
            )
            .is_err());
        assert!(s
            .command(
                action,
                Some(&inspection),
                Some("accept_completed"),
                "checked",
                "retry",
                "once"
            )
            .is_err());
        assert!(s
            .command(
                action,
                Some(&inspection),
                Some("keep_blocked"),
                "",
                "",
                "once"
            )
            .is_err());
        for field in [
            "scope_id",
            "client_session_id",
            "snapshot_seq",
            "dispatch_target",
            "resolution_action",
        ] {
            let mut invalid = inspection.clone();
            invalid[field] = "foreign".into();
            assert!(s.validate_inspection(&invalid, source).is_err(), "{field}");
        }
    }
    #[test]
    fn grants_reject_wrong_nonce_pin_audience_and_expiry() {
        let s = fixture_session();
        let nonce = s.grant["client_nonce"].as_str().unwrap();
        assert!(validate_grant(&s.grant, &s.config, "wrong-nonce").is_err());
        let mut config = s.config.clone();
        config.origin = "http://127.0.0.1:9090".into();
        assert!(validate_grant(&s.grant, &config, nonce).is_err());
        let mut config = s.config.clone();
        config.fingerprint = fingerprint(&ClientKey::generate().jwk()).unwrap();
        assert!(validate_grant(&s.grant, &config, nonce).is_err());
        let mut invalid = s.grant.clone();
        invalid["expires_at_unix_ms"] = 1.into();
        assert!(validate_grant(&invalid, &s.config, nonce).is_err());
    }
    #[test]
    fn inspection_can_advance_entity_version_and_sequence_but_not_target_identity() {
        let s = fixture_session();
        let source = &s.snapshot.as_ref().unwrap()["attention"][0]["available_actions"][0];
        let mut inspection: Value =
            serde_json::from_str(include_str!("contract/outcome-inspection.json")).unwrap();
        inspection["expires_at_unix_ms"] = (now() + 120_000).into();
        inspection["resolution_action"]["expires_at_unix_ms"] = (now() + 120_000).into();
        inspection["snapshot_seq"] = 43.into();
        inspection["dispatch_target"]["entity_version"] = 8.into();
        inspection["resolution_action"]["target"] = inspection["dispatch_target"].clone();
        s.validate_inspection(&inspection, source).unwrap();
        let (_, body) = s
            .command(
                &inspection["resolution_action"],
                Some(&inspection),
                Some("continue"),
                "checked",
                "retry",
                "once",
            )
            .unwrap();
        assert_eq!(body["snapshot_seq"], 43);
        assert_eq!(body["target"]["entity_version"], 8);
        inspection["dispatch_target"]["entity_id"] = "foreign".into();
        assert!(s.validate_inspection(&inspection, source).is_err());
    }
}
