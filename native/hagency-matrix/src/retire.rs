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

/// One bounded Matrix client for retirement acts against one homeserver,
/// pinned to that host like every other `Http` (no ambient resolver).
pub struct RetireClient {
    http: Http,
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
        Ok(Self {
            http: Http::for_host(&base, Some(&authorization), limits, &[])?,
        })
    }

    /// `POST /_matrix/client/v3/rooms/{roomId}/leave` with body `{}`
    /// (matrix-work-executor.js:41-43). No `M_UNKNOWN_TOKEN` arm: TS counts a
    /// leave as done only on `response.ok`.
    pub async fn leave_room(
        &self,
        room_id: &str,
        cancel: &CancellationToken,
    ) -> Result<RetireVerdict, Error> {
        let mut segments = vec!["_matrix", "client", "v3", "rooms"];
        // urlencoding keeps the room id one path segment (`!r:s` stays intact).
        let encoded = urlencoding(room_id);
        segments.push(&encoded);
        segments.push("leave");
        self.act(&segments, false, cancel).await
    }

    /// `POST /_matrix/client/v3/logout` with body `{}`
    /// (matrix-work-executor.js:13-17, 42-43). A 401 answering with errcode
    /// `M_UNKNOWN_TOKEN` means the token is already revoked — `Revoked`.
    pub async fn logout(&self, cancel: &CancellationToken) -> Result<RetireVerdict, Error> {
        self.act(&["_matrix", "client", "v3", "logout"], true, cancel)
            .await
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

/// Percent-encode one path segment the way `encodeURIComponent` does for the
/// retained suffixes (`encodeURIComponent(job.roomId)`, executor line 41-42).
fn urlencoding(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*'
            | b'\'' | b'(' | b')' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn retire_verdict_counts_2xx_as_revoked() {
        assert_eq!(verdict(200, Some(json!({})), false), RetireVerdict::Revoked);
        assert_eq!(verdict(200, None, true), RetireVerdict::Revoked);
    }

    /// TS executor:16-18 and :46-48 — only a logout treats 401
    /// `M_UNKNOWN_TOKEN` as already-revoked; a leave never does.
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
        assert_eq!(verdict(500, Some(json!({})), true), RetireVerdict::Refused(500));
    }

    /// TS `encodeURIComponent(job.roomId)` (executor:41-42): the room id stays
    /// one segment, `:` and every other reserved byte encoded, `!` kept.
    #[test]
    fn retire_path_segment_encoding_matches_encode_uri_component() {
        assert_eq!(urlencoding("!room:example.test"), "!room%3Aexample.test");
        assert_eq!(urlencoding("a b/c?d"), "a%20b%2Fc%3Fd");
    }
}
