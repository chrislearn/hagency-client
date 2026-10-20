//! Account/device-bound encrypted history storage. Only call from blocking workers.
use std::{io::Read, path::PathBuf};

use anyhow::{Result, anyhow, ensure};
use hagency_history::{Event, Index};
use matrix_sdk::{Client, deserialized_responses::TimelineEvent};
use ruma::RoomId;
use zeroize::Zeroizing;

static FAILED_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

#[derive(Debug)]
pub(crate) struct HistoryStoreChanged {
    pub room: Option<ruma::OwnedRoomId>,
    pub epoch: u64,
}

pub(crate) fn ensure_healthy() -> Result<()> {
    ensure!(
        FAILED_EPOCH.load(std::sync::atomic::Ordering::Acquire) != crate::account_session::epoch(),
        "History index updates failed; reconnect before searching"
    );
    Ok(())
}

pub(crate) fn open(client: &Client, room: &RoomId) -> Result<Index> {
    ensure_healthy()?;
    crate::matrix_context::ensure_current(client)?;
    let user = client.user_id().ok_or_else(|| anyhow!("Not signed in"))?;
    let device = client
        .device_id()
        .ok_or_else(|| anyhow!("No device identity"))?;
    let file = std::fs::File::open(crate::persistence::session_file_path(user))?;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 1024 * 1024, "Session file too large");
    let mut session: crate::persistence::FullSessionPersisted = serde_json::from_slice(&bytes)?;
    ensure!(
        session.user_session.meta().user_id == user
            && session.user_session.meta().device_id == device,
        "History index session identity mismatch"
    );
    let homeserver: url::Url = session.client_session.homeserver.parse()?;
    ensure!(
        homeserver == client.homeserver(),
        "History index homeserver mismatch"
    );
    let identity = serde_json::to_string(&(
        client.homeserver().as_str(),
        user.as_str(),
        device.as_str(),
        room.as_str(),
    ))?;
    let secret = Zeroizing::new(std::mem::take(&mut session.client_session.passphrase));
    ensure!(!secret.is_empty(), "History index secret missing");
    let root_key = Zeroizing::new(blake3::derive_key(
        "hagency.history.root.v1",
        secret.as_bytes(),
    ));
    let wrapping_key =
        Zeroizing::new(*blake3::keyed_hash(&root_key, identity.as_bytes()).as_bytes());
    let path = path(client, room)?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    Index::open(&path, &wrapping_key, &identity)
}

fn directory(client: &Client) -> Result<PathBuf> {
    let user = client.user_id().ok_or_else(|| anyhow!("Not signed in"))?;
    let device = client
        .device_id()
        .ok_or_else(|| anyhow!("No device identity"))?;
    let device = blake3::hash(device.as_bytes()).to_hex().to_string();
    Ok(crate::persistence::persistent_state_dir(user)
        .join("history-search-v1")
        .join(device))
}

fn path(client: &Client, room: &RoomId) -> Result<PathBuf> {
    let room = blake3::hash(room.as_bytes()).to_hex().to_string();
    Ok(directory(client)?.join(format!("{room}.sqlite3")))
}

pub(crate) fn event(event: &TimelineEvent, name: Option<&str>) -> Result<Option<Event>> {
    let value: serde_json::Value = serde_json::from_str(event.kind.raw().json().get())?;
    if value["event_id"]
        .as_str()
        .and_then(|id| ruma::OwnedEventId::try_from(id).ok())
        .is_none()
    {
        return Ok(None);
    }
    if !event.kind.is_utd()
        && value["type"] == "m.room.message"
        && value.pointer("/unsigned/redacted_because").is_none()
    {
        let body = if value
            .pointer("/content/m.relates_to/rel_type")
            .and_then(serde_json::Value::as_str)
            == Some("m.replace")
        {
            &value["content"]["m.new_content"]
        } else {
            &value["content"]
        };
        if serde_json::from_value::<ruma::events::room::message::RoomMessageEventContent>(
            body.clone(),
        )
        .is_err()
            || value["sender"]
                .as_str()
                .and_then(|id| ruma::OwnedUserId::try_from(id).ok())
                .is_none()
        {
            return Ok(None);
        }
    }
    let name = name
        .map(str::to_owned)
        .or_else(|| value["sender"].as_str().map(str::to_owned))
        .unwrap_or_default();
    Ok(Some(Event {
        value,
        name,
        locked: event.kind.is_utd(),
    }))
}

