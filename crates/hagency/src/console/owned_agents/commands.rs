//! Private, profile-scoped command intents persisted BEFORE server mutation.
//! Recovery always queries the original command and preserves its key/payload.
use super::*;
use std::{io::Read, sync::Mutex};
static FILES: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Record {
    version: u32,
    pub id: String,
    pub operation: String,
    pub input: Value,
    pub agent_id: Option<String>,
    pub state: String,
    pub creation: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}
fn pending(record: &Record) -> bool {
    matches!(record.state.as_str(), "unknown" | "pending")
}
fn unavailable() -> OwnerError {
    error(503, "agent_command_state_unavailable")
}
fn directory(root: &Path, reply: &OwnerReply) -> Result<PathBuf, OwnerError> {
    let path = ledger_path(
        root,
        &reply.origin,
        &reply.issuer,
        &reply.subject,
        &reply.owner,
    )
    .map_err(|_| unavailable())?
    .parent()
    .ok_or_else(unavailable)?
    .join("agent-commands");
    hagency_store::private::directory(&path).map_err(|_| unavailable())?;
    Ok(path)
}
fn read(path: &Path) -> Result<Record, OwnerError> {
    let file = hagency_store::private::open(path, false).map_err(|_| unavailable())?;
    let mut raw = Vec::new();
    file.take(32769)
        .read_to_end(&mut raw)
        .map_err(|_| unavailable())?;
    if raw.len() > 32768 {
        return Err(unavailable());
    }
    let record: Record = serde_json::from_slice(&raw).map_err(|_| unavailable())?;
    if record.version != 1 || !matches!(record.operation.as_str(), "agent.create" | "agent.bind") {
        return Err(unavailable());
    }
    key(&record.id)?;
    key(record.input["idempotencyKey"]
        .as_str()
        .ok_or_else(unavailable)?)?;
    let expected = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}:{}",
            record.operation,
            record.input["idempotencyKey"].as_str().unwrap()
        ))
    );
    if expected != record.id
        || path.file_stem().and_then(|s| s.to_str()) != Some(record.id.as_str())
    {
        return Err(unavailable());
    }
    Ok(record)
}
fn records(dir: &Path) -> Result<Vec<Record>, OwnerError> {
    let mut result = vec![];
    for entry in std::fs::read_dir(dir).map_err(|_| unavailable())? {
        let entry = entry.map_err(|_| unavailable())?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
            if result.len() >= 10000 {
                return Err(unavailable());
            }
            result.push(read(&entry.path())?);
        }
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}
fn write(dir: &Path, record: &Record) -> Result<(), OwnerError> {
    hagency_store::private::replace(
        &dir.join(format!("{}.json", record.id)),
        &serde_json::to_vec(record).map_err(|_| unavailable())?,
    )
    .map_err(|_| unavailable())
}
fn prepare(
    dir: &Path,
    operation: &str,
    agent: Option<String>,
    input: Value,
) -> Result<Record, OwnerError> {
    let _guard = FILES.lock().map_err(|_| unavailable())?;
    let command = input["idempotencyKey"]
        .as_str()
        .ok_or_else(|| error(400, "invalid_arguments"))?;
    key(command)?;
    let id = format!("{:x}", Sha256::digest(format!("{operation}:{command}")));
    let mut payload = input.clone();
    payload
        .as_object_mut()
        .ok_or_else(|| error(400, "invalid_arguments"))?
        .remove("idempotencyKey");
    for previous in records(dir)? {
        if previous.id == id {
            if previous.input != input
                || previous.agent_id != agent
                || previous.operation != operation
            {
                return Err(error(409, "idempotency_key_reused"));
            }
            return Ok(previous);
        }
        let mut other = previous.input.clone();
        other
            .as_object_mut()
            .ok_or_else(unavailable)?
            .remove("idempotencyKey");
        if pending(&previous)
            && previous.operation == operation
            && previous.agent_id == agent
            && other == payload
        {
            return Err(error(409, "agent_command_recovery_required"));
        }
    }
    let record = Record {
        version: 1,
        id,
        operation: operation.into(),
        input,
        agent_id: agent,
        state: "unknown".into(),
        creation: None,
        error_code: None,
    };
    write(dir, &record)?;
    Ok(record)
}
fn operation(record: &Record) -> Result<OwnerOperation, OwnerError> {
    if record.operation == "agent.create" {
        let input: Create =
            serde_json::from_value(record.input.clone()).map_err(|_| unavailable())?;
        key(&input.idempotency_key)?;
        Ok(OwnerOperation::Create {
            name: input.display_name,
            visibility: record
                .input
                .get("visibility")
                .and_then(Value::as_str)
                .map(str::to_owned),
            command: input.idempotency_key,
        })
    } else {
        let input: Bind =
            serde_json::from_value(record.input.clone()).map_err(|_| unavailable())?;
        let agent = record.agent_id.clone().ok_or_else(unavailable)?;
        key(&agent)?;
        key(&input.project_id)?;
        key(&input.idempotency_key)?;
        matrix(&input.room_id, '!')?;
        Ok(OwnerOperation::Bind {
            agent,
            project: input.project_id,
            room: input.room_id,
            command: input.idempotency_key,
        })
    }
}
async fn execute(
    req: &Request,
    depot: &Depot,
    dir: &Path,
    mut record: Record,
) -> Result<Value, OwnerError> {
    let status = api(
        req,
        depot,
        OwnerOperation::AgentCommandStatus {
            operation: record.operation.clone(),
            command: record.input["idempotencyKey"]
                .as_str()
                .ok_or_else(unavailable)?
                .into(),
        },
    )
    .await;
    let reply = match status {
        Ok(reply) => reply,
        Err(e) if e.status == 404 => {
            let operation = operation(&record)?;
            recheck(depot).map_err(|_| error(401, "sign_in_required"))?;
            begin_attempt(dir, &mut record)?;
            match api(req, depot, operation).await {
                Ok(reply) => reply,
                Err(e) => {
                    recheck(depot).map_err(|_| error(401, "sign_in_required"))?;
                    record_failure(dir, &mut record, &e)?;
                    return Err(e);
                }
            }
        }
        Err(e) => return Err(e),
    };
    recheck(depot).map_err(|_| error(401, "sign_in_required"))?;
    let value = public(&reply.value, "creation")?;
    record.state = value["commandState"]
        .as_str()
        .ok_or_else(unavailable)?
        .into();
    record.creation = Some(value.clone());
    record.error_code = None;
    {
        let _guard = FILES.lock().map_err(|_| unavailable())?;
        write(dir, &record)?;
    }
    Ok(value)
}
// These are explicit pre-mutation rejections, not timeouts or transport failures.
// An unknown outcome must retain the original request/key for reconciliation.
fn begin_attempt(dir: &Path, record: &mut Record) -> Result<(), OwnerError> {
    record.state = "unknown".into();
    record.error_code = None;
    let _guard = FILES.lock().map_err(|_| unavailable())?;
    write(dir, record)
}
fn record_failure(dir: &Path, record: &mut Record, failure: &OwnerError) -> Result<(), OwnerError> {
    if matches!(failure.status, 400 | 403 | 404)
        && matches!(
            failure.code.as_str(),
            "invalid_arguments"
                | "invalid_agent_name"
                | "creation_not_allowed"
                | "forbidden"
                | "project_forbidden"
                | "project_not_found"
                | "room_not_found"
                | "agent_not_found"
                | "private_agent_owner_direct_only"
                | "room_kind_mismatch"
        )
    {
        record.state = "rejected".into();
    }
    record.error_code = (!failure.code.is_empty()
        && failure.code.len() <= 80
        && failure
            .code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'))
    .then(|| failure.code.clone());
    let _guard = FILES.lock().map_err(|_| unavailable())?;
    let current = read(&dir.join(format!("{}.json", record.id)))?;
    if pending(&current) || current.state == "rejected" {
        write(dir, record)?;
    }
    Ok(())
}
async fn location(req: &Request, depot: &Depot) -> Result<PathBuf, OwnerError> {
    let reply = api(req, depot, OwnerOperation::Agents).await?;
    recheck(depot).map_err(|_| error(401, "sign_in_required"))?;
    let root = console(depot)
        .map_err(|_| unavailable())?
        .0
        .server_login
        .state_directory()
        .ok_or_else(unavailable)?;
    directory(&root, &reply)
}
pub(super) async fn submit(
    req: &Request,
    depot: &Depot,
    operation: &str,
    agent: Option<String>,
    input: Value,
) -> Result<Value, OwnerError> {
    let dir = location(req, depot).await?;
    let record = prepare(&dir, operation, agent, input)?;
    execute(req, depot, &dir, record).await
}
// A list refresh only confirms the original command; it never resubmits it.
// Keep unresolved intents when status is missing or the server is unavailable.
fn confirm_completed(dir: &Path, record: &Record, value: Value) -> Result<(), OwnerError> {
    let state = value["commandState"].as_str().ok_or_else(unavailable)?;
    if matches!(state, "pending" | "unknown") {
        return Ok(());
    }
    let _guard = FILES.lock().map_err(|_| unavailable())?;
    let mut current = read(&dir.join(format!("{}.json", record.id)))?;
    // A concurrent resume may have already resolved this intent. Do not replace
    // its newer result with the snapshot from this read.
    if pending(&current) {
        current.state = state.into();
        current.creation = Some(value);
        current.error_code = None;
        write(dir, &current)?;
    }
    Ok(())
}
pub(super) async fn list(req: &Request, depot: &Depot) -> Result<Value, OwnerError> {
    let dir = location(req, depot).await?;
    let unresolved = {
        let _guard = FILES.lock().map_err(|_| unavailable())?;
        records(&dir)?
            .into_iter()
            .filter(pending)
            .collect::<Vec<_>>()
    };
    for record in unresolved {
        if let Ok(reply) = api(
            req,
            depot,
            OwnerOperation::AgentCommandStatus {
                operation: record.operation.clone(),
                command: record.input["idempotencyKey"]
                    .as_str()
                    .ok_or_else(unavailable)?
                    .into(),
            },
        )
        .await
        {
            recheck(depot).map_err(|_| error(401, "sign_in_required"))?;
            if let Ok(value) = public(&reply.value, "creation") {
                confirm_completed(&dir, &record, value)?;
            }
        }
    }
    recheck(depot).map_err(|_| error(401, "sign_in_required"))?;
    let _guard = FILES.lock().map_err(|_| unavailable())?;
    Ok(
        json!({"commands":records(&dir)?.into_iter().filter(|r|pending(r) || r.state == "rejected").collect::<Vec<_>>() }),
    )
}
pub(super) async fn check(req: &Request, depot: &Depot, id: &str) -> Result<Value, OwnerError> {
    key(id)?;
    let dir = location(req, depot).await?;
    let record = {
        let _guard = FILES.lock().map_err(|_| unavailable())?;
        read(&dir.join(format!("{id}.json")))?
    };
    let reply = api(
        req,
        depot,
        OwnerOperation::AgentCommandStatus {
            operation: record.operation.clone(),
            command: record.input["idempotencyKey"]
                .as_str()
                .ok_or_else(unavailable)?
                .into(),
        },
    )
    .await;
    recheck(depot).map_err(|_| error(401, "sign_in_required"))?;
    match reply {
        Ok(reply) => {
            let value = public(&reply.value, "creation")?;
            let _guard = FILES.lock().map_err(|_| unavailable())?;
            let mut current = read(&dir.join(format!("{id}.json")))?;
            if pending(&current) || current.state == "rejected" {
                current.state = value["commandState"]
                    .as_str()
                    .ok_or_else(unavailable)?
                    .into();
                current.creation = Some(value);
                current.error_code = None;
                write(&dir, &current)?;
            }
            Ok(json!({"command":current}))
        }
        // Absence is not permission to submit. A check is always read-only on
        // the server; only an explicit Retry may replay the original request.
        Err(e) if e.status == 404 => Ok(json!({"command":record})),
        Err(e) => Err(e),
    }
}
pub(super) async fn dismiss(req: &Request, depot: &Depot, id: &str) -> Result<Value, OwnerError> {
    key(id)?;
    let dir = location(req, depot).await?;
    recheck(depot).map_err(|_| error(401, "sign_in_required"))?;
    let _guard = FILES.lock().map_err(|_| unavailable())?;
    let mut record = read(&dir.join(format!("{id}.json")))?;
    if record.state != "rejected" {
        return Err(error(409, "agent_command_outcome_unknown"));
    }
    record.state = "dismissed".into();
    write(&dir, &record)?;
    Ok(json!({"dismissed":true}))
}
pub(super) async fn resume(req: &Request, depot: &Depot, id: &str) -> Result<Value, OwnerError> {
    key(id)?;
    let dir = location(req, depot).await?;
    let record = {
        let _guard = FILES.lock().map_err(|_| unavailable())?;
        read(&dir.join(format!("{id}.json")))?
    };
    execute(req, depot, &dir, record).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_requests_are_retryable_and_do_not_block_other_agents() {
        let temp = tempfile::tempdir().unwrap();
        let input =
            json!({"displayName":"First","visibility":"private","idempotencyKey":"first-key"});
        let mut first = prepare(temp.path(), "agent.create", None, input.clone()).unwrap();
        assert!(
            prepare(
                temp.path(),
                "agent.create",
                None,
                json!({"displayName":"Second","visibility":"private","idempotencyKey":"second-key"})
            )
            .is_ok()
        );
        record_failure(temp.path(), &mut first, &error(400, "invalid_arguments")).unwrap();
        let stored = read(&temp.path().join(format!("{}.json", first.id))).unwrap();
        assert_eq!(stored.state, "rejected");
        assert_eq!(stored.error_code.as_deref(), Some("invalid_arguments"));
        assert_eq!(stored.input, input);
        let mut retry = stored.clone();
        begin_attempt(temp.path(), &mut retry).unwrap();
        record_failure(temp.path(), &mut retry, &error(503, "server_unavailable")).unwrap();
        let uncertain = read(&temp.path().join(format!("{}.json", first.id))).unwrap();
        assert_eq!(
            uncertain.state, "unknown",
            "a lost retry must not remain safely dismissible"
        );
        assert_eq!(uncertain.error_code.as_deref(), Some("server_unavailable"));
        assert_eq!(uncertain.input, stored.input);
        write(temp.path(), &stored).unwrap();
        // Retrying the rejected operation preserves its key; a new operation
        // is also permitted after a confirmed rejection.
        assert_eq!(
            prepare(temp.path(), "agent.create", None, input)
                .unwrap()
                .id,
            first.id
        );
        assert!(
            prepare(
                temp.path(),
                "agent.create",
                None,
                json!({"displayName":"First","visibility":"private","idempotencyKey":"new-key"})
            )
            .is_ok()
        );
    }
    #[test]
    fn unknown_transport_outcomes_and_completed_unbound_agents_are_distinct() {
        let temp = tempfile::tempdir().unwrap();
        let input = json!({"displayName":"Agent","idempotencyKey":"original-key"});
        let mut record = prepare(temp.path(), "agent.create", None, input.clone()).unwrap();
        for failure in [
            error(503, "server_unavailable"),
            error(502, "invalid_server_response"),
            error(401, "sign_in_required"),
            error(409, "idempotency_key_reused"),
        ] {
            record_failure(temp.path(), &mut record, &failure).unwrap();
            assert_eq!(
                read(&temp.path().join(format!("{}.json", record.id)))
                    .unwrap()
                    .state,
                "unknown"
            );
        }
        record.state = "active".into();
        record.creation =
            Some(json!({"creation":{"agent":{"id":"created-agent"}},"commandState":"active"}));
        write(temp.path(), &record).unwrap();
        let next = json!({"displayName":"Agent","idempotencyKey":"next-key"});
        assert!(
            prepare(temp.path(), "agent.create", None, next).is_ok(),
            "an Agent without a Room never blocks another creation"
        );
        // Records written by previous clients continue to load.
        let mut old = serde_json::to_value(&record).unwrap();
        old.as_object_mut().unwrap().remove("errorCode");
        assert!(serde_json::from_value::<Record>(old).is_ok());
    }
    #[test]
    fn status_confirmation_preserves_pending_and_never_overwrites_resumed_result() {
        let temp = tempfile::tempdir().unwrap();
        let original = prepare(
            temp.path(),
            "agent.create",
            None,
            json!({"displayName":"Agent","idempotencyKey":"original"}),
        )
        .unwrap();
        confirm_completed(temp.path(), &original, json!({"commandState":"pending"})).unwrap();
        assert!(pending(
            &read(&temp.path().join(format!("{}.json", original.id))).unwrap()
        ));
        confirm_completed(
            temp.path(),
            &original,
            json!({"commandState":"active","creation":{"agent":{"id":"agent-a"}}}),
        )
        .unwrap();
        let path = temp.path().join(format!("{}.json", original.id));
        let mut resolved = read(&path).unwrap();
        assert!(!pending(&resolved));
        assert_eq!(resolved.input, original.input);
        resolved.state = "retired".into();
        write(temp.path(), &resolved).unwrap();
        confirm_completed(temp.path(), &original, json!({"commandState":"active"})).unwrap();
        assert_eq!(read(&path).unwrap().state, "retired");
    }
    #[test]
    fn restart_preserves_intent_and_rejects_new_key_or_changed_payload() {
        let temp = tempfile::tempdir().unwrap();
        let input = json!({"displayName":"Agent","idempotencyKey":"original"});
        let first = prepare(temp.path(), "agent.create", None, input.clone()).unwrap();
        assert_eq!(
            read(&temp.path().join(format!("{}.json", first.id)))
                .unwrap()
                .input,
            input
        );
        assert_eq!(
            prepare(temp.path(), "agent.create", None, input.clone())
                .unwrap()
                .id,
            first.id
        );
        let mut changed = input.clone();
        changed["displayName"] = "Changed".into();
        assert_eq!(
            prepare(temp.path(), "agent.create", None, changed)
                .unwrap_err()
                .code,
            "idempotency_key_reused"
        );
        let mut duplicate = input;
        duplicate["idempotencyKey"] = "new".into();
        assert_eq!(
            prepare(temp.path(), "agent.create", None, duplicate.clone())
                .unwrap_err()
                .code,
            "agent_command_recovery_required"
        );
        let mut completed = first;
        completed.state = "suspended".into();
        write(temp.path(), &completed).unwrap();
        assert!(prepare(temp.path(), "agent.create", None, duplicate.clone()).is_ok());
        completed.state = "retired".into();
        write(temp.path(), &completed).unwrap();
        let mut next = duplicate;
        next["idempotencyKey"] = "after_retire".into();
        // An explicitly new global Agent may reuse a display name after a known
        // retired result. Only unresolved commands require same-key recovery.
        for entry in std::fs::read_dir(temp.path()).unwrap() {
            let path = entry.unwrap().path();
            if path.file_stem().unwrap() != completed.id.as_str() {
                std::fs::remove_file(path).unwrap();
            }
        }
        assert!(prepare(temp.path(), "agent.create", None, next).is_ok());
    }
}
