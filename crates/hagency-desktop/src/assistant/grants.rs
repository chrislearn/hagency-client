//! Which rooms the assistant may read, per Matrix account, kept on disk.
//!
//! A grant is the person's "Always allow" on the read sheet. It belongs to
//! the account that was signed in when they gave it: another account on the
//! same device never inherits it. Settings → Privacy lists and revokes them.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// More than anyone grants by hand; a corrupt or hostile file cannot grow past it.
const MAX_ROOMS_PER_ACCOUNT: usize = 512;

#[derive(Default, Serialize, Deserialize)]
struct File {
    version: u32,
    accounts: BTreeMap<String, BTreeSet<String>>,
}

pub struct Grants {
    path: Option<PathBuf>,
    accounts: BTreeMap<String, BTreeSet<String>>,
}

impl Grants {
    /// Grants kept only in memory (tests).
    pub fn in_memory() -> Self {
        Self { path: None, accounts: BTreeMap::new() }
    }

    /// Grants persisted at `path`. A missing or unreadable file is no grants:
    /// the person is asked again, never the other way round.
    pub fn load(path: PathBuf) -> Self {
        let accounts = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<File>(&bytes).ok())
            .filter(|file| file.version == 1)
            .map(|file| file.accounts)
            .unwrap_or_default();
        Self { path: Some(path), accounts }
    }

    pub fn allows(&self, account: &str, room: &str) -> bool {
        self.accounts.get(account).is_some_and(|rooms| rooms.contains(room))
    }

    /// The rooms `account` granted, sorted by id.
    pub fn rooms(&self, account: &str) -> Vec<String> {
        self.accounts.get(account).map(|rooms| rooms.iter().cloned().collect()).unwrap_or_default()
    }

    pub fn grant(&mut self, account: &str, room: &str) -> Result<(), String> {
        let rooms = self.accounts.entry(account.to_owned()).or_default();
        if !rooms.contains(room) && rooms.len() >= MAX_ROOMS_PER_ACCOUNT {
            return Err("too many rooms granted; revoke some in Settings".into());
        }
        rooms.insert(room.to_owned());
        self.save()
    }

    pub fn revoke(&mut self, account: &str, room: &str) -> Result<(), String> {
        let Some(rooms) = self.accounts.get_mut(account) else { return Ok(()) };
        rooms.remove(room);
        if rooms.is_empty() {
            self.accounts.remove(account);
        }
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else { return Ok(()) };
        let file = File { version: 1, accounts: self.accounts.clone() };
        let bytes = serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        // Write then rename: a crash leaves the old grants, never half a file.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_persist_per_account_and_revoke() {
        let dir = std::env::temp_dir().join(format!("rinx-grants-{}", uuid::Uuid::new_v4()));
        let path = dir.join("room_grants.json");
        let mut grants = Grants::load(path.clone());
        grants.grant("@alice:x", "!team:x").unwrap();
        let reloaded = Grants::load(path.clone());
        assert!(reloaded.allows("@alice:x", "!team:x"));
        assert!(!reloaded.allows("@bob:x", "!team:x"), "never across accounts");
        assert_eq!(reloaded.rooms("@alice:x"), vec!["!team:x".to_string()]);
        let mut reloaded = reloaded;
        reloaded.revoke("@alice:x", "!team:x").unwrap();
        assert!(!Grants::load(path.clone()).allows("@alice:x", "!team:x"));
        // A corrupt file is no grants, not an error.
        std::fs::write(&path, b"{not json").unwrap();
        assert!(Grants::load(path).rooms("@alice:x").is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
