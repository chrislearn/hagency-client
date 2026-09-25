//! The read-only approval-bindings list (board #52, TS `GET
//! /api/approval-bindings` at backend-v2.js:9051-9082, the plain-list
//! branch): every LIVE binding, derived from the store's own
//! observation-driven binding tables through `current_approval_bindings` —
//! never a second writer. Mounted under the API sub-router's `authenticate`
//! hoop with NO scope, the same read class as the roster and the approvals
//! list (ADR-126): scope facts are a payload on reads, never a gate.
//!
//! The wire item carries exactly nine camelCase keys — `engagementId`,
//! `agent`, `fleetId`, `projectId`, `serverName`, `roomId`, `ownerMxid`,
//! `roomGeneration`, `incarnation`. The TS write-side fields
//! (`agentJoined`, `membershipCheckedAt`, `active`, `authorityId`) have no
//! native source: native derives binding liveness from room observations
//! (`observe_approval_room` + the `current_approval_bindings` view), so a
//! membership status an operator could set would assert a fact nothing
//! observes. They are withheld, never invented — the same discipline the
//! project-sides read applies to its `unavailable` columns.
//!
//! The TS write half (PUT bind, PUT membership, DELETE unbind) is not
//! ported: native's binding rows are derived facts, and asserting or
//! removing one by hand would create a row no room observation backs. The
//! TS unbind-revokes semantics already exist natively at the right moment —
//! the `approval_room_retire_grants` trigger revokes every grant of a
/// binding's engagement the instant its room observation goes unsafe.
use super::{Error, Session, console, failed, recheck, usage::query};
use crate::{refusal, resources::domain};
use hagency_core::project::identifier;
use salvo::prelude::*;

pub(super) fn router() -> Router {
    Router::with_path("approval-bindings")
        .get(list)
        .push(Router::with_path("{agent}/{room}").delete(unbind))
}

fn store_error(res: &mut Response, error: hagency_store::Error) {
    let (status, code) = match error {
        hagency_store::Error::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_bindings_query"),
        hagency_store::Error::Capacity => (StatusCode::BAD_REQUEST, "invalid_bindings_query"),
        hagency_store::Error::Busy => (StatusCode::SERVICE_UNAVAILABLE, "busy"),
        hagency_store::Error::OutcomeUnknown => (StatusCode::GATEWAY_TIMEOUT, "outcome_unknown"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "bindings_unavailable"),
    };
    refusal(res, status, code);
}

#[handler]
async fn list(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    // The list takes the TS route's two filters (`agent`, `project`) plus
    // the native `limit` bound — anything else is refused, the same hygiene
    // the approvals list applies to its own allowlist.
    if query(req, &["agent", "project", "limit"], 192).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    let agent = req.query::<String>("agent").unwrap_or_default();
    let project_room_id = req.query::<String>("project").unwrap_or_default();
    let limit = match req.query::<String>("limit") {
        None => 16,
        Some(v) if v.bytes().all(|c| c.is_ascii_digit()) => v.parse::<u64>().unwrap_or(0),
        _ => 0,
    };
    // The TS list route passes a room id in the `project` slot
    // (`listBindings({ agent, project })` matches `projectRoomId`), so the
    // room shape — not a project identifier — is what is validated here.
    // The `project` filter carries a room id whose server suffix is its own
    // evidence (the route cannot know the fleet's server name a priori), so
    // the shape check passes the value's own suffix as the expected server.
    let room_valid = project_room_id.is_empty()
        || project_room_id
            .split_once(':')
            .is_some_and(|(_, suffix)| {
                hagency_core::replies::matrix_room(&project_room_id, suffix).is_ok()
            });
    if !(1..=100).contains(&limit)
        || (!agent.is_empty() && identifier(&agent, 128).is_err())
        || !room_valid
    {
        failed(res, Error::Invalid);
        return;
    }
    // The capability word, as the roster and alerts reads carry theirs: the
    // page renders the unbind control ONLY from this boolean, so a read-only
    // ticket is never shown a button whose only possible answer is a refusal
    // ("buttons that would 404 lie", NativeApprovals' own rule).
    let manage_bindings = match depot.get_typed::<Session>() {
        Ok(session) => match console(depot).and_then(|c| c.0.authority.can_lifecycle(session)) {
            Ok(value) => value,
            Err(error) => {
                failed(res, error);
                return;
            }
        },
        Err(_) => {
            failed(res, Error::Unauthorized);
            return;
        }
    };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store.approval_bindings(agent, project_room_id, limit).await;
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    match result {
        Ok(bindings) => {
            // Statement time, as the roster and approvals reads do: the list
            // has no clock parameter to honor.
            let at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| u64::try_from(d.as_millis()).ok())
                .unwrap_or_default();
            res.render(Json(serde_json::json!({
                "at_ms": at_ms,
                "bindings": bindings,
                "permissions": {"manageBindings": manage_bindings},
            })));
        }
        Err(error) => store_error(res, error),
    }
}

/// The operator unbind (board #52, TS `DELETE
/// /api/approval-bindings/:agent/:roomId` at backend-v2.js:9030-9046): a
/// MUTATION, so it is gated on `Scope::AgentLifecycle` — the console's
/// scope for reviewed lifecycle store mutations — exactly as the approvals
/// grant-revocation DELETE is; a read-only ticket is refused with
/// `agent_lifecycle_scope_required` before any store work. The store
/// removes the derived binding row and revokes every live grant it carried
/// in ONE transaction, which is the TS effect (`removeBinding` then
/// `revokeScopesByBinding`): no saved authority outlives its binding. An
/// unknown pair is a 404, as TS returns.
#[handler]
async fn unbind(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if req.uri().query().is_some() {
        failed(res, Error::Invalid);
        return;
    }
    let session = match depot.get_typed::<Session>() {
        Ok(session) => session,
        Err(_) => {
            failed(res, Error::Unauthorized);
            return;
        }
    };
    match console(depot).and_then(|c| c.0.authority.can_lifecycle(session)) {
        Ok(true) => {}
        Ok(false) => {
            failed(res, Error::LifecycleForbidden);
            return;
        }
        Err(error) => {
            failed(res, error);
            return;
        }
    }
    let prepared = (|| {
        let agent = req.param::<String>("agent").ok_or(Error::Invalid)?;
        identifier(&agent, 128).map_err(|_| Error::Invalid)?;
        // The room is a path segment, so it is percent-decoded by Salvo
        // before it reaches here; its own suffix is its server (the route
        // cannot know the fleet's server name a priori).
        let room = req.param::<String>("room").ok_or(Error::Invalid)?;
        let (_, suffix) = room.split_once(':').ok_or(Error::Invalid)?;
        hagency_core::replies::matrix_room(&room, suffix).map_err(|_| Error::Invalid)?;
        Ok::<_, Error>((agent, room))
    })();
    let (agent, room) = match prepared {
        Ok(value) => value,
        Err(error) => {
            failed(res, error);
            return;
        }
    };
    let Some(store) = domain(depot, res) else {
        return;
    };
    let result = store.retire_approval_binding(agent, room).await;
    if result.is_ok() && recheck(depot).is_err() {
        failed(res, Error::Invalid);
        return;
    }
    match result {
        Ok(binding) => res.render(Json(serde_json::json!({
            "ok": true,
            "binding": binding,
        }))),
        Err(hagency_store::Error::NotFound) => {
            refusal(res, StatusCode::NOT_FOUND, "approval_binding_not_found")
        }
        Err(error) => store_error(res, error),
    }
}
