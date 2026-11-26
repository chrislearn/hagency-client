//! Shared navigation order lives in the Project Space. Separate state keys let
//! members update Boards and Rooms independently. Only opaque ID hashes are
//! published, since not every Space member can discover every private Room.
use super::backend::Service;
use crate::{
    matrix_context,
    sliding_sync::{get_client, spawn_async_task},
};
use makepad_widgets::{Actions, Cx};
use matrix_sdk::{
    Client,
    ruma::{
        OwnedRoomId,
        api::{
            client::state::{get_state_event_for_key, send_state_event},
            error::ErrorKind,
        },
        serde::Raw,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

const EVENT_TYPE: &str = "org.hagency.project.navigation_order";
const REFRESH_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OrderKind {
    Boards,
    Rooms,
}

impl OrderKind {
    fn key(self) -> &'static str {
        match self {
            Self::Boards => "boards",
            Self::Rooms => "rooms",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProjectOrder {
    boards: Vec<String>,
    rooms: Vec<String>,
}

impl ProjectOrder {
    fn ids(&self, kind: OrderKind) -> &[String] {
        match kind {
            OrderKind::Boards => &self.boards,
            OrderKind::Rooms => &self.rooms,
        }
    }

    pub(crate) fn sort_by_id<T>(&self, kind: OrderKind, rows: &mut [T], id: impl Fn(&T) -> &str) {
        let order = self.ids(kind);
        if order.is_empty() {
            return;
        }
        let ranks: HashMap<_, _> = order
            .iter()
            .enumerate()
            .map(|(rank, key)| (key.as_str(), rank))
            .collect();
        rows.sort_by_cached_key(|row| {
            ranks
                .get(blake3::hash(id(row).as_bytes()).to_hex().as_str())
                .copied()
                .unwrap_or(usize::MAX)
        });
    }

    pub(crate) fn cached(cx: &Cx, identity: &str, project: &str) -> Self {
        entry(cx, identity, project)
            .map(|e| e.order.clone())
            .unwrap_or_default()
    }

    pub(crate) fn can_edit(cx: &Cx, identity: &str, project: &str) -> bool {
        entry(cx, identity, project).is_some_and(|e| e.loaded && !e.writing && !e.failed)
    }

    pub(crate) fn load_failed(cx: &Cx, identity: &str, project: &str) -> bool {
        entry(cx, identity, project).is_some_and(|e| e.failed)
    }

    pub(crate) fn waiting_for_initial(cx: &Cx, identity: &str, project: &str) -> bool {
        entry(cx, identity, project).is_some_and(|e| !e.loaded && e.pending)
    }
}

#[derive(Default)]
struct Cache {
    identity: String,
    projects: HashMap<String, Entry>,
    serial: u64,
}

#[derive(Default)]
struct Entry {
    order: ProjectOrder,
    space: String,
    request: u64,
    last: Option<Instant>,
    loaded: bool,
    pending: bool,
    writing: bool,
    failed: bool,
}

fn entry<'a>(cx: &'a Cx, identity: &str, project: &str) -> Option<&'a Entry> {
    let cache = cx.get_global_ref::<Cache>()?;
    (cache.identity == identity)
        .then(|| cache.projects.get(project))
        .flatten()
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectOrderChanged {
    pub identity: String,
    pub project: String,
    pub error: Option<&'static str>,
}

#[derive(Clone, Debug)]
struct Completed {
    identity: String,
    project: String,
    request: u64,
    order: Result<ProjectOrder, ()>,
    write_error: Option<&'static str>,
}

#[derive(Serialize, Deserialize)]
struct StateContent {
    #[serde(default)]
    ids: Vec<String>,
}

async fn read_list(
    client: &Client,
    space: &OwnedRoomId,
    kind: OrderKind,
) -> Result<Vec<String>, ()> {
    let request = get_state_event_for_key::v3::Request::new(
        space.clone(),
        EVENT_TYPE.into(),
        kind.key().into(),
    );
    match client.send(request).await {
        Ok(response) => serde_json::from_str::<StateContent>(response.into_content().json().get())
            .map(|c| c.ids)
            .map_err(|_| ()),
        Err(error) if error.client_api_error_kind() == Some(&ErrorKind::NotFound) => Ok(Vec::new()),
        Err(_) => Err(()),
    }
}

pub(crate) fn refresh(cx: &mut Cx, identity: &str, project: &str, space: &str) {
    fetch(cx, identity, project, space, false);
}

pub(super) fn fetch(cx: &mut Cx, identity: &str, project: &str, space: &str, force: bool) {
    let Ok(space_id) = OwnedRoomId::try_from(space) else {
        return;
    };
    if let Some(e) = entry(cx, identity, project)
        && e.space == space
        && (e.pending || (!force && e.last.is_some_and(|last| last.elapsed() < REFRESH_INTERVAL)))
    {
        return;
    }
    let Some(client) = get_client() else { return };
    if !Service::for_current().is_ok_and(|service| service.identity() == identity) {
        return;
    }
    let cache = cx.global::<Cache>();
    if cache.identity != identity {
        cache.projects.clear();
        cache.identity = identity.to_owned();
    }
    cache.serial += 1;
    let request = cache.serial;
    let e = cache.projects.entry(project.into()).or_default();
    if e.space != space {
        *e = Entry {
            space: space.into(),
            ..Entry::default()
        };
    }
    e.request = request;
    e.pending = true;
    let identity = identity.to_owned();
    let project = project.to_owned();
    spawn_async_task(async move {
        let result = tokio::try_join!(
            read_list(&client, &space_id, OrderKind::Boards),
            read_list(&client, &space_id, OrderKind::Rooms)
        );
        Cx::post_action(Completed {
            identity,
            project,
            request,
            order: result.map(|(boards, rooms)| ProjectOrder { boards, rooms }),
            write_error: None,
        });
    });
}

// Replace the visible items in place. A Room hidden from this editor retains
// its slot, and newly discovered items can be positioned on the first drag.
fn merge_visible(previous: &[String], visible: &[String]) -> Vec<String> {
    let included: HashSet<_> = visible.iter().collect();
    let mut replacement = visible.iter();
    let mut result = Vec::new();
    for item in previous {
        if !included.contains(item) {
            result.push(item.clone());
        } else if let Some(next) = replacement.next() {
            result.push(next.clone());
        }
    }
    result.extend(replacement.cloned());
    result
}

const SAVE_ERROR: &str = "Could not save the project order. Try again.";
const CHANGED: &str = "The project order changed. Drag again to apply your change.";

async fn update_list(
    client: &Client,
    space: &OwnedRoomId,
    expected: &[String],
    kind: OrderKind,
    visible: &[String],
    current_session: impl Fn() -> bool,
) -> Result<(Vec<String>, Option<&'static str>), ()> {
    let current = read_list(client, space, kind).await?;
    if current != expected {
        return Ok((current, Some(CHANGED)));
    }
    if !current_session() {
        return Err(());
    }
    let hashes: Vec<_> = visible
        .iter()
        .map(|id| blake3::hash(id.as_bytes()).to_hex().to_string())
        .collect();
    let content = StateContent {
        ids: merge_visible(&current, &hashes),
    };
    let request = send_state_event::v3::Request::new_raw(
        space.clone(),
        EVENT_TYPE.into(),
        kind.key().into(),
        Raw::new(&content).map_err(|_| ())?.cast_unchecked(),
    );
    client.send(request).await.map_err(|_| ())?;
    Ok((content.ids, None))
}

pub(super) fn save(
    cx: &mut Cx,
    identity: &str,
    project: &str,
    space: &str,
    kind: OrderKind,
    visible: Vec<String>,
) -> bool {
    if !ProjectOrder::can_edit(cx, identity, project)
        || !Service::for_current().is_ok_and(|s| s.identity() == identity)
    {
        return false;
    }
    let Some(client) = get_client() else {
        return false;
    };
    let Ok(space_id) = OwnedRoomId::try_from(space) else {
        return false;
    };
    let cache = cx.global::<Cache>();
    let Some(e) = cache.projects.get_mut(project).filter(|e| e.space == space) else {
        return false;
    };
    cache.serial += 1;
    let request = cache.serial;
    e.request = request;
    e.pending = true;
    e.writing = true;
    let mut order = e.order.clone();
    let identity = identity.to_owned();
    let project = project.to_owned();
    spawn_async_task(async move {
        let list = match kind {
            OrderKind::Boards => &mut order.boards,
            OrderKind::Rooms => &mut order.rooms,
        };
        let (result, error) = match update_list(&client, &space_id, list, kind, &visible, || {
            matrix_context::is_current(&client)
        })
        .await
        {
            Ok((updated, warning)) => {
                *list = updated;
                (Ok(order), warning)
            }
            Err(()) => (Err(()), Some(SAVE_ERROR)),
        };
        Cx::post_action(Completed {
            identity,
            project,
            request,
            order: result,
            write_error: error,
        });
    });
    true
}

pub(crate) fn handle_actions(cx: &mut Cx, actions: &Actions) {
    for done in actions.iter().filter_map(|a| a.downcast_ref::<Completed>()) {
        let cache = cx.global::<Cache>();
        if done.identity != cache.identity {
            continue;
        }
        let Some(e) = cache
            .projects
            .get_mut(&done.project)
            .filter(|e| e.request == done.request)
        else {
            continue;
        };
        let load_failed = done.order.is_err() && !e.writing;
        let notify = !e.loaded
            || e.writing
            || e.failed != load_failed
            || done.order.as_ref().is_ok_and(|order| order != &e.order);
        e.pending = false;
        e.writing = false;
        e.failed = load_failed;
        e.last = Some(Instant::now());
        if let Ok(order) = &done.order {
            e.order = order.clone();
            e.loaded = true;
        }
        if notify {
            cx.action(ProjectOrderChanged {
                identity: done.identity.clone(),
                project: done.project.clone(),
                error: done.write_error,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn project_space_stores_independent_orders_and_detects_stale_edits() {
        use matrix_sdk::{ruma::room_id, test_utils::mocks::MatrixMockServer};
        use serde_json::json;
        use wiremock::{
            Mock, ResponseTemplate,
            matchers::{method, path_regex, body_json},
        };

        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let space = room_id!("!project:example.org");

        let path = r"/state/org\.hagency\.project\.navigation_order/";
        let visible = ["two".to_owned(), "one".to_owned()];
        let expected: Vec<_> = visible
            .iter()
            .map(|s| blake3::hash(s.as_bytes()).to_hex().to_string())
            .collect();
        let missing = ResponseTemplate::new(404)
            .set_body_json(json!({"errcode":"M_NOT_FOUND","error":"No state"}));
        let first_read = Mock::given(method("GET"))
            .and(path_regex(format!("{path}boards$")))
            .respond_with(missing.clone())
            .expect(1)
            .mount_as_scoped(server.server())
            .await;
        Mock::given(method("PUT"))
            .and(path_regex(format!("{path}boards$")))
            .and(body_json(json!({"ids":expected})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"event_id":"$ordered"})))
            .expect(1)
            .mount(server.server())
            .await;
        assert_eq!(
            update_list(
                &client,
                &space.to_owned(),
                &[],
                OrderKind::Boards,
                &visible,
                || true
            )
            .await
            .unwrap(),
            (expected.clone(), None)
        );
        drop(first_read);
        Mock::given(method("GET"))
            .and(path_regex(format!("{path}boards$")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ids":expected})))
            .mount(server.server())
            .await;
        Mock::given(method("GET"))
            .and(path_regex(format!("{path}rooms$")))
            .respond_with(missing)
            .mount(server.server())
            .await;

        let other_client = server.client_builder().build().await;
        assert_eq!(
            read_list(&other_client, &space.to_owned(), OrderKind::Boards)
                .await
                .unwrap(),
            expected
        );
        assert!(
            read_list(&other_client, &space.to_owned(), OrderKind::Rooms)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            update_list(
                &client,
                &space.to_owned(),
                &[],
                OrderKind::Boards,
                &visible,
                || true
            )
            .await
            .unwrap(),
            (expected, Some(CHANGED))
        );
    }

    #[test]
    fn reorder_preserves_hidden_entries_and_appends_unranked_items_stably() {
        let old = ["a", "hidden", "b"].map(String::from);
        let wanted = ["b", "new", "a"].map(String::from);
        assert_eq!(merge_visible(&old, &wanted), ["b", "hidden", "new", "a"]);
        let order = ProjectOrder {
            boards: wanted
                .iter()
                .map(|s| blake3::hash(s.as_bytes()).to_hex().to_string())
                .collect(),
            ..Default::default()
        };
        let mut rows = ["a", "x", "b", "y", "new"];
        order.sort_by_id(OrderKind::Boards, &mut rows, |s| *s);
        assert_eq!(rows, ["b", "new", "a", "x", "y"]);
    }
}

#[cfg(test)]
pub(crate) fn seed_for_test(
    cx: &mut Cx,
    identity: &str,
    project: &str,
    boards: &[&str],
    rooms: &[&str],
) {
    let hashes = |items: &[&str]| {
        items
            .iter()
            .map(|id| blake3::hash(id.as_bytes()).to_hex().to_string())
            .collect()
    };
    let cache = cx.global::<Cache>();
    cache.identity = identity.into();
    cache.projects.insert(
        project.into(),
        Entry {
            order: ProjectOrder {
                boards: hashes(boards),
                rooms: hashes(rooms),
            },
            loaded: true,
            ..Default::default()
        },
    );
}
