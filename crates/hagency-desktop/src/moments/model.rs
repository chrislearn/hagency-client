//! Pure interpretation rules; Matrix membership remains the access boundary.
use std::collections::{BTreeMap, BTreeSet};
use ruma::{OwnedEventId, OwnedRoomId, OwnedUserId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const LIKE: &str = "❤️";
pub const MAX_MEDIA: usize = 9;
pub const MAX_TEXT: usize = 12_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub mimetype: String,
    pub size: u64,
    pub file: ruma::events::room::EncryptedFile,
}

pub fn post_content(body: &str, media: &[Asset]) -> Value {
    let mut value = json!({"msgtype":"m.text", "body":body,
        super::ROOM_TYPE:{"version":1,"media":media}});
    // Ordinary clients can show the first asset; Rinx uses the ordered album.
    if let Some(asset) = media.first() {
        value["msgtype"] = json!(if asset.mimetype.starts_with("video/") {
            "m.video"
        } else {
            "m.image"
        });
        value["file"] = json!(asset.file);
        value["filename"] = json!(asset.name);
        value["info"] = json!({"mimetype":asset.mimetype,"size":asset.size});
        if body.is_empty() {
            value["body"] = json!(asset.name);
        }
    }
    value
}

pub fn comment_content(body: &str, post: &ruma::EventId) -> Value {
    json!({"msgtype":"m.text","body":body,"m.relates_to":{
        "rel_type":"m.thread","event_id":post,"is_falling_back":true,
        "m.in_reply_to":{"event_id":post}}})
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub room: OwnedRoomId,
    pub id: OwnedEventId,
    pub sender: OwnedUserId,
    pub timestamp: u64,
    pub content: Value,
    pub edited: bool,
}
impl Entry {
    pub fn body(&self) -> &str {
        self.content["body"].as_str().unwrap_or("")
    }
    pub fn media(&self) -> Vec<Asset> {
        serde_json::from_value::<Vec<Asset>>(self.content[super::ROOM_TYPE]["media"].clone())
            .unwrap_or_default()
            .into_iter()
            .take(MAX_MEDIA)
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Index {
    // Per-room indexes prevent forged cross-room relationships. Re-inserting an
    // event allows decryption to recover when missing keys subsequently arrive.
    events: BTreeMap<OwnedEventId, Value>,
    redacted: BTreeSet<OwnedEventId>,
}
impl Index {
    pub fn contains(&self, id: &ruma::EventId) -> bool {
        self.events.contains_key(id)
    }
    pub fn unavailable_ids(&self) -> Vec<OwnedEventId> {
        self.events
            .iter()
            .filter(|(_, e)| e["type"] == "m.room.encrypted")
            .map(|(id, _)| id.clone())
            .collect()
    }
    pub fn insert(&mut self, room: &ruma::RoomId, value: Value) {
        if value
            .get("room_id")
            .and_then(Value::as_str)
            .is_some_and(|r| r != room.as_str())
        {
            return;
        }
        let Some(id) = value["event_id"]
            .as_str()
            .and_then(|s| OwnedEventId::try_from(s).ok())
        else {
            return;
        };
        if value.pointer("/unsigned/redacted_because").is_some() {
            self.redacted.insert(id.clone());
        }
        if value["type"] == "m.room.redaction" {
            if let Some(target) = value
                .get("redacts")
                .or_else(|| value.pointer("/content/redacts"))
                .and_then(Value::as_str)
                .and_then(|s| OwnedEventId::try_from(s).ok())
            {
                self.redacted.insert(target);
            }
        }
        // Never replace already decrypted content with a stale encrypted page.
        if value["type"] == "m.room.encrypted"
            && self
                .events
                .get(&id)
                .is_some_and(|e| e["type"] != "m.room.encrypted")
        {
            return;
        }
        self.events.insert(id, value);
    }
    pub fn unavailable(&self) -> usize {
        self.events
            .values()
            .filter(|e| e["type"] == "m.room.encrypted")
            .count()
    }
    pub fn unsupported(&self) -> usize {
        self.events
            .values()
            .filter(|e| {
                e["content"]
                    .get(super::ROOM_TYPE)
                    .is_some_and(|v| v["version"] != 1)
            })
            .count()
    }
    fn original(&self, room: &ruma::RoomId, id: &ruma::EventId) -> Option<Entry> {
        if self.redacted.contains(id) {
            return None;
        }
        let raw = self.events.get(id)?;
        if raw["type"] != "m.room.message"
            || raw["content"].pointer("/m.relates_to/rel_type") == Some(&json!("m.replace"))
        {
            return None;
        }
        let sender = OwnedUserId::try_from(raw["sender"].as_str()?).ok()?;
        let mut content = raw["content"].clone();
        let mut edited = false;
        let latest = self.events.iter().filter(|(edit_id, e)| {
            !self.redacted.contains(*edit_id) && e["type"] == "m.room.message" && e["sender"] == sender.as_str()
                && e["content"].pointer("/m.relates_to/rel_type") == Some(&json!("m.replace"))
                && e["content"].pointer("/m.relates_to/event_id") == Some(&json!(id))
                && e["content"]["m.new_content"]["body"].is_string()
                // An edit cannot change an entry from a comment into a root post.
                && e["content"]["m.new_content"]["m.relates_to"] == content["m.relates_to"]
                && e["content"]["m.new_content"][super::ROOM_TYPE]["version"] == content[super::ROOM_TYPE]["version"]
        }).max_by_key(|(id,e)| (e["origin_server_ts"].as_u64().unwrap_or(0), (*id).clone()));
        if let Some((_, edit)) = latest {
            content = edit["content"]["m.new_content"].clone();
            edited = true;
        }
        Some(Entry {
            room: room.to_owned(),
            id: id.to_owned(),
            sender,
            timestamp: raw["origin_server_ts"].as_u64()?,
            content,
            edited,
        })
    }
    pub fn posts(&self, room: &ruma::RoomId, author: &ruma::UserId) -> Vec<Entry> {
        let mut posts: Vec<_> = self
            .events
            .keys()
            .filter_map(|id| self.original(room, id))
            .filter(|e| {
                e.sender == author
                    && e.content[super::ROOM_TYPE]["version"] == 1
                    && e.content.get("m.relates_to").is_none()
                    && e.content["body"].is_string()
            })
            .collect();
        posts.sort_by(|a, b| (b.timestamp, &b.id).cmp(&(a.timestamp, &a.id)));
        posts
    }
    pub fn comments(&self, post: &Entry) -> Vec<Entry> {
        let mut comments: Vec<_> = self
            .events
            .keys()
            .filter_map(|id| self.original(&post.room, id))
            .filter(|e| {
                e.content.pointer("/m.relates_to/rel_type") == Some(&json!("m.thread"))
                    && e.content.pointer("/m.relates_to/event_id") == Some(&json!(post.id))
                    && e.content["body"].is_string()
            })
            .collect();
        comments.sort_by(|a, b| (a.timestamp, &a.id).cmp(&(b.timestamp, &b.id)));
        comments
    }
    pub fn likes(&self, post: &Entry) -> BTreeMap<OwnedUserId, Vec<OwnedEventId>> {
        let mut likes: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (id, e) in &self.events {
            if self.redacted.contains(id) || e["type"] != "m.reaction" {
                continue;
            }
            let relation = &e["content"]["m.relates_to"];
            if relation["rel_type"] != "m.annotation"
                || relation["event_id"] != post.id.as_str()
                || relation["key"] != LIKE
            {
                continue;
            }
            if let Some(sender) = e["sender"]
                .as_str()
                .and_then(|s| OwnedUserId::try_from(s).ok())
            {
                likes.entry(sender).or_default().push(id.clone());
            }
        }
        likes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(id: &str, sender: &str, content: Value) -> Value {
        json!({"event_id":id,"sender":sender,"type":"m.room.message","origin_server_ts":1,"content":content})
    }
    #[test]
    fn ownership_and_cross_room_relations_are_not_content_claims() {
        let room = ruma::room_id!("!moments:example.org");
        let owner = ruma::user_id!("@alice:example.org");
        let mut index = Index::default();
        index.insert(
            room,
            event("$own", owner.as_str(), post_content("real", &[])),
        );
        index.insert(
            room,
            event("$forged", "@bob:example.org", post_content("fake", &[])),
        );
        let mut wrong = event(
            "$cross",
            "@bob:example.org",
            comment_content("no", ruma::event_id!("$own")),
        );
        wrong["room_id"] = json!("!elsewhere:example.org");
        index.insert(room, wrong);
        index.insert(
            room,
            event(
                "$comment",
                "@bob:example.org",
                comment_content("yes", ruma::event_id!("$own")),
            ),
        );
        let posts = index.posts(room, owner);
        assert_eq!(posts.len(), 1);
        assert_eq!(index.comments(&posts[0]).len(), 1);
    }
    #[test]
    fn edits_redactions_and_key_recovery_keep_stable_identity() {
        let room = ruma::room_id!("!moments:example.org");
        let owner = ruma::user_id!("@alice:example.org");
        let mut i = Index::default();
        i.insert(room, json!({"event_id":"$p","type":"m.room.encrypted"}));
        assert_eq!(i.unavailable(), 1);
        i.insert(
            room,
            event("$p", owner.as_str(), post_content("before", &[])),
        );
        assert_eq!(i.unavailable(), 0);
        let edit = json!({"body":"* after","m.new_content":post_content("after",&[]),"m.relates_to":{"rel_type":"m.replace","event_id":"$p"}});
        i.insert(room, event("$bad", "@bob:example.org", edit.clone()));
        assert_eq!(i.posts(room, owner)[0].body(), "before");
        i.insert(room, event("$good", owner.as_str(), edit));
        assert_eq!(i.posts(room, owner)[0].body(), "after");
        i.insert(
            room,
            json!({"event_id":"$r","type":"m.room.redaction","content":{"redacts":"$good"}}),
        );
        assert_eq!(i.posts(room, owner)[0].body(), "before");
        i.insert(
            room,
            json!({"event_id":"$r2","type":"m.room.redaction","redacts":"$p"}),
        );
        assert!(i.posts(room, owner).is_empty());
    }
    #[test]
    fn duplicate_likes_and_unknown_versions() {
        let room = ruma::room_id!("!moments:example.org");
        let owner = ruma::user_id!("@alice:example.org");
        let mut i = Index::default();
        i.insert(room, event("$p", owner.as_str(), post_content("post", &[])));
        for id in ["$l1", "$l2"] {
            i.insert(room,json!({"event_id":id,"sender":owner,"type":"m.reaction","content":{"m.relates_to":{"rel_type":"m.annotation","event_id":"$p","key":LIKE}}}));
        }
        let post = i.posts(room, owner).remove(0);
        assert_eq!(i.likes(&post).len(), 1);
        assert_eq!(i.likes(&post)[owner].len(), 2);
        let mut future = post_content("future", &[]);
        future[super::super::ROOM_TYPE]["version"] = json!(2);
        i.insert(room, event("$future", owner.as_str(), future));
        assert_eq!(i.posts(room, owner).len(), 1);
        assert_eq!(i.unsupported(), 1);
        assert_eq!(
            post.content,
            json!({"msgtype":"m.text","body":"post","rs.robius.robrix.moments":{"version":1,"media":[]}})
        );
    }
}
