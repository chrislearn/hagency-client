use anyhow::{Result, bail, ensure};
use matrix_sdk::{Client, Room, RoomState, RoomMemberships, room::MessagesOptions, reqwest};
use ruma::{OwnedUserId, serde::Raw};
use serde_json::{Value, json};
use super::{SCHEMA, EVENT_KEY, protocol::*};

const MAX_RESPONSE: usize = 1024 * 1024;

/// A separate HTTP client: never reuse Matrix authorization or proxy settings.
pub struct Transport(reqwest::Client);
impl Transport {
    pub fn new() -> Result<Self> {
        Ok(Self(
            crate::http::client_builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(15))
                .build()?,
        ))
    }
    async fn response(response: reqwest::Response) -> Result<Option<Value>> {
        let status = response.status();
        ensure!(
            !status.is_redirection(),
            "Agent endpoint redirects are refused"
        );
        if status.as_u16() == 204 {
            return Ok(None);
        }
        ensure!(
            response.content_length().unwrap_or(0) <= MAX_RESPONSE as u64,
            "Agent response is too large"
        );
        let mut response = response;
        let mut data = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                data.len() + chunk.len() <= MAX_RESPONSE,
                "Agent response is too large"
            );
            data.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&data)?;
        if !status.is_success() {
            // Do not echo arbitrary server errors: they may contain capabilities.
            let code = value["code"]
                .as_str()
                .filter(|s| s.len() <= 64 && s.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
                .unwrap_or("request_failed");
            bail!("Agent request refused: {code}");
        }
        Ok(Some(value))
    }
    pub async fn exchange(
        &self,
        key: ClientKey,
        config: Config,
        owner: OwnedUserId,
        grant: Value,
        client_nonce: &str,
    ) -> Result<Session> {
        validate_grant(&grant, &config, client_nonce)?;
        let path = "/api/agent-ops/v1/session/exchange";
        let body = json!({"grant_jti":grant["grant_jti"], "client_nonce":client_nonce,"server_challenge":grant["server_challenge"],"audience":config.origin});
        let proof_nonce = nonce();
        let material = json!({"schema":SCHEMA,"kind":"grant_exchange","grant_jti":grant["grant_jti"],"client_nonce":client_nonce,
            "server_challenge":grant["server_challenge"],"proof_nonce":proof_nonce,"http_method":"POST","request_path":path,"body_sha256":digest(&body),"audience":config.origin});
        let response = self
            .0
            .post(format!("{}{path}", config.origin))
            .header("X-Agent-Ops-Proof-Nonce", proof_nonce)
            .header("X-Agent-Ops-Proof", key.sign(&material))
            .header("Content-Type", "application/json")
            .body(canonical(&body))
            .send()
            .await?;
        let session = Self::response(response)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Missing session response"))?;
        Session::from_exchange(key, config, owner, grant, session)
    }
    pub async fn request(
        &self,
        session: &Session,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Option<Value>> {
        session.valid()?;
        ensure!(
            path.starts_with("/api/agent-ops/v1/") && !path.contains('#'),
            "Invalid Agent Operations path"
        );
        let method = if body.is_some() { "POST" } else { "GET" };
        let proof_nonce = nonce();
        let material = session.proof(method, path, body.unwrap_or(&Value::Null), &proof_nonce);
        let mut request = self
            .0
            .request(
                if body.is_some() {
                    reqwest::Method::POST
                } else {
                    reqwest::Method::GET
                },
                format!("{}{path}", session.config.origin),
            )
            .header(
                "Authorization",
                format!(
                    "AgentOps {}",
                    string(&session.session, "session_capability")?
                ),
            )
            .header(
                "X-Agent-Ops-Client-Session",
                string(&session.session, "client_session_id")?,
            )
            .header("X-Agent-Ops-Proof-Nonce", proof_nonce)
            .header("X-Agent-Ops-Proof", session.key.sign(&material));
        if let Some(body) = body {
            ensure!(
                canonical(body).len() <= 100 * 1024,
                "Agent request is too large"
            );
            request = request
                .header("Content-Type", "application/json")
                .body(canonical(body));
        }
        Self::response(request.send().await?).await
    }
    pub async fn refresh(&self, session: &mut Session) -> Result<()> {
        let value = self
            .request(session, "/api/agent-ops/v1/snapshot", None)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Missing snapshot"))?;
        session.accept_snapshot(value)
    }
    pub async fn invalidated(&self, session: &Session) -> Result<bool> {
        let after = session
            .snapshot
            .as_ref()
            .map(|s| number(s, "seq"))
            .transpose()?
            .unwrap_or(0);
        let Some(value) = self
            .request(
                session,
                &format!("/api/agent-ops/v1/invalidation?after={after}"),
                None,
            )
            .await?
        else {
            return Ok(false);
        };
        session.validate_projection(&value)?;
        ensure!(
            value["scope_id"] == session.session["scope_id"],
            "Agent scope mismatch"
        );
        Ok(true)
    }
}

