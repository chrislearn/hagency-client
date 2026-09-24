//! Retirement room-leave and logout calls (TS parity:
//! `lib/matrix-work-executor.js:13-49`). One bounded POST per act, on the
//! host's own bounded client: the agent leaves a room
//! (`POST /_matrix/client/v3/rooms/{roomId}/leave`), the agent's device logs
//! out (`POST /_matrix/client/v3/logout`), and the representative logs out the
//! same way with its own token. Every body is exactly `{}` and the credential
//! rides as the bearer token — the retained executor's only headers.
//!
//! The TS outcome rule is kept whole: a call is revoked when the homeserver
//! answers 2xx, and a *logout* (agent or representative) also counts 401 with
//! errcode `M_UNKNOWN_TOKEN` as already-revoked — the credential it would
//! revoke is already gone. A leave has no such arm. Any other status is the
//! retained `matrix_http_<status>` refusal, and a transport failure is
//! `Unknown`: the bridge never decides it is done on its own view of the
//! transport.

use crate::http::Http;
use crate::{Error, Limits};
use reqwest::Url;
use reqwest::header::HeaderValue;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

/// The outcome of one retirement call, classified exactly as the retained
/// executor classifies it (`matrix-work-executor.js:16-18, 46-48`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetireVerdict {
    /// 2xx, or a logout answered 401 `M_UNKNOWN_TOKEN`: the intended state
    /// already holds.
    Revoked,
    /// A definitive answer that is not the intended state: the retained
    /// `matrix_http_<status>` word.
    Refused(u16),
    /// No definitive answer reached the host; the act stays retryable.
    Unknown,
}

/// One agent's whole retirement: a verdict per room in the order the rooms were
/// given, then the device logout's (`lib/matrix-work-executor.js:40-49`,
/// `backend-v2.js:10657,10663`). The two parts stay separate so a caller can
/// report which room, if any, would not release the agent — the retained
/// product reports exactly that per-membership result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRetirement {
    pub leaves: Vec<RetireVerdict>,
    pub logout: RetireVerdict,
}

/// One bounded Matrix client for retirement acts against one homeserver,
/// pinned to that host like every other `Http` (no ambient resolver).
pub struct RetireClient {
    http: Http,
    /// Retained so [`RetireClient::with_root_pem`] can rebuild the client with
    /// added trust material; the same three facts every other host call holds.
    base: Url,
    authorization: HeaderValue,
    limits: Limits,
    roots: Vec<reqwest::Certificate>,
}

impl RetireClient {
    /// `homeserver` is the side's API base URL (TS `stored.homeserver` /
    /// `side.apiBaseUrl`, trailing slashes already stripped by the caller's
    /// stored value); `token` is the exact bearer credential for the act —
    /// the agent's access token or the representative token. Construction
    /// performs no I/O.
    pub fn new(homeserver: &str, token: &str, limits: &Limits) -> Result<Self, Error> {
        let base = Url::parse(homeserver).map_err(|_| Error::Config)?;
        if base.scheme() != "https"
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(Error::Config);
        }
        let mut authorization =
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| Error::Config)?;
        authorization.set_sensitive(true);
        let roots = Vec::new();
        let http = Http::for_host(&base, Some(&authorization), limits, &roots)?;
        Ok(Self {
            http,
            base,
            authorization,
            limits: limits.clone(),
            roots,
        })
    }

    /// Trust one private CA, exactly as the provisioning host and the ordinary
    /// host config do (`TokenProvisioningHost::with_root_pem`). A retirement
    /// act reaches the same homeserver as every other call, so it accepts the
    /// same retained trust material.
    pub fn with_root_pem(mut self, pem: &[u8]) -> Result<Self, Error> {
        if pem.len() > 16384 || self.roots.len() >= 4 {
            return Err(Error::Config);
        }
        self.roots
            .push(reqwest::Certificate::from_pem(pem).map_err(|_| Error::Config)?);
        self.http =
            Http::for_host(&self.base, Some(&self.authorization), &self.limits, &self.roots)?;
        Ok(self)
    }

    /// `POST /_matrix/client/v3/rooms/{roomId}/leave` with body `{}`
    /// (matrix-work-executor.js:41-43). No `M_UNKNOWN_TOKEN` arm: TS counts a
    /// leave as done only on `response.ok`.
    ///
    /// The room id travels as ONE path segment, passed raw to the shared
    /// `Http` layer — exactly how every other ported room call passes one
    /// (`collector.rs` / `intake.rs` `…/rooms/{roomId}/state`). TS wrote
    /// `encodeURIComponent(job.roomId)` (executor:41-42), which renders `:` as
    /// `%3A`; the port's shared path layer leaves `:` literal and would
    /// double-escape a pre-encoded `%`. Both spell the same path segment, and a
    /// homeserver decodes them to the identical room id, so the port keeps its
    /// one convention rather than adding a second encoder.
    pub async fn leave_room(
        &self,
        room_id: &str,
        cancel: &CancellationToken,
    ) -> Result<RetireVerdict, Error> {
        self.act(
            &["_matrix", "client", "v3", "rooms", room_id, "leave"],
            false,
            cancel,
        )
        .await
    }

    /// `POST /_matrix/client/v3/logout` with body `{}`
    /// (matrix-work-executor.js:13-17, 42-43). A 401 answering with errcode
    /// `M_UNKNOWN_TOKEN` means the token is already revoked — `Revoked`.
    pub async fn logout(&self, cancel: &CancellationToken) -> Result<RetireVerdict, Error> {
        self.act(&["_matrix", "client", "v3", "logout"], true, cancel)
            .await
    }

    /// Retire this credential: leave every room **one by one**, then log the
    /// device out. The order is the retained one
    /// (`backend-v2.js:10657,10663` enqueue the agent's room withdrawals before
    /// `logout`), and a room that refuses (or answers nothing) does not abort
    /// the walk: the retained loop withdraws each membership independently and
    /// only then revokes the token, so one unreachable room can never strand the
    /// agent in the rest. The returned per-room verdicts are in `rooms` order,
    /// followed by the logout's.
    pub async fn retire_agent(
        &self,
        rooms: &[String],
        cancel: &CancellationToken,
    ) -> Result<AgentRetirement, Error> {
        let mut leaves = Vec::with_capacity(rooms.len());
        for room_id in rooms {
            leaves.push(self.leave_room(room_id, cancel).await?);
        }
        let logout = self.logout(cancel).await?;
        Ok(AgentRetirement { leaves, logout })
    }

    async fn act(
        &self,
        segments: &[&str],
        logout: bool,
        cancel: &CancellationToken,
    ) -> Result<RetireVerdict, Error> {
        let response = self.http.post(segments, "{}".to_owned(), cancel).await;
        match response {
            Ok(response) => Ok(verdict(response.status, response.value, logout)),
            // A dial that never connected is the one retry the bounded client
            // already exhausted; anything past that is unknown, never terminal.
            Err(Error::Cancelled) => Err(Error::Cancelled),
            Err(_) => Ok(RetireVerdict::Unknown),
        }
    }
}

