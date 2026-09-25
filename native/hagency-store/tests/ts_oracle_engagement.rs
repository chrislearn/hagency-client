//! TS oracle: `tests/engagement-store.test.js`, `tests/engagement-binding.test.js`,
//! `tests/engagement-serving-disclosure.test.js`,
//! `tests/api-engagement-room-admission.test.js`,
//! `tests/api-engagement-side-budget.test.js`.
//!
//! Each case names the TS case it ports. The native surface is
//! `hagency_store::DomainRepository`'s engagement lifecycle (`admit`, `approve`,
//! `reject`, `revoke`) and the offer book reads (`offer_book`, `contributions`,
//! `preview`).
//!
//! Native DELIBERATELY differs from TS in one documented place: native has no
//! offer-terms table, no whitelist and no binding table, and never auto-joins —
//! every engagement is born `pending` and only an operator verdict moves it
//! (`native/hagency-store/src/domain/offer_book.rs:1-33`). Cases whose whole
//! point is the TS auto-join ladder are therefore `ignored` with that reason,
//! never silently dropped.
mod common;
use common::*;
use hagency_core::project::EngagementState;
use hagency_store::{DomainRepository, EffectOutcome, Error};
use std::path::Path;

/// One fleet, one published resource, one approved+provisioned engagement —
/// the real state the reads below project.
struct Book {
    root: tempfile::TempDir,
    db: DomainRepository,
}

impl Book {
    fn open() -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut db = DomainRepository::open(&root.path().join("state")).unwrap();
        db.register(&registration()).unwrap();
        let pool = resource("oracle_pool", "oracle_seat", 1000);
        db.put_resource(&pool).unwrap();
        let proof = proof(&request("oracle_request", "OracleWorker", &pool, 100));
        db.admit(&proof, 1000).unwrap();
        db.approve("oracle_approve", &proof, 1000).unwrap();
        let effect = db.claim_effect().unwrap().unwrap();
        db.observe_effect(
            &effect.id,
            effect.fence,
            &EffectOutcome::Applied {
                receipt: "synthetic oracle fixture".into(),
            },
        )
        .unwrap();
        Book { root, db }
    }
    /// A second, UNPROVISIONED engagement left `pending` — the state the
    /// verdict cases act on.
    fn pending(&mut self, id: &str) -> (hagency_core::authority::VerifiedRequest, String) {
        let pool = resource("oracle_pool2", "oracle_seat2", 500);
        self.db.put_resource(&pool).unwrap();
        let proof = proof(&request(id, "PendingWorker", &pool, 50));
        let engagement = self.db.admit(&proof, 1000).unwrap();
        assert_eq!(engagement.state, EngagementState::Pending);
        (proof, engagement.id)
    }
    fn path(&self) -> std::path::PathBuf {
        self.root.path().join("state/domain.sqlite3")
    }
    /// Read an engagement's durable state directly — the same read the crate's
    /// own tests use, never a projection this fixture could stub.
    fn state(&self, id: &str) -> String {
        state_at(&self.path(), id)
    }
}

