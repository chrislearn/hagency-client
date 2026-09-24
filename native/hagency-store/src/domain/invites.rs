//! Pending agent invitations (task #12): the retained
//! `bridge-matrix.js` `state.pendingInvites` family
//! (`rememberPendingInvite`/`settlePendingInvite`/`listPendingInvites`,
//! `:2381-2455`) behind the backend routes `backend-v2.js:10813-10855`.
//!
//! An untrusted or unreadable inviter makes the invitation a PENDING
//! DECISION, never a join and never a log line (ADR-014's 2026-08-11
//! amendment): lending an agent spends the contributor's tokens, so the
//! decision waits for a human at the console.
//!
//! `inviter` is NULL when the invite state names no sender — surfaced,
//! never guessed, because the inviter IS the owner (ADR-002) and inventing
//! one would forge ownership. `declined` is REMEMBERED rather than deleted
//! so the invite poll cannot resurrect it: the invitation is still in
//! Matrix state and would be seen again next round. `accepted` records
//! that the join has happened (including the stale-record reconcile,
//! attributed to the policy `trusted-inviter`, never to a person).
use crate::Error;
use rusqlite::OptionalExtension;
use serde::Serialize;

/// The two settled decisions plus the waiting state — the TS store's
/// exact vocabulary (`state.pendingInvites` rows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteState {
    Pending,
    Accepted,
    Declined,
}

impl InviteState {
    fn as_str(self) -> &'static str {
        match self {
            InviteState::Pending => "pending",
            InviteState::Accepted => "accepted",
            InviteState::Declined => "declined",
        }
    }
}

/// One pending-invitation row, camelCase on the wire exactly as the TS
/// store renders it (`bridge-matrix.js:2399-2409`): `roomId`, `agentName`,
/// `inviter` (nullable), `projectServer`, `state`, `seenAt`, and the
/// decision columns when settled.
#[derive(Debug, Clone, Serialize)]
pub struct PendingInvite {
    pub room_id: String,
    pub agent_name: String,
    pub inviter: Option<String>,
    pub project_server: String,
    pub state: &'static str,
    pub seen_at: i64,
    pub decided_at: Option<i64>,
    pub decided_by: Option<String>,
}

/// TS `projectServerFromRoomId`: the server part of a room id, or the
/// whole id when it carries none — the same derivation the store applies
/// when recording.
fn project_server_from_room_id(room_id: &str) -> String {
    room_id
        .rsplit_once(':')
        .map(|(_, server)| server.to_owned())
        .unwrap_or_else(|| room_id.to_owned())
}

fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<PendingInvite> {
    Ok(PendingInvite {
        room_id: row.get(0)?,
        agent_name: row.get(1)?,
        inviter: row.get(2)?,
        project_server: row.get(3)?,
        seen_at: row.get(4)?,
        decided_at: row.get(5)?,
        decided_by: row.get(6)?,
        state: match row.get::<_, String>(7)?.as_str() {
            "accepted" => "accepted",
            "declined" => "declined",
            _ => "pending",
        },
    })
}

const SELECT_COLUMNS: &str = "room_id,agent,inviter,project_server,seen_at,decided_at,decided_by,state";

