//! Operator task graphs (board #47) — TS parity with the retained
//! `lib/task-graph.js` document store and its routes
//! (`backend-v2.js:15974-16020`).
//!
//! The retained backend persists graphs as ONE JSON document,
//! `task_graphs.json`, in the backend's data directory
//! (`backend-v2.js:1732` `loadJsonSync`, `:3466` `saveJson`). Native has no
//! document/kv store API and this task forbids a migration, so the document
//! lives on the operator state directory (the same root `bootstrap` opens)
//! and is loaded at startup, exactly like the retained boot. Writes go
//! through the store's private-file policy (0600, atomic replace) via
//! `hagency_store::private`.
//!
//! A create is not just a row insert: the retained store dispatches every
//! root node (one durable name-addressed `task_graph_dispatch` message per
//! root, `backend-v2.js:3899-3959` -> `:4716`), which the oracle asserts
//! (`tests/api-task-graphs.test.js:58-64`). Native has no other producer of
//! that message, so this module persists it itself — the same durable effect
//! `dispatchInternalDirectMessage` produced (`messages.json` append +
//! `.msg_counter` id) — rather than mint a runner dispatch, which would be a
//! different thing and collide with "runner routes unchanged".
use super::{Error, body, console, failed, recheck};
use crate::refusal;
use salvo::prelude::*;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const GRAPHS_FILE: &str = "task_graphs.json";
const MESSAGES_FILE: &str = "messages.json";
const COUNTER_FILE: &str = ".msg_counter";
const MAX_RESULT_BYTES: usize = 65_536;
const MAX_GRAPH_BYTES: usize = 4 * 1024 * 1024;

pub(super) fn router() -> Router {
    Router::with_path("task-graphs")
        .post(create)
        .get(list)
        .push(
            Router::with_path("{id}")
                .get(read)
                .delete(remove),
        )
        .push(Router::with_path("{id}/nodes/{node_id}").patch(update_node))
}

/// One persisted task graph — the TS `normalizeGraph` shape
/// (`lib/task-graph.js:196-216`): camelCase timestamps, `nodes` keyed by id.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Graph {
    id: String,
    owner: String,
    label: String,
    status: String,
    nodes: BTreeMap<String, Node>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "completedAt")]
    completed_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Node {
    id: String,
    assignee: String,
    description: String,
    #[serde(default)]
    depends_on: Vec<String>,
    status: String,
    #[serde(default)]
    result: Value,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    condition: Option<Value>,
    #[serde(default)]
    message_id: Option<String>,
    #[serde(rename = "dispatchedAt", default)]
    dispatched_at: Option<String>,
    #[serde(rename = "completedAt", default)]
    completed_at: Option<String>,
    #[serde(rename = "startedAt", default)]
    started_at: Option<String>,
}

fn graph_statuses() -> &'static [&'static str] {
    &["active", "complete", "failed", "cancelled"]
}
fn node_statuses() -> &'static [&'static str] {
    &["pending", "dispatched", "active", "complete", "failed", "skipped", "cancelled"]
}
fn terminal_node(status: &str) -> bool {
    matches!(status, "complete" | "failed" | "skipped" | "cancelled")
}

/// TS `createGraphError` (`lib/task-graph.js:13`): the route maps the code
/// through `taskGraphErrorStatus` (`backend-v2.js:3961-3974`).
#[derive(Debug)]
struct GraphError {
    code: &'static str,
    message: String,
}
fn graph_error(code: &'static str, message: impl Into<String>) -> GraphError {
    GraphError { code, message: message.into() }
}

fn text(value: &Value, max: usize) -> Option<String> {
    let trimmed = value.as_str()?.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut out = trimmed.chars();
    let collected: String = out.by_ref().take(max).collect();
    Some(collected)
}

fn string_array(value: Option<&Value>, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(Value::Array(items)) = value {
        for item in items {
            if let Some(text) = text(item, max)
                && seen.insert(text.clone())
            {
                out.push(text);
            }
        }
    }
    out
}

