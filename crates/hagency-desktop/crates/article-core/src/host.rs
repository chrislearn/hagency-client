//! The embedding host owns identity and authority. No credentials cross this API.
use std::{
    path::Path,
    sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Capability {
    ReadDrafts, WriteDrafts, ReadAssets, ImportAssets, ReadPublished, Publish,
}

#[derive(Clone, Copy, Debug)]
pub struct Capabilities(u32);
impl Capabilities {
    pub fn only(capabilities: &[Capability]) -> Self {
        Self(capabilities.iter().fold(0, |bits, c| bits | (1 << *c as u8)))
    }
    pub fn editor() -> Self {
        Self::only(&[Capability::ReadDrafts, Capability::WriteDrafts,
            Capability::ReadAssets, Capability::ImportAssets,
            Capability::ReadPublished, Capability::Publish])
    }
    pub fn reader() -> Self { Self::only(&[Capability::ReadPublished]) }
    fn contains(self, capability: Capability) -> bool { self.0 & (1 << capability as u8) != 0 }
}

/// A host-owned login epoch. Keep one authority for a host's account lifecycle.
/// Invalidate on logout, account switch, or session replacement.
#[derive(Clone, Debug, Default)]
pub struct SessionAuthority(Arc<AtomicU64>);

/// Opaque, instance-scoped consent. Deliberately not serializable.
#[derive(Clone, Debug)]
pub struct ConsentGrant {
    owner: String,
    instance: String,
    authority: Arc<AtomicU64>,
    generation: u64,
    expires: Instant,
    revoked: Arc<AtomicBool>,
    capabilities: Capabilities,
}
impl SessionAuthority {
    /// Called only by a trusted host after consent (or for its read-only reader).
    pub fn issue(&self, owner: String, capabilities: Capabilities, lifetime: Duration) -> ConsentGrant {
        ConsentGrant {
            owner, instance: crate::document::new_id(), authority: self.0.clone(),
            generation: self.0.load(Ordering::SeqCst),
            expires: Instant::now() + lifetime.min(Duration::from_secs(3600)),
            revoked: Arc::new(AtomicBool::new(false)), capabilities,
        }
    }
    pub fn invalidate(&self) { self.0.fetch_add(1, Ordering::SeqCst); }
    pub fn valid(&self, grant: &ConsentGrant, account: Option<&str>) -> bool {
        Arc::ptr_eq(&self.0, &grant.authority)
            && !grant.owner.is_empty()
            && account == Some(grant.owner.as_str())
            && grant.generation == self.0.load(Ordering::SeqCst)
            && Instant::now() < grant.expires
            && !grant.revoked.load(Ordering::SeqCst)
    }
    pub fn authorize(&self, grant: &ConsentGrant, account: Option<&str>, capability: Capability) -> Result<(), String> {
        if !self.valid(grant, account) { return Err("Authorization expired".into()); }
        if !grant.capabilities.contains(capability) { return Err("This operation was not authorized.".into()); }
        Ok(())
    }
}
impl ConsentGrant {
    pub fn owner(&self) -> &str { &self.owner }
    pub fn instance(&self) -> &str { &self.instance }
    pub fn revoke(&self) { self.revoked.store(true, Ordering::SeqCst); }
}

/// Minimal host contract used by shared storage. Account identifiers are opaque:
/// a Matrix user ID, OctoSense account ID or local profile ID are all valid.
/// `data_root` is selected by trusted host code, never by document/script input.
pub trait ArticleHost {
    fn active_account(&self) -> Option<String>;
    fn data_root(&self) -> &Path;
    fn authority(&self) -> &SessionAuthority;
    fn authorize(&self, grant: &ConsentGrant, capability: Capability) -> Result<(), String> {
        self.authority().authorize(grant, self.active_account().as_deref(), capability)
    }
}

/// Publication is host-provided. The core never obtains an access token or a
/// network client. Operations/receipts remain host-specific (e.g. Matrix edits).
pub trait ArticlePublisher {
    type Operation: Send;
    type Receipt: Send;
    fn publish(&self, grant: ConsentGrant, operation: Self::Operation)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Receipt, String>> + Send + '_>>;
}