impl crate::DomainRepository {
    /// TS `rememberPendingInvite` (`bridge-matrix.js:2391-2417`): insert a
    /// pending record; returns FALSE when one already exists in state
    /// `pending` or `declined`, so a poll every few seconds does not
    /// re-notify and cannot resurrect a decline. The mode/since pair is
    /// the direct-room binding the TS DM path records
    /// (`backend-v2.js:16287`: `mode` in {direct, group}, `sinceTs` the
    /// admission boundary).
    pub fn remember_pending_invite(
        &mut self,
        room_id: &str,
        agent: &str,
        inviter: Option<&str>,
        mode: &str,
        since_ts: i64,
        now_ms: i64,
    ) -> Result<bool, Error> {
        validate_invite_key(room_id, agent)?;
        let mode = match mode {
            "direct" => "direct",
            "group" => "group",
            _ => return Err(hagency_core::InvalidInput("invalid room mode").into()),
        };
        if since_ts < 0 {
            return Err(hagency_core::InvalidInput("invalid history boundary").into());
        }
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        // The settle guard in one statement: a row in pending or declined
        // answers "already known", exactly the TS early return.
        let known: Option<String> = tx
            .query_row(
                "SELECT state FROM pending_invites WHERE room_id=?1 AND agent=?2",
                [room_id, agent],
                |r| r.get(0),
            )
            .optional()?;
        if matches!(known.as_deref(), Some("pending" | "declined")) {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO pending_invites \
             (room_id,agent,inviter,mode,since_ts,state,seen_at,project_server) \
             VALUES(?1,?2,?3,?4,?5,'pending',?6,?7) \
             ON CONFLICT(room_id,agent) DO UPDATE SET \
             inviter=excluded.inviter,mode=excluded.mode,since_ts=excluded.since_ts, \
             state='pending',seen_at=excluded.seen_at,decided_at=NULL,decided_by=NULL",
            rusqlite::params![
                room_id,
                agent,
                inviter,
                mode,
                since_ts,
                now_ms,
                project_server_from_room_id(room_id)
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// TS `backfillPendingInviteInviter` (`:2381-2389`): fills a NULL
    /// inviter on a still-pending record only — the poll that could not
    /// read the invite state records `inviter: null`, and the next poll
    /// resolves the sender; it never overwrites a known inviter and never
    /// touches a settled record.
    pub fn backfill_pending_invite_inviter(
        &mut self,
        room_id: &str,
        agent: &str,
        inviter: &str,
    ) -> Result<bool, Error> {
        validate_invite_key(room_id, agent)?;
        if inviter.is_empty() || inviter.len() > 255 {
            return Err(hagency_core::InvalidInput("invalid inviter").into());
        }
        let changed = self.db.execute(
            "UPDATE pending_invites SET inviter=?3 \
             WHERE room_id=?1 AND agent=?2 AND state='pending' AND inviter IS NULL",
            rusqlite::params![room_id, agent, inviter],
        )?;
        Ok(changed == 1)
    }

    /// TS `listPendingInvites` (`:2419-2428`): every invitation still
    /// awaiting a decision, newest `seenAt` first.
    pub fn pending_invites(&self) -> Result<Vec<PendingInvite>, Error> {
        let mut query = self.db.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM pending_invites \
             WHERE state='pending' ORDER BY seen_at DESC, room_id, agent"
        ))?;
        let rows = query.query_map([], row_from)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    /// TS `getPendingInvite`: the record for one (room, agent), whatever
    /// its state.
    pub fn pending_invite(
        &self,
        room_id: &str,
        agent: &str,
    ) -> Result<Option<PendingInvite>, Error> {
        validate_invite_key(room_id, agent)?;
        self.db
            .query_row(
                &format!(
                    "SELECT {SELECT_COLUMNS} FROM pending_invites \
                     WHERE room_id=?1 AND agent=?2"
                ),
                [room_id, agent],
                row_from,
            )
            .optional()
            .map_err(Error::from)
    }

    /// TS `settlePendingInvite` (`:2438-2447`): mark a decision. Returns
    /// None when no record exists (the TS null). `by` names the decider —
    /// `trusted-inviter` for the stale-record reconcile (the agent is
    /// already in the room, so the invitation WAS answered by policy, with
    /// no human at a screen), the operator name for console decisions.
    pub fn settle_pending_invite(
        &mut self,
        room_id: &str,
        agent: &str,
        accepted: bool,
        by: &str,
        now_ms: i64,
    ) -> Result<Option<PendingInvite>, Error> {
        validate_invite_key(room_id, agent)?;
        if by.is_empty() || by.len() > 128 {
            return Err(hagency_core::InvalidInput("invalid decider").into());
        }
        let changed = self.db.execute(
            "UPDATE pending_invites SET state=?3,decided_at=?4,decided_by=?5 \
             WHERE room_id=?1 AND agent=?2",
            rusqlite::params![
                room_id,
                agent,
                if accepted { "accepted" } else { "declined" },
                now_ms,
                by
            ],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        self.pending_invite(room_id, agent)
    }

    /// The settled mode/since pair a join consumes: the DM binding facts
    /// the accept path needs (`mode` decides the reply-scope guard, TS
    /// `matrix-direct-chat.js:279-292`).
    pub fn pending_invite_binding(
        &self,
        room_id: &str,
        agent: &str,
    ) -> Result<Option<(String, i64)>, Error> {
        validate_invite_key(room_id, agent)?;
        self.db
            .query_row(
                "SELECT mode,since_ts FROM pending_invites WHERE room_id=?1 AND agent=?2",
                [room_id, agent],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(Error::from)
    }
}

fn validate_invite_key(room_id: &str, agent: &str) -> Result<(), Error> {
    if room_id.is_empty() || room_id.len() > 256 {
        return Err(hagency_core::InvalidInput("invalid room id").into());
    }
    if agent.is_empty() || agent.len() > 255 {
        return Err(hagency_core::InvalidInput("invalid agent").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> (tempfile::TempDir, crate::DomainRepository) {
        let root = tempfile::tempdir().unwrap();
        let db = crate::DomainRepository::open(&root.path().join("domain")).unwrap();
        (root, db)
    }

    /// TS `rememberPendingInvite`: the first sight inserts pending; a
    /// second sight while pending does not re-notify (false); a decline is
    /// remembered and cannot be resurrected by another poll.
    #[test]
    fn remember_does_not_renotify_or_resurrect_a_decline() {
        let (_root, mut db) = open();
        assert!(db
            .remember_pending_invite("!room:a.test", "Worker", Some("@owner:a.test"), "group", 7, 100)
            .unwrap());
        assert!(!db
            .remember_pending_invite("!room:a.test", "Worker", Some("@other:a.test"), "group", 8, 200)
            .unwrap());
        let list = db.pending_invites().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].room_id, "!room:a.test");
        assert_eq!(list[0].agent_name, "Worker");
        assert_eq!(list[0].inviter.as_deref(), Some("@owner:a.test"));
        assert_eq!(list[0].project_server, "a.test");
        assert_eq!(list[0].state, "pending");
        assert_eq!(list[0].seen_at, 100);
        db.settle_pending_invite("!room:a.test", "Worker", false, "operator", 300)
            .unwrap();
        // The poll sees the invitation again — it must not resurrect it.
        assert!(!db
            .remember_pending_invite("!room:a.test", "Worker", None, "group", 9, 400)
            .unwrap());
        let record = db
            .pending_invite("!room:a.test", "Worker")
            .unwrap()
            .expect("decline is remembered, not deleted");
        assert_eq!(record.state, "declined");
        assert_eq!(record.decided_by.as_deref(), Some("operator"));
    }

    /// A NULL inviter is recorded as NULL (surfaced, never guessed) and a
    /// later poll that can read the sender fills it — pending rows only.
    #[test]
    fn inviter_backfill_fills_only_a_pending_null() {
        let (_root, mut db) = open();
        assert!(db
            .remember_pending_invite("!room:a.test", "Worker", None, "direct", 5, 100)
            .unwrap());
        assert!(db
            .backfill_pending_invite_inviter("!room:a.test", "Worker", "@owner:a.test")
            .unwrap());
        assert!(db
            .pending_invite("!room:a.test", "Worker")
            .unwrap()
            .unwrap()
            .inviter
            .is_some());
        // A second backfill changes nothing; a settled record is untouched.
        assert!(!db
            .backfill_pending_invite_inviter("!room:a.test", "Worker", "@other:a.test")
            .unwrap());
        db.settle_pending_invite("!room:a.test", "Worker", true, "trusted-inviter", 200)
            .unwrap();
        assert!(!db
            .backfill_pending_invite_inviter("!room:a.test", "Worker", "@third:a.test")
            .unwrap());
    }

    /// `listPendingInvites` is pending-only, newest first — the TS render
    /// order the /projects page shows.
    #[test]
    fn list_is_pending_only_newest_first() {
        let (_root, mut db) = open();
        db.remember_pending_invite("!old:a.test", "A", None, "group", 1, 100).unwrap();
        db.remember_pending_invite("!new:a.test", "B", None, "group", 2, 300).unwrap();
        db.remember_pending_invite("!mid:a.test", "C", None, "group", 3, 200).unwrap();
        db.settle_pending_invite("!mid:a.test", "C", true, "trusted-inviter", 250).unwrap();
        let list = db.pending_invites().unwrap();
        let order: Vec<&str> = list.iter().map(|r| r.room_id.as_str()).collect();
        assert_eq!(order, ["!new:a.test", "!old:a.test"]);
    }

    /// The settle the stale-reconcile performs: `accepted` attributed to
    /// the policy, and the binding facts survive for the join.
    #[test]
    fn settle_records_the_decider_and_keeps_the_binding() {
        let (_root, mut db) = open();
        db.remember_pending_invite("!room:a.test", "Worker", Some("@owner:a.test"), "direct", 42, 100)
            .unwrap();
        let settled = db
            .settle_pending_invite("!room:a.test", "Worker", true, "trusted-inviter", 500)
            .unwrap()
            .expect("record exists");
        assert_eq!(settled.state, "accepted");
        assert_eq!(settled.decided_at, Some(500));
        assert_eq!(settled.decided_by.as_deref(), Some("trusted-inviter"));
        let (mode, since) = db
            .pending_invite_binding("!room:a.test", "Worker")
            .unwrap()
            .expect("binding survives the decision");
        assert_eq!(mode, "direct");
        assert_eq!(since, 42);
        // Settling an unknown invitation answers None, the TS null.
        assert!(db
            .settle_pending_invite("!none:a.test", "Worker", true, "operator", 600)
            .unwrap()
            .is_none());
    }

    /// The TS 400s: an invalid mode or a negative history boundary.
    #[test]
    fn invalid_mode_or_boundary_is_refused() {
        let (_root, mut db) = open();
        assert!(matches!(
            db.remember_pending_invite("!r:a.test", "W", None, "dm", 1, 100),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            db.remember_pending_invite("!r:a.test", "W", None, "group", -1, 100),
            Err(Error::Invalid(_))
        ));
    }
}