fn normalize_graph_status(value: Option<&Value>) -> Result<String, GraphError> {
    let normalized = value.and_then(|v| text(v, 32)).unwrap_or_else(|| "active".into());
    if !graph_statuses().contains(&normalized.as_str()) {
        return Err(graph_error(
            "invalid_graph_status",
            format!("invalid graph status: {}", value.and_then(Value::as_str).unwrap_or("")),
        ));
    }
    Ok(normalized)
}
fn normalize_node_status(value: Option<&Value>, fallback: &str) -> Result<String, GraphError> {
    let normalized = value.and_then(|v| text(v, 32)).unwrap_or_else(|| fallback.into());
    if !node_statuses().contains(&normalized.as_str()) {
        return Err(graph_error(
            "invalid_node_status",
            format!("invalid node status: {}", value.and_then(Value::as_str).unwrap_or("")),
        ));
    }
    Ok(normalized)
}

fn normalize_result(value: Option<&Value>) -> Result<Value, GraphError> {
    let Some(value) = value else {
        return Ok(Value::Null);
    };
    let bytes = serde_json::to_vec(value).map_err(|_| graph_error("invalid_nodes", "invalid result"))?;
    if bytes.len() > MAX_RESULT_BYTES {
        return Err(graph_error(
            "result_too_large",
            format!("result exceeds {MAX_RESULT_BYTES} bytes when serialized"),
        ));
    }
    Ok(value.clone())
}

fn iso_now() -> String {
    // The retained store stamps ISO-8601 UTC (`new Date().toISOString()`).
    // Produce the same shape from the epoch without a date crate: seconds and
    // civil date via the civil-from-days algorithm.
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    iso_from_millis(millis)
}
fn iso_from_millis(millis: u64) -> String {
    let secs = millis / 1000;
    let ms = millis % 1000;
    let days = (secs / 86_400) as i64;
    let day_secs = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{ms:03}Z",
        day_secs / 3600,
        (day_secs % 3600) / 60,
        day_secs % 60,
    )
}
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
fn normalize_timestamp(value: Option<&Value>, fallback: Option<String>) -> Option<String> {
    let candidate = value.and_then(|v| text(v, 128))?;
    // TS `Date.parse` accepts any parseable date; we accept only ISO-8601 UTC
    // (the format this service itself emits) and otherwise keep the fallback.
    if candidate.len() == 24 && candidate.ends_with('Z') && candidate.as_bytes()[4] == b'-' {
        Some(candidate)
    } else {
        fallback
    }
}

