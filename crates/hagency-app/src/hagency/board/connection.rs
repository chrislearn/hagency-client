//! The production path always resolves the current SDK-backed management service.
//! Debug examples may attach a real NativeOwner to an explicit local fixture.
use super::*;

#[derive(Clone)]
pub(super) enum BoardConnection {
    Current(Service),
    #[cfg(debug_assertions)]
    Development {
        owner: std::sync::Arc<::hagency::native_owner::NativeOwner>,
        identity: String,
    },
}
impl BoardConnection {
    pub(super) fn identity(&self) -> String {
        match self {
            Self::Current(service) => service.identity(),
            #[cfg(debug_assertions)]
            Self::Development { identity, .. } => identity.clone(),
        }
    }
    pub(super) async fn execute(&self, command: Command) -> Result<Value, String> {
        match self {
            Self::Current(service) => service.execute(command).await,
            #[cfg(debug_assertions)]
            Self::Development { owner, .. } => owner.execute(command).await.map_err(|e| e.code),
        }
    }
}
impl ProjectBoard {
    pub(super) fn connection(&self) -> Result<BoardConnection, String> {
        if let Some(connection) = &self.connection_override {
            return Ok(connection.clone());
        }
        Service::for_current().map(BoardConnection::Current)
    }
}
#[cfg(debug_assertions)]
impl ProjectBoardRef {
    /// Attach the same board UI to a real native owner in a development example.
    /// The example must validate its disposable loopback fixture before opening it.
    /// No production caller can bypass SDK account/session checks through this API.
    pub fn connect_development(
        &self,
        cx: &mut Cx,
        owner: std::sync::Arc<::hagency::native_owner::NativeOwner>,
        project: &str,
        account: &str,
    ) {
        if let Some(mut board) = self.borrow_mut() {
            let identity = format!("development:{account}:{}", nonce());
            board.connection_override = Some(BoardConnection::Development {
                owner,
                identity: identity.clone(),
            });
            board.bind(cx, &identity, Some(project), true);
        }
    }
}
