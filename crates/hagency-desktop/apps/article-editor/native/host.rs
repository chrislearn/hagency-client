//! Rinx owns the login lifecycle, filesystem root and Matrix client.
//! Shared article crates have no dependency back into this application.
use std::{path::Path, sync::LazyLock};
use article_core::host::{ArticleHost, ArticlePublisher, Capability, ConsentGrant, SessionAuthority};
use super::{model::Grant, storage::{Operation, Publication}};

pub(super) static AUTHORITY: LazyLock<SessionAuthority> = LazyLock::new(SessionAuthority::default);

pub struct RobrixArticleHost<'a> { root: &'a Path }
impl<'a> RobrixArticleHost<'a> {
    pub fn new(root: &'a Path) -> Self { Self { root } }
}
impl ArticleHost for RobrixArticleHost<'_> {
    fn active_account(&self) -> Option<String> {
        if crate::logout::logout_state_machine::is_logout_in_progress() { return None; }
        crate::sliding_sync::current_user_id().map(|id| id.to_string())
    }
    fn data_root(&self) -> &Path { self.root }
    fn authority(&self) -> &SessionAuthority { &AUTHORITY }
}

pub struct RobrixPublisher;
impl ArticlePublisher for RobrixPublisher {
    type Operation = Operation;
    type Receipt = Publication;
    fn publish(&self, lease: ConsentGrant, operation: Operation)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Publication, String>> + Send + '_>> {
        Box::pin(async move {
            let host = RobrixArticleHost::new(crate::app_data_dir());
            host.authorize(&lease, Capability::Publish)?;
            let owner = ruma::UserId::parse(lease.owner()).map_err(|_| "Invalid article account")?;
            let grant = Grant::from_lease(owner, lease);
            let client = crate::sliding_sync::get_client().ok_or("Authorization expired")?;
            super::backend::execute(client, grant, operation).await
        })
    }
}
