//! ADR-186: an engagement's token allocation is chosen at approval (§A),
//! pauses its agent when used up (§B), and can be topped up (§C).
mod common;
use common::*;
use hagency_core::project::EngagementState;
use hagency_store::{DomainRepository, Error};

fn setup(ceiling: u64) -> (tempfile::TempDir, DomainRepository) {
    let dir = tempfile::tempdir().unwrap();
    let mut db = DomainRepository::open(&dir.path().join("state")).unwrap();
    db.register(&registration()).unwrap();
    db.put_resource(&resource("alloc_pool", "alloc_seat", ceiling))
        .unwrap();
    (dir, db)
}
fn admitted(db: &mut DomainRepository, id: &str, name: &str, tokens: u64) -> String {
    let pool = resource("alloc_pool", "alloc_seat", 0);
    db.admit(&proof(&request(id, name, &pool, tokens)), 1000)
        .unwrap()
        .id
}
fn allocated_column(dir: &tempfile::TempDir, id: &str) -> Option<u64> {
    rusqlite::Connection::open(dir.path().join("state/domain.sqlite3"))
        .unwrap()
        .query_row(
            "SELECT allocated_tokens FROM engagements WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
}

/// §A1/§A4: the operator grants less than the request. The engagement holds
/// the granted amount everywhere a commitment is counted — reservation,
/// draw and side commitment — while the request stays the requester's ask.
#[test]
fn native_allocation_approve_with_a_lower_amount() {
    let (dir, mut db) = setup(1000);
    let id = admitted(&mut db, "low_request", "LowWorker", 300);
    let pool = resource("alloc_pool", "alloc_seat", 0);
    let p = proof(&request("low_request", "LowWorker", &pool, 300));
    let approved = db.approve_allocating("approve_low", &p, 1000, Some(120)).unwrap();
    assert_eq!(approved.state, EngagementState::Reserved);
    assert_eq!(u64::from(approved.requested_tokens), 300, "the ask is kept");
    assert_eq!(approved.allocated_tokens.map(u64::from), Some(120));
    assert_eq!(u64::from(approved.allocation()), 120);
    assert_eq!(allocated_column(&dir, &id), Some(120));
    // Reservation and draw: the ceiling report and the pool budget count 120.
    let report = db.resource_ceiling(&pool.id(), 1000).unwrap();
    assert_eq!(report.reserved, 120);
    assert_eq!(report.drawn, 120);
    let budget = db.resource_budget(&pool.id()).unwrap();
    assert_eq!(u64::from(budget.pool.committed), 120);
    // Side commitment.
    let side = db.side_budget("example.test").unwrap();
    assert_eq!(side.committed, 120);
    assert_eq!(side.commitments[0].allocated_tokens, 120);
    // The headroom behind any engagement on the resource is what is left.
    assert_eq!(db.engagement_headroom(&id, 1000).unwrap(), Some(880));
    // The same command id replays the receipt; the same id with another
    // amount is a different act and conflicts.
    assert_eq!(
        db.approve_allocating("approve_low", &p, 1000, Some(120))
            .unwrap()
            .allocated_tokens
            .map(u64::from),
        Some(120)
    );
    assert!(matches!(
        db.approve_allocating("approve_low", &p, 1000, Some(121)),
        Err(Error::Conflict)
    ));
}

/// §A1: without an amount the approval is the plain one — the requested
/// amount is the allocation and no column is written.
#[test]
fn native_allocation_plain_approval_keeps_the_request() {
    let (dir, mut db) = setup(1000);
    let id = admitted(&mut db, "plain_request", "PlainWorker", 200);
    let pool = resource("alloc_pool", "alloc_seat", 0);
    let p = proof(&request("plain_request", "PlainWorker", &pool, 200));
    let approved = db.approve_allocating("approve_plain", &p, 1000, None).unwrap();
    assert_eq!(approved.allocated_tokens, None);
    assert_eq!(u64::from(approved.allocation()), 200);
    assert_eq!(allocated_column(&dir, &id), None);
    // The plain `approve` shares the decision digest: it replays.
    assert!(db.approve("approve_plain", &p, 1000).is_ok());
    // A zero amount is not an allocation.
    let other = admitted(&mut db, "zero_request", "ZeroWorker", 10);
    let zero = proof(&request("zero_request", "ZeroWorker", &pool, 10));
    assert!(matches!(
        db.approve_allocating("approve_zero", &zero, 1000, Some(0)),
        Err(Error::Invalid(_))
    ));
    assert_eq!(db.get(&other).unwrap().state, EngagementState::Pending);
}

/// §A2: an amount above the headroom is refused with the over-commit
/// refusal and its human message; the engagement stays pending.
#[test]
fn native_allocation_approve_above_headroom_is_refused_with_the_message() {
    let (_dir, mut db) = setup(1000);
    let id = admitted(&mut db, "big_request", "BigWorker", 100);
    let pool = resource("alloc_pool", "alloc_seat", 0);
    let p = proof(&request("big_request", "BigWorker", &pool, 100));
    match db.approve_allocating("approve_big", &p, 1000, Some(1001)) {
        Err(Error::OverCommit { message }) => {
            assert!(message.contains("BigWorker"), "{message}");
            assert!(message.contains("would exceed"), "{message}");
            assert!(message.contains("ceiling"), "{message}");
        }
        other => panic!("expected over_commit, got {other:?}"),
    }
    assert_eq!(db.get(&id).unwrap().state, EngagementState::Pending);
    // A larger REQUEST approved with a smaller amount fits.
    let larger = admitted(&mut db, "larger_request", "LargerWorker", 5000);
    let q = proof(&request("larger_request", "LargerWorker", &pool, 5000));
    assert!(matches!(
        db.approve("approve_larger_plain", &q, 1000),
        Err(Error::OverCommit { .. })
    ));
    assert_eq!(
        db.approve_allocating("approve_larger", &q, 1000, Some(900))
            .unwrap()
            .allocated_tokens
            .map(u64::from),
        Some(900)
    );
    assert_eq!(db.get(&larger).unwrap().state, EngagementState::Reserved);
}

/// §A3: "All remaining" is exactly the headroom the approval is checked
/// against — approving it fits, and one token more does not.
#[test]
fn native_allocation_all_remaining() {
    let (_dir, mut db) = setup(1000);
    let pool = resource("alloc_pool", "alloc_seat", 0);
    let first = admitted(&mut db, "first_request", "FirstWorker", 250);
    db.approve(
        "approve_first",
        &proof(&request("first_request", "FirstWorker", &pool, 250)),
        1000,
    )
    .unwrap();
    let id = admitted(&mut db, "rest_request", "RestWorker", 100);
    let all = db.engagement_headroom(&id, 1000).unwrap().unwrap();
    assert_eq!(all, 750);
    let p = proof(&request("rest_request", "RestWorker", &pool, 100));
    assert!(matches!(
        db.approve_allocating("approve_rest_over", &p, 1000, Some(all + 1)),
        Err(Error::OverCommit { .. })
    ));
    let approved = db.approve_allocating("approve_rest", &p, 1000, Some(all)).unwrap();
    assert_eq!(approved.allocated_tokens.map(u64::from), Some(750));
    assert_eq!(db.engagement_headroom(&id, 1000).unwrap(), Some(0));
    assert_eq!(db.engagement_headroom(&first, 1000).unwrap(), Some(0));
}