fn normalize_condition(value: Option<&Value>) -> Result<Option<Value>, GraphError> {
    let Some(value) = value else { return Ok(None) };
    if value.is_null() {
        return Ok(None);
    }
    let Some(object) = value.as_object() else {
        return Err(graph_error("invalid_condition", "condition must be an object"));
    };
    let mut out = Map::new();
    for key in ["dep", "path", "field", "op"] {
        if let Some(text) = object.get(key).and_then(|v| text(v, 512)) {
            out.insert(if key == "field" { "path".into() } else { key.into() }, Value::String(text));
        }
    }
    for key in ["eq", "neq", "in", "value"] {
        if let Some(v) = object.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    if out.is_empty() { Ok(None) } else { Ok(Some(Value::Object(out))) }
}

fn normalize_node(id_key: &str, raw: &Value) -> Result<Node, GraphError> {
    let id = text(raw.get("id").unwrap_or(&Value::Null), 255)
        .or_else(|| text(&Value::String(id_key.into()), 255))
        .ok_or_else(|| graph_error("invalid_node_id", "node id required"))?;
    let assignee = text(raw.get("assignee").unwrap_or(&Value::Null), 255)
        .ok_or_else(|| graph_error("invalid_node_assignee", format!("node '{id}' assignee required")))?;
    let description = text(raw.get("description").unwrap_or(&Value::Null), 4000)
        .ok_or_else(|| graph_error("invalid_node_description", format!("node '{id}' description required")))?;
    let has_result = raw.get("result").is_some();
    Ok(Node {
        id: id.clone(),
        assignee,
        description,
        depends_on: string_array(raw.get("depends_on"), 255),
        status: normalize_node_status(raw.get("status"), "pending")?,
        result: if has_result {
            normalize_result(raw.get("result"))?
        } else {
            Value::Null
        },
        error: raw.get("error").and_then(|v| text(v, 4000)),
        condition: normalize_condition(raw.get("condition"))?,
        message_id: raw.get("message_id").and_then(|v| text(v, 255)),
        dispatched_at: normalize_timestamp(
            raw.get("dispatchedAt").or_else(|| raw.get("dispatched_at")),
            None,
        ),
        completed_at: normalize_timestamp(
            raw.get("completedAt").or_else(|| raw.get("completed_at")),
            None,
        ),
        started_at: normalize_timestamp(
            raw.get("startedAt").or_else(|| raw.get("started_at")),
            None,
        ),
    })
}

fn normalize_nodes(raw: Option<&Value>) -> Result<BTreeMap<String, Node>, GraphError> {
    let Some(Value::Object(entries)) = raw else {
        return Err(graph_error("invalid_nodes", "nodes must be an object"));
    };
    if entries.is_empty() {
        return Err(graph_error("invalid_nodes", "graph must contain at least one node"));
    }
    let mut out = BTreeMap::new();
    for (key, raw_node) in entries {
        let node = normalize_node(key, raw_node)?;
        out.insert(node.id.clone(), node);
    }
    for node in out.values() {
        for dep in &node.depends_on {
            if !out.contains_key(dep) {
                return Err(graph_error(
                    "invalid_dependency",
                    format!("node '{}' depends on missing node '{dep}'", node.id),
                ));
            }
            if dep == &node.id {
                return Err(graph_error(
                    "invalid_dependency",
                    format!("node '{}' cannot depend on itself", node.id),
                ));
            }
        }
        if let Some(condition) = &node.condition
            && let Some(dep) = condition.get("dep").and_then(Value::as_str)
            && !dep.is_empty()
            && !out.contains_key(dep)
        {
            return Err(graph_error(
                "invalid_condition",
                format!("node '{}' condition references missing dep '{dep}'", node.id),
            ));
        }
    }
    // Cycle detection, TS `visit` (`lib/task-graph.js:137-153`).
    fn visit(
        id: &str,
        nodes: &BTreeMap<String, Node>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.into()) {
            return Err(graph_error(
                "invalid_dependency_cycle",
                format!("dependency cycle detected at node '{id}'"),
            ));
        }
        for dep in &nodes[id].depends_on {
            visit(dep, nodes, visiting, visited)?;
        }
        visiting.remove(id);
        visited.insert(id.into());
        Ok(())
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in out.keys() {
        visit(id, &out, &mut visiting, &mut visited)?;
    }
    Ok(out)
}

fn normalize_graph(raw: &Value, id_fallback: Option<String>) -> Result<Graph, GraphError> {
    let id = text(raw.get("id").unwrap_or(&Value::Null), 255)
        .or(id_fallback)
        .ok_or_else(|| graph_error("invalid_nodes", "graph id required"))?;
    let owner = text(raw.get("owner").unwrap_or(&Value::Null), 255)
        .ok_or_else(|| graph_error("invalid_graph_owner", "graph owner required"))?;
    let label = text(raw.get("label").unwrap_or(&Value::Null), 4000)
        .ok_or_else(|| graph_error("invalid_graph_label", "graph label required"))?;
    let created_at = normalize_timestamp(
        raw.get("createdAt").or_else(|| raw.get("created_at")),
        Some(iso_now()),
    )
    .unwrap_or_else(iso_now);
    let updated_at = normalize_timestamp(
        raw.get("updatedAt").or_else(|| raw.get("updated_at")),
        Some(created_at.clone()),
    )
    .unwrap_or_else(|| created_at.clone());
    let completed_at = normalize_timestamp(
        raw.get("completedAt").or_else(|| raw.get("completed_at")),
        None,
    );
    Ok(Graph {
        id,
        owner,
        label,
        status: normalize_graph_status(raw.get("status"))?,
        nodes: normalize_nodes(raw.get("nodes"))?,
        created_at,
        updated_at,
        completed_at,
    })
}

fn default_graph_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut suffix = [0u8; 4];
    let _ = getrandom::fill(&mut suffix);
    let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let tail: String = suffix
        .iter()
        .map(|b| alphabet[(*b as usize) % 36] as char)
        .collect();
    // TS `graph_${Date.now()}_${random base36(6)}`.
    format!("graph_{millis}_{tail}")
}