/// TS `matrix_http_<status>` parity: 2xx is revoked; a logout's 401 with
/// `M_UNKNOWN_TOKEN` is revoked; anything else is the refusal word's status.
fn verdict(status: u16, value: Option<Value>, logout: bool) -> RetireVerdict {
    if (200..300).contains(&status) {
        return RetireVerdict::Revoked;
    }
    if logout
        && status == 401
        && value
            .as_ref()
            .and_then(|v| v.get("errcode"))
            .and_then(Value::as_str)
            == Some("M_UNKNOWN_TOKEN")
    {
        return RetireVerdict::Revoked;
    }
    RetireVerdict::Refused(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// TS executor:16-18 and :46-48 — a logout treats 401 `M_UNKNOWN_TOKEN` as
    /// already-revoked; a leave never does.
    #[test]
    fn retire_verdict_counts_2xx_as_revoked() {
        assert_eq!(verdict(200, Some(json!({})), false), RetireVerdict::Revoked);
        assert_eq!(verdict(200, None, true), RetireVerdict::Revoked);
        assert_eq!(verdict(204, None, false), RetireVerdict::Revoked);
    }

    #[test]
    fn retire_verdict_logout_already_revoked_only() {
        assert_eq!(
            verdict(401, Some(json!({"errcode":"M_UNKNOWN_TOKEN"})), true),
            RetireVerdict::Revoked
        );
        assert_eq!(
            verdict(401, Some(json!({"errcode":"M_UNKNOWN_TOKEN"})), false),
            RetireVerdict::Refused(401)
        );
        assert_eq!(
            verdict(401, Some(json!({"errcode":"M_FORBIDDEN"})), true),
            RetireVerdict::Refused(401)
        );
        assert_eq!(verdict(401, None, true), RetireVerdict::Refused(401));
        assert_eq!(
            verdict(500, Some(json!({})), true),
            RetireVerdict::Refused(500)
        );
    }

    /// The retained client is https-only with no embedded authority: the same
    /// refusal `HostConfig`/`TokenProvisioningHost` apply to their endpoints.
    #[test]
    fn retire_client_refuses_a_non_https_or_compound_endpoint() {
        let limits = Limits::default();
        for endpoint in [
            "http://side.example.test",
            "https://user@side.example.test",
            "https://side.example.test/?q=1",
            "https://side.example.test/#frag",
        ] {
            assert!(
                RetireClient::new(endpoint, "synthetic-credential", &limits).is_err(),
                "endpoint must be refused: {endpoint}"
            );
        }
        assert!(RetireClient::new("https://side.example.test", "synthetic-credential", &limits).is_ok());
    }
}