/// A single account-scoped consumer; no unbounded task/connection per room.
/// Lagging never silently leaves searchable stale edits/redactions behind.
pub(crate) fn start(client: Client) {
    FAILED_EPOCH.store(u64::MAX, std::sync::atomic::Ordering::Release);
    start_decryption_updates(client.clone());
    let mut updates = client.subscribe_to_all_room_updates();
    let epoch = crate::account_session::epoch();
    crate::account_session::spawn(async move {
        loop {
            let update = updates.recv().await;
            if !crate::matrix_context::is_current(&client) {
                break;
            }
            let rooms = match update {
                Ok(update) => update
                    .joined
                    .into_keys()
                    .chain(update.left.into_keys())
                    .collect::<std::collections::BTreeSet<_>>(),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => client
                    .joined_rooms()
                    .into_iter()
                    .map(|room| room.room_id().to_owned())
                    .collect(),
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            for room_id in rooms {
                let Ok(file) = path(&client, &room_id) else {
                    continue;
                };
                if !file.exists() {
                    continue;
                }
                let Some(room) = client.get_room(&room_id) else {
                    continue;
                };
                // SDK broadcasts are wakeups only. All message mutations come
                // from the index's own ordered, persisted /messages cursor.
                // A newer SDK sync token can never skip an unindexed redaction.
                let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
                let repair = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let result =
                    crate::home::room_history::catch_up_history(&client, &room, &slot, &repair)
                        .await;
                if !crate::matrix_context::is_current(&client) {
                    return;
                }
                if result.is_err() {
                    let marked =
                        tokio::task::spawn_blocking(move || Index::mark_stale(&file)).await;
                    if !crate::matrix_context::is_current(&client) {
                        return;
                    }
                    if !matches!(marked, Ok(Ok(()))) {
                        FAILED_EPOCH.store(epoch, std::sync::atomic::Ordering::Release);
                        makepad_widgets::SignalToUI::set_ui_signal();
                        return;
                    }
                }
                makepad_widgets::Cx::post_action(HistoryStoreChanged {
                    room: Some(room_id),
                    epoch,
                });
            }
        }
    });
}

static DECRYPTION_GATE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

async fn retry_locked_page(
    client: &Client,
    room: &matrix_sdk::Room,
    session: Option<&str>,
    after: Option<String>,
) -> Result<(Option<String>, std::collections::BTreeSet<String>)> {
    crate::matrix_context::ensure_current(client)?;
    let worker_client = client.clone();
    let room_id = room.room_id().to_owned();
    let session = session.map(str::to_owned);
    let (events, generation) = tokio::task::spawn_blocking(move || -> Result<_> {
        let index = open(&worker_client, &room_id)?;
        Ok((
            index.locked_page(session.as_deref(), after.as_deref())?,
            index.generation()?,
        ))
    })
    .await??;
    let last = events
        .last()
        .and_then(|e| e.value["event_id"].as_str())
        .map(str::to_owned);
    let mut decrypted = Vec::new();
    let mut sessions = std::collections::BTreeSet::new();
    for event in events {
        let Ok(raw) = ruma::serde::Raw::new(&event.value) else {
            continue;
        };
        if let Ok(resolved) = room.decrypt_event(&raw.cast_unchecked(), None).await
            && !resolved.kind.is_utd()
        {
            if let Some(session) = resolved.kind.session_id() {
                sessions.insert(session.to_owned());
            }
            if let Some(event) = self::event(&resolved, Some(&event.name))? {
                decrypted.push(event);
            }
        }
    }
    if !decrypted.is_empty() {
        crate::matrix_context::ensure_current(client)?;
        let worker_client = client.clone();
        let room_id = room.room_id().to_owned();
        tokio::task::spawn_blocking(move || -> Result<()> {
            open(&worker_client, &room_id)?.insert_for_generation(&decrypted, None, &generation)
        })
        .await??;
        crate::matrix_context::ensure_current(client)?;
        makepad_widgets::Cx::post_action(HistoryStoreChanged {
            room: Some(room.room_id().to_owned()),
            epoch: crate::account_session::epoch(),
        });
    }
    Ok((last, sessions))
}

async fn retry_sessions(
    client: &Client,
    room: &matrix_sdk::Room,
    sessions: std::collections::BTreeSet<String>,
) -> Result<()> {
    for session in sessions {
        let mut after = None;
        loop {
            let (next, _) = retry_locked_page(client, room, Some(&session), after).await?;
            if next.is_none() {
                break;
            }
            after = next;
        }
    }
    Ok(())
}

/// A cheap bounded probe on open. Resolved sessions continue in one shared
/// worker; scanning historical UTDs never holds up a search or creates N tasks.
pub(crate) fn retry_locked_on_open(client: Client, room: matrix_sdk::Room) {
    let Ok(permit) = DECRYPTION_GATE.try_acquire() else {
        return;
    };
    crate::account_session::spawn(async move {
        let _permit = permit;
        if let Ok((_, sessions)) = retry_locked_page(&client, &room, None, None).await {
            let _ = retry_sessions(&client, &room, sessions).await;
        }
    });
}

fn start_decryption_updates(client: Client) {
    use futures_util::StreamExt;
    crate::account_session::spawn(async move {
        let cache = client.event_cache();
        let reports = cache.subscribe_to_decryption_reports();
        futures_util::pin_mut!(reports);
        while let Some(report) = reports.next().await {
            if !crate::matrix_context::is_current(&client) {
                break;
            }
            let Ok(matrix_sdk::event_cache::RedecryptorReport::ResolvedUtds { room_id, events }) =
                report
            else {
                continue;
            };
            let Some(room) = client.get_room(&room_id) else {
                continue;
            };
            let Ok(file) = path(&client, &room_id) else {
                continue;
            };
            if !file.exists() {
                continue;
            }
            let Ok(_permit) = DECRYPTION_GATE.acquire().await else {
                break;
            };
            let worker_client = client.clone();
            let sessions = tokio::task::spawn_blocking(move || -> Result<_> {
                let index = open(&worker_client, &room_id)?;
                let mut sessions = std::collections::BTreeSet::new();
                for id in events {
                    if let Some(session) = index.locked_session(id.as_str())? {
                        sessions.insert(session);
                    }
                }
                Ok(sessions)
            })
            .await;
            if let Ok(Ok(sessions)) = sessions {
                let _ = retry_sessions(&client, &room, sessions).await;
            }
        }
    });
}
