//! ADR-189: the console's setup page.
//!
//! `GET /console/api/setup` reports each step: the coding agents found on
//! this machine (path, version, whether and how they are signed in), whether
//! the runtime is configured, and the Palpo connection. `POST
//! /console/api/setup/check` detects again and, when a signed-in Codex is
//! found and no runtime is configured yet, writes and validates
//! `fleet-runtime.json` with `hagency setup`'s code. Hagency never signs a
//! coding agent in; the page asks the user to.
use super::engagements::check_lifecycle;
use super::{failed, recheck};
use crate::refusal;
use salvo::prelude::*;
use serde_json::json;

pub(super) fn router() -> Router {
    Router::with_path("setup")
        .get(status)
        .push(Router::with_path("check").post(check))
}

fn live(depot: &Depot) -> Option<crate::bootstrap::palpo::Live> {
    depot
        .get_typed::<crate::App>()
        .ok()
        .and_then(|app| app.palpo_live().cloned())
}

async fn report(
    live: &crate::bootstrap::palpo::Live,
    agents: Vec<crate::setup::AgentStatus>,
    configured_now: Option<Result<(), String>>,
) -> serde_json::Value {
    let runtime = live.state_dir().join("fleet-runtime.json").is_file();
    let mut value = json!({
        "ok": true,
        "agents": agents,
        "runtimeConfigured": runtime,
        "palpo": {
            "imported": live.is_imported(),
            "transport": live.status().get(),
        },
    });
    if let Some(result) = configured_now {
        value["configured"] = json!(result.is_ok());
        if let Err(problem) = result {
            value["problem"] = json!(problem);
        }
    }
    value
}

#[handler]
async fn status(depot: &mut Depot, res: &mut Response) {
    let Some(live) = live(depot) else {
        refusal(res, StatusCode::SERVICE_UNAVAILABLE, "setup_unavailable");
        return;
    };
    let agents = vec![crate::setup::detect_codex().await];
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    res.render(Json(report(&live, agents, None).await));
}

#[handler]
async fn check(depot: &mut Depot, res: &mut Response) {
    // Writing the runtime configuration is a lifecycle write, like a Palpo
    // import.
    if !check_lifecycle(depot, res) {
        return;
    }
    let Some(live) = live(depot) else {
        refusal(res, StatusCode::SERVICE_UNAVAILABLE, "setup_unavailable");
        return;
    };
    let Some(address) = live.fleet_address() else {
        // A coordinator install configures its runtime through agent-driver.json.
        refusal(res, StatusCode::CONFLICT, "setup_not_fleet");
        return;
    };
    let codex = crate::setup::detect_codex().await;
    let state = live.state_dir().to_owned();
    let configured_now =
        if !state.join("fleet-runtime.json").is_file() && codex.found && codex.signed_in {
            let options = crate::setup::Options {
                state_dir: state,
                listen: address,
                codex: codex.path.clone(),
                codex_home: None,
                no_local_codex: false,
                force: false,
            };
            let result = tokio::task::spawn_blocking(move || crate::setup::configure(&options))
                .await
                .map_err(|_| "configuration did not finish".to_owned())
                .and_then(|r| r.map(|_| ()));
            Some(result)
        } else {
            None
        };
    if let Err(error) = recheck(depot) {
        failed(res, error);
        return;
    }
    res.render(Json(report(&live, vec![codex], configured_now).await));
}
