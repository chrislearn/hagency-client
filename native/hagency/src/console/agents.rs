//! The read-only agent roster (ADR-126, widened by board #22): a bounded
//! observation of the engagement projections, mounted under the API
//! sub-router's `authenticate` hoop with NO scope — the same read class as
//! `engagements` and the resources list: scope facts are a payload on
//! reads, never a gate (only the mutations consult a scope). The wire
//! item carries EXACTLY nine scalar keys derived at the store
//! (`DomainRepository::agent_roster`): no credential home, no workdir,
//! no state dir, no workspace path, no tmux target, no pane buffer, no
//! token — and no nested object at all, so nothing can hide inside one.
//! The client validator refuses a tenth key, so a future widening
//! fails the whole read instead of leaking silently (fail-closed).
//!
//! One row per AGENT, the TS roster's shape (`backend-v2.js:11696`
//! serializes every agent record): an agent whose engagements all ended
//! still appears. `online` is REAL worker state — a live dispatch
//! (`leased`/`started`/`parked`) in one of the agent's sessions — and
//! `last_seen_ms` is the newest `runner_attempts.created_at` the agent
//! produced: null, never zero, when it never attempted. `last_activity_ms`
//! keeps "last dispatch activity" of the representative engagement.
//! `unavailable` is server-owned, like the alert transition map: the
//! page renders whatever the server names, so a future source turns a
//! column on by removing its name here, not by a client edit.
use super::resources::failure;
use super::{Error, Session, body, console, failed, recheck, usage::query};
use crate::{refusal, resources::domain};
use hagency_core::project::{EngagementState, identifier};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

pub(super) fn router() -> Router {
    Router::with_path("agents")
        .get(list)
        .push(Router::with_path("{name}").get(super::agent_detail::detail))
        .push(Router::with_path("{id}/start").post(start))
        .push(Router::with_path("{id}/stop").post(stop))
        .push(Router::with_path("{id}/preset").post(preset))
        .push(Router::with_path("{id}/recover-dispatch").post(recover_dispatch))
        .push(Router::with_path("{id}/stopped-dispatches").get(stopped_dispatches))
        .push(
            Router::with_path("{id}/stopped-dispatches/{dispatch}/inspection")
                .get(stopped_dispatch_inspection),
        )
        .push(
            Router::with_path("{id}/stopped-dispatches/{dispatch}/inspect")
                .post(begin_outcome_inspection),
        )
        .push(Router::with_path("{id}/resolve-stopped-dispatch").post(resolve_stopped_dispatch))
        .push(Router::with_path("{id}/continue-stopped-dispatch").post(continue_stopped_dispatch))
        .push(Router::with_path("{id}/refuse").post(refuse))
}

/// Exactly twelve keys, in the ADR-126 order extended by board #22 and
/// board #60. Every key except `name`, `framework`, `role`, `state`,
/// `engagement_id`, `requested_tokens`, `online` and `seat` is nullable at
/// the source; `null` means "unknown", rendered as such. `consumed` is
/// `null` when nothing was measured — unknown, never zero. One row per
/// AGENT (TS parity: `backend-v2.js:11696` serializes every agent record).
#[derive(Serialize)]
struct RosterItem {
    name: String,
    framework: String,
    role: String,
    /// The ENGAGEMENT LIFECYCLE word (pending/reserved/active/…), which is
    /// what the store's projection carries.
    state: EngagementState,
    engagement_id: String,
    requested_tokens: u64,
    online: bool,
    last_seen_ms: Option<u64>,
    last_activity_ms: Option<u64>,
    /// The LIVE DISPATCH's word, separate from `state` by construction
    /// (board #60 item 2; TS `:6872` reads `machine.state`, a liveness
    /// value). Null when native's dispatch record shows no live dispatch.
    liveness: Option<String>,
    /// Tokens observed consumed by the agent's engagements, summed the way
    /// the usage report sums it. Null when nothing was measured.
    consumed: Option<u64>,
}