/// The whole store: a map plus the state dir, mirroring TS's in-memory map
/// backed by `task_graphs.json`.
pub(crate) struct GraphStore {
    dir: PathBuf,
    inner: std::sync::Mutex<StoreInner>,
}
struct StoreInner {
    graphs: BTreeMap<String, Graph>,
    messages: Vec<Value>,
    counter: u64,
}

impl GraphStore {
    pub(super) fn open(dir: &Path) -> Result<Self, Error> {
        let graphs = read_json(&dir.join(GRAPHS_FILE)).unwrap_or_else(|| json!({}));
        let messages = read_json(&dir.join(MESSAGES_FILE)).unwrap_or_else(|| json!([]));
        let counter = read_json(&dir.join(COUNTER_FILE)).unwrap_or_else(|| json!(0));
        let mut map = BTreeMap::new();
        if let Value::Object(entries) = &graphs {
            for (key, raw) in entries {
                if let Ok(mut graph) = normalize_graph(raw, Some(key.clone()).filter(|k| !k.is_empty())) {
                    if graph.id.is_empty() {
                        graph.id = key.clone();
                    }
                    map.insert(graph.id.clone(), graph);
                }
            }
        }
        let messages_vec = messages.as_array().cloned().unwrap_or_default();
        // TS boot reconciles the counter against persisted message ids
        // (`backend-v2.js:2829-2834`).
        let mut counter = counter.as_u64().unwrap_or(0);
        for message in &messages_vec {
            if let Some(n) = message
                .get("id")
                .and_then(Value::as_str)
                .and_then(message_counter_from_id)
            {
                counter = counter.max(n);
            }
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            inner: std::sync::Mutex::new(StoreInner {
                graphs: map,
                messages: messages_vec,
                counter,
            }),
        })
    }

    fn persist_graphs(&self, graphs: &BTreeMap<String, Graph>) -> Result<(), GraphError> {
        let value = serde_json::to_value(graphs)
            .map_err(|_| graph_error("graph_persistence_failed", "task graph persistence failed"))?;
        if serde_json::to_vec(&value).map(|v| v.len()).unwrap_or(0) > MAX_GRAPH_BYTES {
            return Err(graph_error("graph_persistence_failed", "task graph persistence failed"));
        }
        hagency_store::private::replace(
            &self.dir.join(GRAPHS_FILE),
            value.to_string().as_bytes(),
        )
        .map_err(|_| graph_error("graph_persistence_failed", "task graph persistence failed"))
    }
    fn persist_messages(&self, messages: &[Value]) -> Result<(), GraphError> {
        let value = Value::Array(messages.to_vec());
        hagency_store::private::replace(
            &self.dir.join(MESSAGES_FILE),
            value.to_string().as_bytes(),
        )
        .map_err(|_| graph_error("graph_dispatch_failed", "message persistence failed"))
    }
    fn persist_counter(&self, counter: u64) -> Result<(), GraphError> {
        hagency_store::private::replace(
            &self.dir.join(COUNTER_FILE),
            counter.to_string().as_bytes(),
        )
        .map_err(|_| graph_error("graph_dispatch_failed", "msg_counter persistence failed"))
    }

    /// TS `dispatchTaskGraphMessage` -> `dispatchInternalDirectMessage`:
    /// one durable, name-addressed message with the `task_graph_dispatch`
    /// schema, deduped on the dispatch key. Returns the message id.
    fn dispatch(&self, inner: &mut StoreInner, graph: &Graph, node: &Node) -> Result<String, GraphError> {
        let dispatch_key = format!("task_graph_dispatch:{}:{}", graph.id, node.id);
        if let Some(existing) = inner.messages.iter().rev().find(|m| {
            m.get("schema")
                .and_then(|s| s.get("payload"))
                .and_then(|p| p.get("dispatchKey"))
                .and_then(Value::as_str)
                == Some(dispatch_key.as_str())
        }) {
            return existing
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| graph_error("graph_dispatch_failed", "task graph dispatch did not return a durable message id"));
        }
        let next = inner.counter + 1;
        self.persist_counter(next)?;
        let dependency_results: Vec<Value> = node
            .depends_on
            .iter()
            .filter(|dep| graph.nodes.get(*dep).map(|d| d.status.as_str()) == Some("complete"))
            .map(|dep| {
                let d = &graph.nodes[dep];
                json!({"nodeId": dep, "assignee": d.assignee, "result": d.result})
            })
            .collect();
        let deps_text = if dependency_results.is_empty() {
            "No dependencies.".to_string()
        } else {
            format!(
                "Dependency results:\n{}",
                dependency_results
                    .iter()
                    .map(|d| {
                        let serialized = d.get("result").map(Value::to_string).unwrap_or_default();
                        let truncated: String = serialized.chars().take(500).collect();
                        format!(
                            "- {} ({}): {}",
                            d["nodeId"].as_str().unwrap_or_default(),
                            d["assignee"].as_str().unwrap_or_default(),
                            truncated
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        let message = json!({
            "id": format!("msg_{next:04}"),
            "ts": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0),
            "from": graph.owner,
            "to": node.assignee,
            "group": Value::Null,
            "type": "request",
            "priority": "high",
            "summary": format!("Task assigned: {}", node.description),
            "full": format!("## Task Graph Assignment\n\nGraph: {} ({})\nNode: {}\nDescription: {}\n\n{}", graph.label, graph.id, node.id, node.description, deps_text),
            "mentions": [],
            "reply_to": Value::Null,
            "source": "system",
            "sourceRoom": Value::Null,
            "viewToken": view_token(),
            "schema": {
                "kind": "task_graph_dispatch",
                "version": 1,
                "payload": {
                    "dispatchKey": dispatch_key,
                    "graphId": graph.id,
                    "nodeId": node.id,
                    "description": node.description,
                    "dependencyResults": dependency_results,
                }
            }
        });
        let mut messages = inner.messages.clone();
        messages.push(message.clone());
        self.persist_messages(&messages)?;
        inner.counter = next;
        inner.messages = messages;
        message
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| graph_error("graph_dispatch_failed", "task graph dispatch did not return a durable message id"))
    }

    fn create(&self, raw: &Value) -> Result<Graph, GraphError> {
        let mut inner = self.inner.lock().map_err(|_| graph_error("graph_persistence_failed", "task graph persistence failed"))?;
        let graph = normalize_graph(raw, Some(default_graph_id()).filter(|_| raw.get("id").is_none()))?;
        if inner.graphs.contains_key(&graph.id) {
            return Err(graph_error("graph_exists", format!("graph already exists: {}", graph.id)));
        }
        let mut next = inner.graphs.clone();
        next.insert(graph.id.clone(), graph.clone());
        self.persist_graphs(&next)?;
        inner.graphs = next;
        Ok(graph)
    }

    fn get(&self, id: &str) -> Option<Graph> {
        self.inner.lock().ok()?.graphs.get(id).cloned()
    }
    fn list(&self, status: Option<&str>) -> Vec<Graph> {
        let mut graphs: Vec<Graph> = self
            .inner
            .lock()
            .map(|i| {
                i.graphs
                    .values()
                    .filter(|g| status.is_none_or(|s| g.status == s))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        graphs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        graphs
    }

    /// TS `deleteGraph` (`lib/task-graph.js:421`): a cancel, not a removal.
    fn delete(&self, id: &str) -> Result<Option<Graph>, GraphError> {
        let mut inner = self.inner.lock().map_err(|_| graph_error("graph_persistence_failed", "task graph persistence failed"))?;
        if !inner.graphs.contains_key(id) {
            return Ok(None);
        }
        let mut next = inner.graphs.clone();
        let graph = next.get_mut(id).expect("checked above");
        if graph.status != "cancelled" {
            let at = iso_now();
            graph.status = "cancelled".into();
            graph.completed_at = Some(at.clone());
            graph.updated_at = at.clone();
            for node in graph.nodes.values_mut() {
                if !terminal_node(&node.status) {
                    node.status = "cancelled".into();
                    node.completed_at = Some(at.clone());
                }
            }
            self.persist_graphs(&next)?;
            inner.graphs = next;
        }
        Ok(inner.graphs.get(id).cloned())
    }

    /// TS `updateNode` (`lib/task-graph.js:455`).
    fn update_node(&self, id: &str, node_id: &str, patch: &Value) -> Result<(Graph, Node), GraphError> {
        let mut inner = self.inner.lock().map_err(|_| graph_error("graph_persistence_failed", "task graph persistence failed"))?;
        if !inner.graphs.contains_key(id) {
            return Err(graph_error("graph_not_found", format!("graph not found: {id}")));
        }
        if !inner.graphs[id].nodes.contains_key(node_id) {
            return Err(graph_error("node_not_found", format!("node not found: {node_id}")));
        }
        let has_status = patch.get("status").is_some();
        let has_result = patch.get("result").is_some();
        let has_error = patch.get("error").is_some();
        if !has_status && !has_result && !has_error {
            return Err(graph_error("invalid_patch", "node patch requires status, result, or error"));
        }
        let mut next = inner.graphs.clone();
        let graph = next.get_mut(id).expect("checked");
        let node = graph.nodes.get_mut(node_id).expect("checked");
        let normalized_result = if has_result { Some(normalize_result(patch.get("result"))?) } else { None };
        let mut changed = false;
        if has_status {
            let next_status = normalize_node_status(patch.get("status"), &node.status.clone())?;
            if next_status != node.status {
                node.status = next_status.clone();
                changed = true;
            }
            if next_status == "active" && node.started_at.is_none() {
                node.started_at = Some(iso_now());
                changed = true;
            }
            if next_status == "dispatched" && node.dispatched_at.is_none() {
                node.dispatched_at = Some(iso_now());
                changed = true;
            }
            if terminal_node(&next_status) {
                if node.completed_at.is_none() {
                    node.completed_at = Some(iso_now());
                }
                if next_status == "complete" && has_result {
                    node.result = normalized_result.clone().unwrap_or(Value::Null);
                }
                if next_status == "failed" {
                    node.error = patch
                        .get("error")
                        .and_then(|v| text(v, 4000))
                        .or_else(|| node.error.clone())
                        .or_else(|| Some("node failed".into()));
                }
                changed = true;
            }
        }
        if has_result
            && (!has_status || node.status == "complete" || node.status == "active" || node.status == "dispatched")
        {
            node.result = normalized_result.clone().unwrap_or(Value::Null);
            changed = true;
        }
        if has_error {
            node.error = patch.get("error").and_then(|v| text(v, 4000));
            changed = true;
        }
        if changed {
            graph.updated_at = iso_now();
            self.persist_graphs(&next)?;
            inner.graphs = next;
        }
        let graph = inner.graphs[id].clone();
        let node = graph.nodes[node_id].clone();
        Ok((graph, node))
    }

    /// TS `advanceGraph` (`lib/task-graph.js:522`): dispatch ready roots,
    /// cascade failed dependencies, finalize a terminal graph.
    fn advance(&self, id: &str) -> Result<Option<Graph>, GraphError> {
        let mut inner = self.inner.lock().map_err(|_| graph_error("graph_persistence_failed", "task graph persistence failed"))?;
        if !inner.graphs.contains_key(id) {
            return Ok(None);
        }
        let mut next = inner.graphs.clone();
        let graph = next.get_mut(id).expect("checked");
        if graph.status != "active" {
            return Ok(Some(graph.clone()));
        }
        let mut changed = false;
        loop {
            let mut progress = false;
            let ids: Vec<String> = graph.nodes.keys().cloned().collect();
            for node_id in ids {
                if graph.nodes[&node_id].status != "pending" {
                    continue;
                }
                let node = graph.nodes[&node_id].clone();
                let failed: Vec<String> = node
                    .depends_on
                    .iter()
                    .filter(|dep| matches!(graph.nodes.get(*dep).map(|d| d.status.as_str()), Some("failed" | "cancelled")))
                    .cloned()
                    .collect();
                if !failed.is_empty() {
                    let n = graph.nodes.get_mut(&node_id).expect("checked");
                    n.status = "failed".into();
                    n.completed_at = Some(iso_now());
                    n.error = Some(format!("dependency failed: {}", failed.join(", ")));
                    changed = true;
                    progress = true;
                    continue;
                }
                let all_resolved = node
                    .depends_on
                    .iter()
                    .all(|dep| matches!(graph.nodes.get(dep).map(|d| d.status.as_str()), Some("complete" | "skipped")));
                if !all_resolved {
                    continue;
                }
                match evaluate_condition(graph, &node) {
                    None => continue,
                    Some(false) => {
                        let n = graph.nodes.get_mut(&node_id).expect("checked");
                        n.status = "skipped".into();
                        n.completed_at = Some(iso_now());
                        changed = true;
                        progress = true;
                        continue;
                    }
                    Some(true) => {}
                }
                let message_id = self.dispatch(&mut inner, graph, &node)?;
                let n = graph.nodes.get_mut(&node_id).expect("checked");
                n.status = "dispatched".into();
                n.dispatched_at = Some(iso_now());
                n.message_id = Some(message_id);
                changed = true;
                progress = true;
            }
            if !progress {
                break;
            }
        }
        let all_terminal = !graph.nodes.is_empty()
            && graph.nodes.values().all(|n| terminal_node(&n.status));
        if all_terminal && graph.status == "active" {
            let at = iso_now();
            graph.status = if graph.nodes.values().any(|n| n.status == "failed") { "failed" } else { "complete" }.into();
            graph.completed_at = Some(at.clone());
            graph.updated_at = at;
            changed = true;
        }
        if changed {
            graph.updated_at = iso_now();
            self.persist_graphs(&next)?;
            inner.graphs = next;
        }
        Ok(inner.graphs.get(id).cloned())
    }
}

fn nested<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty()
        || path
            .split('.')
            .any(|part| matches!(part, "__proto__" | "constructor" | "prototype"))
    {
        return None;
    }
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

/// TS `evaluateCondition` (`lib/task-graph.js:173`): None = not yet decidable,
/// Some(false) = skip, Some(true) = dispatch.
fn evaluate_condition(graph: &Graph, node: &Node) -> Option<bool> {
    // TS `if (!node?.condition) return true;` — no condition always passes.
    let Some(condition) = node.condition.as_ref() else {
        return Some(true);
    };
    let dep_id = condition
        .get("dep")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| node.depends_on.first().cloned())?;
    let dep = graph.nodes.get(&dep_id)?;
    match dep.status.as_str() {
        "pending" | "dispatched" | "active" => return None,
        "complete" => {}
        _ => return Some(false),
    }
    let path = condition
        .get("path")
        .or_else(|| condition.get("field"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    let value = path.and_then(|p| nested(&dep.result, p)).cloned().unwrap_or_else(|| dep.result.clone());
    if let Some(eq) = condition.get("eq") {
        return Some(&value == eq);
    }
    if let Some(neq) = condition.get("neq") {
        return Some(&value != neq);
    }
    if let Some(set) = condition.get("in") {
        return Some(set.as_array().is_some_and(|a| a.contains(&value)));
    }
    match condition.get("op").and_then(Value::as_str) {
        Some("eq") => return Some(condition.get("value").is_some_and(|v| &value == v)),
        Some("neq") => return Some(condition.get("value").is_none_or(|v| &value != v)),
        Some("in") => {
            return Some(
                condition
                    .get("value")
                    .and_then(Value::as_array)
                    .is_some_and(|a| a.contains(&value)),
            );
        }
        _ => {}
    }
    Some(!value.is_null() && value != Value::Bool(false))
}

fn view_token() -> String {
    // TS `createMessageViewToken` = 24 random bytes, base64url.
    let mut bytes = [0u8; 24];
    let _ = getrandom::fill(&mut bytes);
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(32);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        }
    }
    out
}

fn message_counter_from_id(id: &str) -> Option<u64> {
    let rest = id.strip_prefix("msg_")?;
    let value: u64 = rest.parse().ok()?;
    (value > 0).then_some(value)
}

fn read_json(path: &Path) -> Option<Value> {
    let bytes = hagency_store::private::read_secret(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn error_response(res: &mut Response, error: GraphError, fallback: &str) {
    // TS `taskGraphErrorStatus` (`backend-v2.js:3961`).
    let status = match error.code {
        "graph_not_found" | "node_not_found" => StatusCode::NOT_FOUND,
        "graph_exists" => StatusCode::CONFLICT,
        "graph_persistence_failed" | "graph_dispatch_failed" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::BAD_REQUEST,
    };
    res.status_code(status);
    res.render(Json(json!({ "error": if error.message.is_empty() { fallback.into() } else { error.message } })));
}

fn store_for<'a>(depot: &'a Depot) -> Option<&'a GraphStore> {
    console(depot).ok()?.graphs()
}

fn graph_id(req: &Request) -> Result<String, Error> {
    let id = req.param::<String>("id").ok_or(Error::Invalid)?;
    if text(&Value::String(id.clone()), 255).is_none() {
        return Err(Error::Invalid);
    }
    Ok(id)
}

#[handler]
async fn create(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    let Some(store) = store_for(depot) else {
        refusal(res, StatusCode::SERVICE_UNAVAILABLE, "console_unavailable");
        return;
    };
    let input: Value = match serde_json::from_slice(&match body(req, 64 * 1024).await {
        Ok(bytes) => bytes,
        Err(error) => {
            failed(res, error);
            return;
        }
    }) {
        Ok(value) => value,
        Err(_) => {
            failed(res, Error::Invalid);
            return;
        }
    };
    match store.create(&input).and_then(|created| {
        store
            .advance(&created.id)
            .map(|advanced| advanced.unwrap_or(created))
    }) {
        Ok(graph) => res.render(Json(json!({ "ok": true, "graph": graph }))),
        Err(error) => error_response(res, error, "failed to create task graph"),
    }
}

#[handler]
async fn list(req: &mut Request, depot: &Depot, res: &mut Response) {
    let Some(store) = store_for(depot) else {
        refusal(res, StatusCode::SERVICE_UNAVAILABLE, "console_unavailable");
        return;
    };
    let status = req.query::<String>("status").and_then(|s| text(&Value::String(s), 32));
    let status = match status {
        Some(status) if !graph_statuses().contains(&status.as_str()) => {
            error_response(
                res,
                graph_error("invalid_graph_status", format!("invalid graph status: {status}")),
                "failed to list task graphs",
            );
            return;
        }
        other => other,
    };
    res.render(Json(json!(store.list(status.as_deref()))));
}

#[handler]
async fn read(req: &mut Request, depot: &Depot, res: &mut Response) {
    let Some(store) = store_for(depot) else {
        refusal(res, StatusCode::SERVICE_UNAVAILABLE, "console_unavailable");
        return;
    };
    let id = match graph_id(req) {
        Ok(id) => id,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    match store.get(&id) {
        Some(graph) => res.render(Json(json!(graph))),
        None => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Json(json!({ "error": "task graph not found" })));
        }
    }
}

#[handler]
async fn remove(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    let Some(store) = store_for(depot) else {
        refusal(res, StatusCode::SERVICE_UNAVAILABLE, "console_unavailable");
        return;
    };
    let id = match graph_id(req) {
        Ok(id) => id,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    match store.delete(&id) {
        Ok(Some(graph)) => res.render(Json(json!({ "ok": true, "graph": graph }))),
        Ok(None) => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Json(json!({ "error": "task graph not found" })));
        }
        Err(error) => error_response(res, error, "failed to delete task graph"),
    }
}

