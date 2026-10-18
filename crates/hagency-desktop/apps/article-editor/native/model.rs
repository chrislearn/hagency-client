use std::time::Duration;
use article_core::host::{Capabilities, Capability, ConsentGrant};
use super::host::AUTHORITY;
use ruma::{
    OwnedUserId,
    events::room::message::{MessageType, RoomMessageEventContent},
};
use serde::{Deserialize, Serialize};

pub const MSGTYPE: &str = "rs.robius.robrix.article_app";
const APP_ID: &str = crate::system_apps::ARTICLE_ID;
pub use article_core::bindings::SOURCE;
pub fn invalidate_sessions() { AUTHORITY.invalidate(); }

/// A reference to a built-in, reviewed package. The build is the trust root;
/// this is deliberately NOT a remotely asserted publisher signature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArticlePackage {
    pub app_id: String,
    pub version: u32,
    pub source_hash: String,
}
impl ArticlePackage {
    pub fn builtin() -> Self {
        Self {
            app_id: APP_ID.into(),
            version: 2,
            source_hash: blake3::hash(SOURCE.as_bytes()).to_hex().to_string(),
        }
    }
    pub fn from_message(message: &MessageType) -> Result<Self, String> {
        if message.msgtype() != MSGTYPE {
            return Err("Not an article app".into());
        }
        let package: Self = serde_json::from_value(
            message
                .data()
                .get("app")
                .cloned()
                .ok_or("Missing article app")?,
        )
        .map_err(|_| "Invalid article app")?;
        let builtin = Self::builtin();
        if package.app_id != builtin.app_id || package.source_hash != builtin.source_hash || !matches!(package.version, 1 | 2) {
            return Err("This article app needs a compatible Rinx update.".into());
        }
        Ok(package)
    }
    pub fn message(&self) -> RoomMessageEventContent {
        let mut data = serde_json::Map::new();
        data.insert("app".into(), serde_json::to_value(self).unwrap());
        RoomMessageEventContent::new(
            MessageType::new(
                MSGTYPE,
                "[Mini app] Article editor · Markdown / native preview".into(),
                data,
            )
            .unwrap(),
        )
    }
}

/// Matrix identity is kept by the Rinx adapter; consent is host-independent.
#[derive(Clone, Debug)]
pub struct Grant {
    pub owner: OwnedUserId,
    pub instance: String,
    pub(super) lease: ConsentGrant,
}
impl Grant {
    pub fn new(owner: OwnedUserId) -> Self {
        let lease = AUTHORITY.issue(owner.to_string(), Capabilities::editor(), Duration::from_secs(3600));
        Self::from_lease(owner, lease)
    }
    pub fn reader(owner: OwnedUserId) -> Self {
        let lease = AUTHORITY.issue(owner.to_string(), Capabilities::reader(), Duration::from_secs(3600));
        Self::from_lease(owner, lease)
    }
    pub(super) fn from_lease(owner: OwnedUserId, lease: ConsentGrant) -> Self {
        Self { owner, instance: lease.instance().into(), lease }
    }
    pub fn valid(&self, owner: Option<&ruma::UserId>) -> bool {
        AUTHORITY.valid(&self.lease, owner.map(|o| o.as_str()))
    }
    pub fn authorize(&self, capability: Capability) -> Result<(), String> {
        use article_core::host::ArticleHost;
        super::host::RobrixArticleHost::new(crate::app_data_dir()).authorize(&self.lease, capability)
    }
    pub fn revoke(&self) { self.lease.revoke(); }
}

pub use article_core::editing::EditorState as Draft;
pub use article_core::bindings::{realize_editor, apply_input};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_never_transports_identity_or_grants() {
        let package = ArticlePackage::builtin();
        let message = package.message();
        assert_eq!(ArticlePackage::from_message(&message.msgtype).unwrap(), package);
        let wire = serde_json::to_string(&message).unwrap();
        for key in ["access_token", "owner", "grant", "draft", "password"] { assert!(!wire.contains(key)); }
        let mut forged = package;
        forged.source_hash = "forged".into();
        assert!(ArticlePackage::from_message(&forged.message().msgtype).is_err());
    }
}