/// Board #60 item 2. The columns that were printed as `unknown` are now
/// ANSWERED from native state — `consumed` (`usage_sources.latest_counts`)
/// and `liveness` (the live dispatch row, separate from the engagement
/// `state` word) — so they render as columns and nothing is left to name.
///
/// The four that cannot be answered are DROPPED rather than printed as
/// "unknown", and `seat` joins them: TS's own roster serializer
/// (`backend-v2.js:6822-6916`) carries no seat, and this codebase treats the
/// seat id as private (`console/accounts.rs` deliberately withholds
/// `seat_id`; the roster fixture asserts a seat name never reaches the
/// wire). Filling it would invent a field TS does not have, so the honest
/// reading of "fill what native can answer" is that `seat` is not one of
/// them. `tmux`/`pane`/`credential_home`/`workspace_path` are unanswerable
/// by design (ADR-126 keeps panes and paths off the wire, and the store
/// holds no such column).
///
/// The list is kept, empty, because it is the server-owned mechanism the
/// page renders verbatim: a future column with no source turns itself on by
/// being named here.
const UNAVAILABLE: [&str; 0] = [];

#[handler]
async fn list(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    // A roster takes no selection: any query parameter is refused, the
    // same hygiene the alerts read applies to its own allowlist.
    if query(req, &[], 0).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store.agent_roster().await;
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    match result {
        Ok(rows) => {
            // Statement time, as the alerts read does: the roster has no
            // clock parameter to honor.
            let at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| u64::try_from(d.as_millis()).ok())
                .unwrap_or_default();
            let agents: Vec<_> = rows
                .into_iter()
                .map(|row| RosterItem {
                    name: row.name,
                    framework: row.framework,
                    role: row.role,
                    state: row.state,
                    engagement_id: row.engagement_id,
                    requested_tokens: row.requested_tokens,
                    online: row.online,
                    last_seen_ms: row.last_seen_ms,
                    last_activity_ms: row.last_activity_ms,
                    liveness: row.liveness,
                    consumed: row.consumed,
                })
                .collect();
            // CL-S2 (ADR-130): the lifecycle controls render ONLY from the
            // served boolean — a read-only session renders none enabled.
            let manage_lifecycle = match depot.get_typed::<Session>() {
                Ok(session) => {
                    match console(depot).and_then(|c| c.0.authority.can_lifecycle(session)) {
                        Ok(value) => value,
                        Err(error) => {
                            failed(res, error);
                            return;
                        }
                    }
                }
                Err(_) => {
                    failed(res, Error::Unauthorized);
                    return;
                }
            };
            res.render(Json(serde_json::json!({
                "at_ms": at_ms,
                "unavailable": UNAVAILABLE,
                "agents": agents,
                "permissions": {"manageLifecycle": manage_lifecycle},
            })));
        }
        Err(error) => store_error(res, error),
    }
}

/// The engagement id path parameter — an opaque identifier, never a path
/// component that can address a session, authority or filesystem.
fn engagement_id(req: &Request) -> Result<String, Error> {
    let id = req.param::<String>("id").ok_or(Error::Invalid)?;
    identifier(&id, 128).map_err(|_| Error::Invalid)?;
    Ok(id)
}

/// The lifecycle routes share one scope gate: a valid session whose
/// grant is `Scope::AgentLifecycle`. A read-only or other-scoped session is
/// refused with `agent_lifecycle_scope_required` before any store work.
fn check_lifecycle(depot: &Depot, res: &mut Response) -> bool {
    let session = match depot.get_typed::<Session>() {
        Ok(session) => session,
        Err(_) => {
            failed(res, Error::Unauthorized);
            return false;
        }
    };
    match console(depot).and_then(|c| c.0.authority.can_lifecycle(session)) {
        Ok(true) => true,
        Ok(false) => {
            failed(res, Error::LifecycleForbidden);
            false
        }
        Err(error) => {
            failed(res, error);
            false
        }
    }
}

/// Native has no durable agent lifecycle record or safe way to re-arm a
/// stop-fenced dispatch. A successful no-op here would lie to the operator,
/// so the retained route fails closed until a host-owned start transition is
/// implemented. It never reads the roster and never spawns a process.
#[handler]
async fn start(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if query(req, &[], 0).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    if let Err(error) = engagement_id(req) {
        failed(res, error);
        return;
    }
    if !check_lifecycle(depot, res) {
        return;
    }
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    refusal(res, StatusCode::NOT_IMPLEMENTED, "agent_start_unavailable");
}