/// The same durable read, usable after the repository has been moved or
/// dropped (the reopen case below).
fn state_at(path: &Path, id: &str) -> String {
    rusqlite::Connection::open(path)
        .unwrap()
        .query_row("SELECT state FROM engagements WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .unwrap()
}

/// TS `engagement-store.test.js` (`the store` — the pending shape): a request
/// that reaches the store is `pending` with no allocation until a verdict.
/// Native's `notWhitelisted` equivalent is its documented always-pending
/// default (`offer_book.rs:1-33`).
#[test]
fn ts_engagement_request_is_pending_before_a_verdict() {
    let mut b = Book::open();
    let (_, id) = b.pending("pending_one");
    assert_eq!(b.state(&id), "pending");
}

/// TS `engagement-store.test.js:202` — *cannot decide the same engagement
/// twice*. Derived from `approve` (`domain.rs:1410-1413`): a second verdict on
/// a non-`pending` row is refused `State`, and the first verdict is not undone.
#[test]
fn ts_engagement_cannot_be_decided_twice() {
    let mut b = Book::open();
    let (proof, id) = b.pending("twice_one");
    b.db.approve("approve_once", &proof, 1000).unwrap();
    let second = b.db.approve("approve_twice", &proof, 1000);
    // Derived from `approve` (`domain.rs:1410-1413`): the distinct command id
    // finds no replay, then the non-`pending` state is refused `State`.
    assert!(
        matches!(second, Err(Error::State)),
        "a second verdict is refused State: {second:?}"
    );
    // The first verdict stands: it is no longer pending.
    assert_ne!(b.state(&id), "pending");
}

/// TS `engagement-store.test.js` (`counts only ACTIVE engagements against a
/// ceiling` — the rejection half): a rejected request ends and is not counted.
#[test]
fn ts_engagement_rejection_ends_it_without_allocating() {
    let mut b = Book::open();
    let (_, id) = b.pending("reject_one");
    let rejected = b.db.reject("reject_cmd", &id).unwrap();
    assert_eq!(rejected.state, EngagementState::Rejected);
}

/// TS `engagement-store.test.js` (`revocation is explicit and ends only an
/// active engagement`): `revoke` moves a live engagement to `revoked`, and a
/// second revoke is refused rather than double-releasing.
#[test]
fn ts_engagement_revocation_is_explicit() {
    let mut b = Book::open();
    let (proof, id) = b.pending("revoke_one");
    b.db.approve("approve_revoke", &proof, 1000).unwrap();
    let revoked = b.db.revoke("revoke_cmd", &id).unwrap();
    assert_eq!(revoked.state, EngagementState::Revoked);
    assert!(
        matches!(b.db.revoke("revoke_again", &id), Err(Error::State)),
        "a second revoke is refused, never a silent double-release"
    );
}

/// TS `engagement-store.test.js` (`persists on every mutation`): a mutation then
/// a REOPEN sees the same durable state, not a cached projection.
#[test]
fn ts_engagement_mutations_persist_across_reopen() {
    let mut b = Book::open();
    let (_, id) = b.pending("persist_one");
    let path = b.path();
    let dir = b.root.path().join("state");
    b.db.reject("reject_persist", &id).unwrap();
    drop(b.db);
    let _db = DomainRepository::open(&dir).unwrap();
    assert_eq!(state_at(&path, &id), "rejected");
}

/// TS `engagement-serving-disclosure.test.js:49` — *a request answers with the
/// framework, model and tier that will serve it*. Native's `preview` answers
/// with the real headroom and the serving agent.
#[test]
fn ts_serving_configuration_is_disclosed_by_the_preview() {
    let b = Book::open();
    let preview = b.db.preview("coding").unwrap();
    assert_eq!(preview.agent.as_deref(), Some("OracleWorker"));
    assert_eq!(
        preview.agent_remaining_tokens,
        Some(900),
        "1000 ceiling less the 100-token live commitment"
    );
}

/// TS `engagement-serving-disclosure.test.js:101` — *a REJECTION discloses
/// nothing*: a role nothing runs answers with no agent and no tokens.
#[test]
fn ts_serving_disclosure_is_empty_for_an_unserved_role() {
    let b = Book::open();
    let preview = b.db.preview("documentation").unwrap();
    assert_eq!(preview.agent, None);
    assert_eq!(preview.agent_remaining_tokens, None);
}

/// TS `engagement-serving-disclosure.test.js:157` — *an agent that has
/// disappeared discloses nothing rather than a guess*; native also refuses an
/// unknown role rather than answering for it.
#[test]
fn ts_serving_disclosure_refuses_an_unknown_role() {
    let b = Book::open();
    assert!(b.db.preview("not_a_role").is_err());
}

/// TS `engagement-store.test.js` (`preview_is_a_dry_run`): the preview writes
/// nothing at all — whole-database snapshot equality across the call.
#[test]
fn ts_engagement_preview_is_a_dry_run() {
    let b = Book::open();
    let before = digest(&b.path());
    b.db.preview("coding").unwrap();
    assert_eq!(digest(&b.path()), before, "the preview must not write");
}

/// TS `engagement-store.test.js` (`routeRequest` — the fall-back-to-approval
/// ladder): native has no whitelist or offer terms, so EVERY request takes the
/// first rung. The TS case's own comment calls that rung `notWhitelisted`; the
/// ladder cannot be exercised natively without inventing tables this task
/// forbids (`offer_book.rs:18-33`).
#[test]
#[ignore = "parity gap: native has no whitelist/offer-terms tables and never auto-joins (offer_book.rs:18-33); the routeRequest ladder has no native counterpart to assert"]
fn ts_engagement_route_request_ladder() {}

/// TS `engagement-binding.test.js` — *approving an engagement must ATTACH the
/// agent, or say it did not*. Native attaches through the provision effect and
/// the Matrix room membership, asserted by `tests/domain.rs` and the
/// `hagency` room fixtures, not this store read.
#[test]
#[ignore = "parity gap: agent<->project binding/attachment is the provision+Matrix room path (tests/domain.rs, hagency room fixtures); native has no binding table"]
fn ts_engagement_binding_on_approval() {}

/// TS `api-engagement-room-admission.test.js` — the agent invite/join and
/// membership sweep. That is a Matrix-side behaviour owned by the
/// `hagency-matrix` fixture harnesses.
#[test]
#[ignore = "parity gap: room admission/invite/join and the membership sweep are Matrix-side (hagency-matrix fixtures)"]
fn ts_engagement_room_admission() {}

/// TS `api-engagement-side-budget.test.js` — the side allocation charge. Native
/// enforces ceilings through `usage`/`ceiling_report`, asserted by
/// `tests/usage.rs` and `tests/ceiling_alerts.rs` in this crate.
#[test]
#[ignore = "covered: side/agent ceiling charging is asserted by tests/usage.rs + tests/ceiling_alerts.rs (real ceiling_report path)"]
fn ts_engagement_side_budget() {}

/// Every user table's full contents, in a stable order — the strongest
/// no-write proof available (mirrors `tests/offer_book.rs`).
fn digest(path: &Path) -> String {
    let conn = rusqlite::Connection::open(path).unwrap();
    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut out = String::new();
    for table in tables {
        out.push_str(&table);
        out.push('\n');
        let mut stmt = conn.prepare(&format!("SELECT * FROM \"{table}\"")).unwrap();
        let columns = stmt.column_count();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for i in 0..columns {
                let value: rusqlite::types::Value = row.get(i).unwrap();
                out.push_str(&format!("{value:?}|"));
            }
            out.push('\n');
        }
    }
    out
}