pub async fn guard(
    client: &Client,
    owner: &OwnedUserId,
    config: &Config,
) -> Result<(Room, OwnedUserId)> {
    ensure!(
        crate::sliding_sync::current_user_id().as_ref() == Some(owner),
        "Account changed; reconnect"
    );
    let room = client
        .get_room(&config.owner_room)
        .ok_or_else(|| anyhow::anyhow!("Join the owner approval room first"))?;
    ensure!(
        room.state() == RoomState::Joined && room.encryption_state().is_encrypted(),
        "Use an encrypted owner approval room"
    );
    ensure!(
        client
            .get_room(&config.project_room)
            .is_some_and(|room| room.state() == RoomState::Joined),
        "Join the project room first"
    );
    let members = room.members(RoomMemberships::ACTIVE).await?;
    ensure!(
        members.len() == 3
            && members
                .iter()
                .all(|m| m.membership() == &ruma::events::room::member::MembershipState::Join)
            && members.iter().any(|m| m.user_id() == owner)
            && members.iter().any(|m| m.user_id() == config.bridge)
            && members
                .iter()
                .any(|m| m.user_id() != owner && m.user_id() != config.bridge),
        "Owner approval room membership changed"
    );
    let agent = members
        .iter()
        .find(|m| m.user_id() != owner && m.user_id() != config.bridge)
        .unwrap()
        .user_id()
        .to_owned();
    ensure!(
        config
            .owner_agent
            .as_ref()
            .is_none_or(|expected| expected == &agent),
        "Owner approval room membership changed"
    );
    Ok((room, agent))
}

pub async fn connect(client: &Client, mut config: Config) -> Result<(Transport, Session)> {
    ensure!(
        super::available(),
        "Agent Operations awaits a released backend contract"
    );
    super::contract::verify()?;
    config.validate()?;
    let owner = client
        .user_id()
        .ok_or_else(|| anyhow::anyhow!("Sign in first"))?
        .to_owned();
    let (room, agent) = guard(client, &owner, &config).await?;
    config.owner_agent = Some(agent);
    let key = ClientKey::generate();
    let client_nonce = nonce();
    let content = json!({"msgtype":"com.hagency.agent_ops.client_session.request.v1", "body":"Request a scoped Agent Operations session",
        EVENT_KEY:{"schema":SCHEMA,"agent":config.agent,"project_room_id":config.project_room,"client_nonce":client_nonce,"client_public_jwk":key.jwk()}});
    client
        .encryption()
        .request_user_identity(&config.bridge)
        .await?;
    room.discard_room_key().await?;
    room.send_raw("m.room.message", Raw::new(&content)?.cast_unchecked())
        .await?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(90);
    while tokio::time::Instant::now() < deadline {
        guard(client, &owner, &config).await?;
        let mut options = MessagesOptions::backward();
        options.limit = ruma::uint!(40);
        for event in room.messages(options).await?.chunk {
            // A plaintext event in an encrypted room is not an authenticated grant.
            if event.kind.encryption_info().is_none() {
                continue;
            }
            let value: Value = serde_json::from_str(event.kind.raw().json().get())?;
            if value["sender"] != config.bridge.as_str()
                || value["content"]["msgtype"] != "com.hagency.agent_ops.client_session.grant.v1"
            {
                continue;
            }
            let grant = &value["content"][EVENT_KEY];
            if grant["client_nonce"] != client_nonce {
                continue;
            }
            let transport = Transport::new()?;
            let mut session = transport
                .exchange(key, config, owner, grant.clone(), &client_nonce)
                .await?;
            transport.refresh(&mut session).await?;
            return Ok((transport, session));
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    bail!("No session grant received. Check device enrollment and the backend feature settings.")
}