/// Stop — fence, never settle: the store resolves the named engagement's
/// dispatch (live set or unsettled stop row) and serves the five-key wire
/// object verbatim. Idempotent by construction (see `stop_dispatch_for_agent`).
#[handler]
async fn stop(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if query(req, &[], 0).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    let id = match engagement_id(req) {
        Ok(id) => id,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    if !check_lifecycle(depot, res) {
        return;
    }
    let Some(store) = domain(depot, res) else {
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or_default();
    let result = store.stop_dispatch_for_agent(id, now).await;
    if result.is_ok() && recheck(depot).is_err() {
        failure(res, hagency_store::Error::OutcomeUnknown);
        return;
    }
    match result {
        Ok(value) => res.render(Json(value)),
        Err(error) => failure(res, error),
    }
}

/// Refuse — the operator's verdict against a pending engagement request
/// (parity: the retained `POST /api/engagements/:id/verdict` else-branch,
/// backend-v2.js:15160-15187 → lib/engagement-store.js:593-613). Reaches the
/// store's own refusal arm `DomainStore::reject` → `end(..., revoke = false)`
/// — never a second write path; the pending-only guard, decision idempotency
/// and the engagement_ends stamp stay the store's. Triggered ONLY by this
/// route, under the existing `Scope::AgentLifecycle` (same class as retire);
/// no timer, no sweep, and a refusal schedules no retirement work.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Refuse {
    command_id: String,
}

#[handler]
async fn refuse(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if query(req, &[], 0).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    let id = match engagement_id(req) {
        Ok(id) => id,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    if !check_lifecycle(depot, res) {
        return;
    }
    let input: Refuse =
        match serde_json::from_slice::<Refuse>(&body(req, 512).await.unwrap_or_default()) {
            Ok(input) if identifier(&input.command_id, 128).is_ok() => input,
            _ => {
                failed(res, Error::Invalid);
                return;
            }
        };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store.reject(input.command_id, id).await;
    if result.is_ok() && recheck(depot).is_err() {
        failure(res, hagency_store::Error::OutcomeUnknown);
        return;
    }
    match result {
        Ok(engagement) => res.render(Json(serde_json::json!({"engagement": engagement}))),
        Err(hagency_store::Error::State) => {
            // The pending-only guard (domain.rs:1294-1300): refusal of an
            // already-terminal or reserved/active engagement, conflict word as
            // the retained verdict route maps it (409).
            refusal(res, StatusCode::CONFLICT, "engagement_not_pending")
        }
        Err(hagency_store::Error::NotFound) => refusal(res, StatusCode::NOT_FOUND, "not_found"),
        Err(hagency_store::Error::Conflict) => {
            // Reused command id with a different decision digest
            // (replay_decision, domain.rs:359-381): a changed replay, never a
            // silent second write.
            refusal(res, StatusCode::CONFLICT, "decision_conflict")
        }
        Err(error) => failure(res, error),
    }
}

/// Recover-dispatch — operator recovery and resume of an orphaned dispatch
/// (ADR-148). The operator names the crashed dispatch's replacement and the
/// evidence of what was inspected; the store enforces the orphan state, the
/// stop-row refusal and the evidence record. Triggered ONLY by this route,
/// under the existing `Scope::AgentLifecycle` — never automatic, never a sweep,
/// and no second clearer over the stop-fenced sibling's state.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RecoverDispatch {
    original: String,
    replacement: hagency_core::tasks::DispatchInput,
    evidence: String,
}

#[handler]
async fn recover_dispatch(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if query(req, &[], 0).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    let id = match engagement_id(req) {
        Ok(id) => id,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    if !check_lifecycle(depot, res) {
        return;
    }
    let raw = match body(req, 8192).await {
        Ok(raw) => raw,
        Err(_) => {
            failed(res, Error::Invalid);
            return;
        }
    };
    let input: RecoverDispatch = match serde_json::from_slice(&raw) {
        Ok(input) => input,
        Err(_) => {
            failed(res, Error::Invalid);
            return;
        }
    };
    if input.replacement.validate().is_err()
        || identifier(&input.original, 128).is_err()
        || input.evidence.is_empty()
        || input.evidence.chars().count() > 4096
    {
        failed(res, Error::Invalid);
        return;
    }
    let Some(store) = domain(depot, res) else {
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or_default();
    let result = store
        .recover_dispatch(
            id,
            input.original.clone(),
            input.replacement,
            input.evidence,
            now,
        )
        .await;
    if result.is_ok() && recheck(depot).is_err() {
        failure(res, hagency_store::Error::OutcomeUnknown);
        return;
    }
    match result {
        Ok(()) => res.render(Json(serde_json::json!({"ok": true}))),
        Err(hagency_store::Error::State) => {
            // The orphan-state guard or the stop-row refusal (execution.rs:1044-1050).
            refusal(res, StatusCode::CONFLICT, "dispatch_not_recoverable")
        }
        Err(hagency_store::Error::NotFound) => refusal(res, StatusCode::NOT_FOUND, "not_found"),
        // The original was already recovered, or the replacement id is taken:
        // the recovery happened once and a replay mints nothing (live proof
        // 2026-09-22 answered this with the resources page's revision word).
        Err(hagency_store::Error::Conflict) => {
            refusal(res, StatusCode::CONFLICT, "recovery_conflict")
        }
        Err(error) => failure(res, error),
    }
}

/// Bounded private discovery; inspection availability grants no resolution.
#[handler]
async fn stopped_dispatches(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if !check_lifecycle(depot, res) {
        return;
    }
    let prepared = (|| {
        query(req, &["after"], 160)?;
        let agent = engagement_id(req)?;
        let after = req.query::<String>("after").unwrap_or_default();
        if !after.is_empty() {
            identifier(&after, 128).map_err(|_| Error::Invalid)?;
        }
        Ok::<_, Error>((agent, after))
    })();
    let (agent, after) = match prepared {
        Ok(value) => value,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store.stopped_dispatches_for_agent(agent, after).await;
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    outcome_response(res, result);
}

/// The historical content inventory is private operator data, even on GET.
#[handler]
async fn stopped_dispatch_inspection(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if !check_lifecycle(depot, res) {
        return;
    }
    let prepared = (|| {
        query(req, &[], 0)?;
        let agent = engagement_id(req)?;
        let id = req.param::<String>("dispatch").ok_or(Error::Invalid)?;
        identifier(&id, 128).map_err(|_| Error::Invalid)?;
        Ok::<_, Error>((agent, id))
    })();
    let (agent, id) = match prepared {
        Ok(value) => value,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store.stopped_dispatch_inspection(agent, id).await;
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    match result {
        Ok(value) => res.render(Json(value)),
        Err(hagency_store::Error::NotFound) => refusal(res, StatusCode::NOT_FOUND, "not_found"),
        Err(error) => failure(res, error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ContinueStoppedDispatch {
    original: String,
    fence: u64,
    inspection_digest: String,
    replacement: hagency_core::tasks::DispatchInput,
    evidence: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct OutcomeInspection {
    #[serde(default = "inspection_lifetime")]
    ttl_ms: u64,
}
fn inspection_lifetime() -> u64 {
    900_000
}

fn outcome_response(res: &mut Response, result: Result<serde_json::Value, hagency_store::Error>) {
    match result {
        Ok(value) => res.render(Json(value)),
        Err(hagency_store::Error::NotFound) => refusal(res, StatusCode::NOT_FOUND, "not_found"),
        Err(hagency_store::Error::Invalid(_)) => failed(res, Error::Invalid),
        Err(hagency_store::Error::Conflict) => {
            refusal(res, StatusCode::CONFLICT, "resolution_conflict")
        }
        Err(
            hagency_store::Error::State
            | hagency_store::Error::Quarantined
            | hagency_store::Error::RunnerAuthority
            | hagency_store::Error::Generation,
        ) => refusal(res, StatusCode::CONFLICT, "dispatch_not_resolvable"),
        Err(error) => failure(res, error),
    }
}

#[handler]
async fn begin_outcome_inspection(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if !check_lifecycle(depot, res) {
        return;
    }
    let prepared = async {
        query(req, &[], 0)?;
        let agent = engagement_id(req)?;
        let id = req.param::<String>("dispatch").ok_or(Error::Invalid)?;
        identifier(&id, 128).map_err(|_| Error::Invalid)?;
        let input: OutcomeInspection =
            serde_json::from_slice(&body(req, 512).await?).map_err(|_| Error::Invalid)?;
        Ok::<_, Error>((agent, id, input))
    }
    .await;
    let (agent, id, input) = match prepared {
        Ok(value) => value,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store
        .begin_outcome_inspection(agent, id, input.ttl_ms)
        .await;
    if result.is_ok() && recheck(depot).is_err() {
        failure(res, hagency_store::Error::OutcomeUnknown);
        return;
    }
    outcome_response(res, result);
}

#[handler]
async fn resolve_stopped_dispatch(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if !check_lifecycle(depot, res) {
        return;
    }
    let prepared = async {
        query(req, &[], 0)?;
        let agent = engagement_id(req)?;
        let input: hagency_store::OutcomeResolution =
            serde_json::from_slice(&body(req, 16 * 1024).await?).map_err(|_| Error::Invalid)?;
        Ok::<_, Error>((agent, input))
    }
    .await;
    let (agent, input) = match prepared {
        Ok(value) => value,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store.resolve_stopped_dispatch(agent, input).await;
    if result.is_ok() && recheck(depot).is_err() {
        failure(res, hagency_store::Error::OutcomeUnknown);
        return;
    }
    outcome_response(res, result);
}

#[handler]
async fn continue_stopped_dispatch(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if !check_lifecycle(depot, res) {
        return;
    }
    let prepared = async {
        query(req, &[], 0)?;
        let agent = engagement_id(req)?;
        let input: ContinueStoppedDispatch =
            serde_json::from_slice(&body(req, 16 * 1024).await?).map_err(|_| Error::Invalid)?;
        Ok::<_, Error>((agent, input))
    }
    .await;
    let (agent, input) = match prepared {
        Ok(value) => value,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store
        .continue_stopped_dispatch(
            agent,
            input.original,
            (input.fence, input.inspection_digest),
            input.replacement,
            input.evidence,
        )
        .await;
    if result.is_ok() && recheck(depot).is_err() {
        failure(res, hagency_store::Error::OutcomeUnknown);
        return;
    }
    match result {
        Ok(()) => res.render(Json(serde_json::json!({"ok":true}))),
        Err(hagency_store::Error::NotFound) => refusal(res, StatusCode::NOT_FOUND, "not_found"),
        Err(hagency_store::Error::Invalid(_)) => failed(res, Error::Invalid),
        Err(hagency_store::Error::Conflict) => {
            refusal(res, StatusCode::CONFLICT, "continuation_conflict")
        }
        Err(
            hagency_store::Error::State
            | hagency_store::Error::Quarantined
            | hagency_store::Error::RunnerAuthority
            | hagency_store::Error::Generation,
        ) => refusal(res, StatusCode::CONFLICT, "dispatch_not_continuable"),
        Err(error) => failure(res, error),
    }
}

/// Native engagements are provisioned against one immutable resource and
/// their budget, account binding, effect payload and running profile all
/// derive from it. Rebinding only a preset id would corrupt that invariant.
/// Refuse until a complete retire/reprovision transition owns every effect.
#[handler]
async fn preset(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if query(req, &[], 0).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    if let Err(error) = engagement_id(req) {
        failed(res, error);
        return;
    }
    if !check_lifecycle(depot, res) {
        return;
    }
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    refusal(res, StatusCode::NOT_IMPLEMENTED, "agent_preset_unavailable");
}

fn store_error(res: &mut Response, error: hagency_store::Error) {
    let (status, code) = match error {
        hagency_store::Error::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_roster_query"),
        hagency_store::Error::Busy => (StatusCode::SERVICE_UNAVAILABLE, "busy"),
        hagency_store::Error::OutcomeUnknown => (StatusCode::GATEWAY_TIMEOUT, "outcome_unknown"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "roster_unavailable"),
    };
    refusal(res, status, code);
}
