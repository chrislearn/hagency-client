//! Authenticated, homeserver-local room-name search, with cached joined rooms.
use matrix_sdk::{Client, ruma::{OwnedRoomId, api::client::directory::get_public_rooms_filtered}};

#[derive(Clone, Debug)]
pub(super) struct DirectoryRoom {
    pub id: OwnedRoomId,
    pub name: String,
    pub description: String,
}

#[derive(Debug)]
pub(super) struct DirectoryPage {
    pub rooms: Vec<DirectoryRoom>,
    pub next_batch: Option<String>,
    pub warning: Option<String>,
}

pub(super) async fn search(client: &Client, query: &str, since: Option<String>) -> DirectoryPage {
    let mut rooms = Vec::new();
    if since.is_none() {
        let needle = query.to_lowercase();
        for room in client.joined_rooms() {
            if crate::moments::is_moments(&room) {continue;}
            let name = room.name().unwrap_or_else(|| room.room_id().to_string());
            let alias = room.canonical_alias();
            if name.to_lowercase().contains(&needle)
                || alias.as_ref().is_some_and(|alias| alias.as_str().to_lowercase().contains(&needle)) {
                rooms.push(DirectoryRoom { id: room.room_id().to_owned(), name, description: crate::i18n::tr("Joined").into() });
            }
        }
        rooms.sort_by_key(|room| (room.name.to_lowercase(), room.id.clone()));
    }
    let mut request = get_public_rooms_filtered::v3::Request::new();
    request.filter.generic_search_term = Some(query.to_owned());
    request.limit = Some(50u32.into());
    request.since = since;
    // Leaving `server` unset searches the signed-in account's homeserver, even
    // when it is not matrix.org. Private/unlisted rooms need an address/invite.
    match client.public_rooms_filtered(request).await {
        Ok(response) => {
            for room in response.chunk {
                if rooms.iter().any(|known| known.id == room.room_id) { continue; }
                let name = room.name.or_else(|| room.canonical_alias.as_ref().map(ToString::to_string))
                    .unwrap_or_else(|| room.room_id.to_string());
                rooms.push(DirectoryRoom {
                    id: room.room_id,
                    name,
                    description: format!("{} {}{}", room.num_joined_members,
                        if u64::from(room.num_joined_members) == 1 { crate::i18n::tr("member") } else { crate::i18n::tr("members") },
                        room.canonical_alias.map(|alias| format!(" · {alias}")).unwrap_or_default()),
                });
            }
            DirectoryPage { rooms, next_batch: response.next_batch, warning: None }
        }
        Err(error) => DirectoryPage {
            rooms, next_batch: None,
            warning: Some(crate::i18n::format("Could not search the public directory: {error}. You can still open joined results or enter a room address.", &[("error", (error).to_string())])),
        },
    }
}