#[handler]
async fn update_node(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    // TS gates the node PATCH on the node ASSIGNEE's agent token
    // (`backend-v2.js:16009` `requireAgentToken(_tokenFromNodeAssignee)`).
    // The native console session is the operator's authority over the same
    // surface, so the console login stands in for it here; the wire shape and
    // behaviour are unchanged.
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    let Some(store) = store_for(depot) else {
        refusal(res, StatusCode::SERVICE_UNAVAILABLE, "console_unavailable");
        return;
    };
    let id = match graph_id(req) {
        Ok(id) => id,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    let node_id = req.param::<String>("node_id").ok_or(Error::Invalid);
    let node_id = match node_id {
        Ok(node_id) if text(&Value::String(node_id.clone()), 255).is_some() => node_id,
        _ => {
            failed(res, Error::Invalid);
            return;
        }
    };
    let patch: Value = match serde_json::from_slice(&match body(req, 128 * 1024).await {
        Ok(bytes) => bytes,
        Err(error) => {
            failed(res, error);
            return;
        }
    }) {
        Ok(value) => value,
        Err(_) => {
            failed(res, Error::Invalid);
            return;
        }
    };
    let updated = store.update_node(&id, &node_id, &patch);
    match updated {
        Ok((graph, node)) => {
            let advanced = store.advance(&id).ok().flatten().unwrap_or(graph);
            res.render(Json(json!({ "ok": true, "graph": advanced, "node": node })))
        }
        Err(error) => error_response(res, error, "failed to update task graph node"),
    }
}
