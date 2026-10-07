use super::*;
const OWNER: &str = "@alice:example.org";
#[test]
fn agent_policy_before_binding_shares_runtime_budget_and_preserves_charges() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("local.db");
    let mut l = Ledger::open(&path, OWNER).unwrap();
    assert!(matches!(
        l.set_agent_policy("@other:example.org", "a1", 0, &config(50, Period::Lifetime)),
        Err(Error::Unauthorized)
    ));
    l.set_agent_policy(OWNER, "a1", 0, &config(50, Period::Lifetime))
        .unwrap();
    let model = ModelProfile {
        model: "test-model".into(),
        credential_ref: "keychain:codex:owner".into(),
        workspace_root: tmp.path().to_str().unwrap().into(),
    };
    l.set_agent_model_profile(OWNER, "a1", &model).unwrap();
    assert!(
        l.set_agent_model_profile("@other:example.org", "a1", &model)
            .is_err()
    );
    assert!(matches!(
        l.set_agent_policy(OWNER, "a1", 0, &config(999, Period::Lifetime)),
        Err(Error::Conflict)
    ));
    assert_eq!(
        l.agent_account(OWNER, "a1", Period::Lifetime, 10).unwrap(),
        (0, 0)
    );
    l.register_binding(OWNER, &scope()).unwrap();
    assert_eq!(l.model_profile(&scope()).unwrap(), Some(model.clone()));
    l.set_policy(
        OWNER,
        &scope(),
        Layer::Room,
        0,
        &config(500, Period::Lifetime),
    )
    .unwrap();
    assert_eq!(
        l.policy_snapshot(&scope()).unwrap()[0].policy.budget.limit,
        Limit::Tokens(50)
    );
    l.reserve(&scope(), "call", "dispatch", 40, 10).unwrap();
    assert_eq!(
        l.agent_account(OWNER, "a1", Period::Lifetime, 10).unwrap(),
        (0, 40)
    );
    assert!(matches!(
        l.reserve(&scope(), "second", "dispatch2", 20, 10),
        Err(Error::Budget)
    ));
    l.settle(&scope(), "call", &usage(25)).unwrap();
    l.set_agent_policy(OWNER, "a1", 1, &config(100, Period::Lifetime))
        .unwrap();
    drop(l);
    let l = Ledger::open(path, OWNER).unwrap();
    assert_eq!(l.agent_model_profile(OWNER, "a1").unwrap(), Some(model));
    assert_eq!(l.agent_policy(OWNER, "a1").unwrap().revision, 2);
    assert_eq!(
        l.account(&scope(), Layer::Agent, Period::Lifetime, 10)
            .unwrap(),
        (25, 0)
    );
}
fn scope() -> Scope {
    Scope {
        agent: "a1".into(),
        binding: "b1".into(),
        room: "!r:example.org".into(),
        requester: "@bob:example.org".into(),
        thread: "main".into(),
    }
}
fn config(n: u64, period: Period) -> Policy {
    Policy {
        budget: Budget {
            limit: Limit::Tokens(n),
            period,
        },
        requests: RequestPolicy::Allow,
        high_risk: ToolPolicy::AskOwner,
    }
}
fn setup(path: &Path) -> Ledger {
    let mut l = Ledger::open(path, OWNER).unwrap();
    l.register_binding(OWNER, &scope()).unwrap();
    l.set_policy(
        OWNER,
        &scope(),
        Layer::Room,
        0,
        &config(100, Period::UtcDay),
    )
    .unwrap();
    l
}
fn usage(n: u64) -> Usage {
    Usage {
        input: n,
        output: 0,
        cached_input: 0,
        reasoning_output: 0,
        accounting_version: "codex/v1-input-output".into(),
    }
}
#[test]
fn concurrent_call_reservations_do_not_overspend() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("local.db");
    setup(&path);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut l = Ledger::open(path, OWNER).unwrap();
                barrier.wait();
                l.reserve(&scope(), &format!("c{i}"), "d", 30, 10).is_ok()
            })
        })
        .collect();
    let successes = threads
        .into_iter()
        .filter_map(|t| t.join().ok())
        .filter(|ok| *ok)
        .count();
    assert_eq!(successes, 3);
    assert_eq!(
        Ledger::open(&path, OWNER)
            .unwrap()
            .account(&scope(), Layer::Room, Period::UtcDay, 10)
            .unwrap(),
        (0, 90)
    );
}
#[test]
fn requester_limits_and_cross_room_context_are_independent() {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = setup(&tmp.path().join("local.db"));
    let s = scope();
    l.set_policy(
        OWNER,
        &s,
        Layer::Requester,
        0,
        &config(10, Period::Lifetime),
    )
    .unwrap();
    assert!(matches!(
        l.reserve(&s, "c", "d", 11, 10),
        Err(Error::Budget)
    ));
    let mut other = s.clone();
    other.requester = "@charlie:example.org".into();
    assert_eq!(
        l.reserve(&other, "c2", "d", 40, 10).unwrap(),
        Reservation::New
    );
    l.set_context_session(&s, "session-a").unwrap();
    assert_eq!(l.context_session(&other).unwrap(), None);
    other.binding = "b2".into();
    other.room = "!r2:example.org".into();
    l.register_binding(OWNER, &other).unwrap();
    assert_eq!(l.context_session(&other).unwrap(), None);
    other.thread = "thread2".into();
    assert_eq!(l.context_session(&other).unwrap(), None);
    other.binding = s.binding;
    assert!(matches!(
        l.context_session(&other),
        Err(Error::Unauthorized)
    ));
}
#[test]
fn restart_unknown_retains_hold_across_window_and_settlement_is_once() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("local.db");
    let mut l = setup(&path);
    l.reserve(&scope(), "c", "d", 80, 10).unwrap();
    drop(l);
    let mut l = Ledger::open(path, OWNER).unwrap();
    assert_eq!(l.recover_interrupted(OWNER).unwrap(), 1);
    assert!(matches!(
        l.reserve(&scope(), "new", "d", 10, 86410),
        Err(Error::Unknown)
    ));
    assert_eq!(
        l.account(&scope(), Layer::Room, Period::UtcDay, 10)
            .unwrap(),
        (0, 80)
    );
    l.settle(&scope(), "c", &usage(60)).unwrap();
    l.settle(&scope(), "c", &usage(60)).unwrap();
    assert!(matches!(
        l.settle(&scope(), "c", &usage(61)),
        Err(Error::Conflict)
    ));
    assert_eq!(
        l.account(&scope(), Layer::Room, Period::UtcDay, 10)
            .unwrap(),
        (60, 0)
    );
    assert_eq!(
        l.account(&scope(), Layer::Room, Period::UtcDay, 86410)
            .unwrap(),
        (0, 0)
    );
    assert_eq!(
        l.reserve(&scope(), "c", "d", 80, 86410).unwrap(),
        Reservation::AlreadySettled(usage(60))
    );
    assert!(matches!(
        l.reserve(&scope(), "c", "other", 80, 10),
        Err(Error::Conflict)
    ));
}
#[test]
fn tools_require_owner_parameter_bound_single_use_confirmation_and_current_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = setup(&tmp.path().join("local.db"));
    let s = scope();
    l.set_policy(OWNER, &s, Layer::Agent, 0, &config(1000, Period::Lifetime))
        .unwrap();
    l.set_policy(
        OWNER,
        &s,
        Layer::Requester,
        0,
        &config(100, Period::Lifetime),
    )
    .unwrap();
    l.reserve(&s, "tool-call", "d", 1, 10).unwrap();
    let p = ToolProposal {
        scope: s.clone(),
        dispatch: "d".into(),
        tool: "shell".into(),
        arguments: serde_json::json!({"command":"pwd"}),
        canonical_directory: "/tmp/task".into(),
        risk: "high".into(),
        policy_revision: [1, 1, 1],
        expires: 100,
    };
    assert!(matches!(
        l.approve_tool("@mallory:example.org", &p, 10),
        Err(Error::Unauthorized)
    ));
    l.approve_tool(OWNER, &p, 10).unwrap();
    let mut changed = p.clone();
    changed.arguments = serde_json::json!({"command":"rm -rf /"});
    assert!(matches!(l.authorize_tool(&changed, 10), Err(Error::Denied)));
    l.authorize_tool(&p, 10).unwrap();
    assert!(matches!(l.authorize_tool(&p, 10), Err(Error::Denied)));
    let mut p2 = p.clone();
    p2.dispatch = "d2".into();
    l.reserve(&s, "tool-call2", "d2", 1, 10).unwrap();
    l.approve_tool(OWNER, &p2, 10).unwrap();
    l.set_policy(OWNER, &s, Layer::Room, 1, &config(100, Period::UtcDay))
        .unwrap();
    assert!(matches!(l.authorize_tool(&p2, 10), Err(Error::Denied)));
}
#[test]
fn request_ask_owner_cannot_be_reused_by_other_requester_or_after_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = setup(&tmp.path().join("local.db"));
    let s = scope();
    let mut p = config(100, Period::Lifetime);
    p.requests = RequestPolicy::AskOwner;
    l.set_policy(OWNER, &s, Layer::Requester, 0, &p).unwrap();
    assert!(matches!(
        l.reserve(&s, "c", "d", 10, 10),
        Err(Error::Denied)
    ));
    l.approve_request(OWNER, &s, "d", 100, 10).unwrap();
    l.reserve(&s, "c", "d", 10, 10).unwrap();
    assert!(matches!(
        l.reserve(&s, "c2", "d2", 10, 10),
        Err(Error::Denied)
    ));
    l.set_policy(OWNER, &s, Layer::Requester, 1, &p).unwrap();
    assert!(matches!(
        l.reserve(&s, "c3", "d", 10, 10),
        Err(Error::Denied)
    ));
}
#[test]
fn foreign_schema_owner_and_scope_mismatch_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("legacy.db");
    Connection::open(&path)
        .unwrap()
        .execute("CREATE TABLE fleet(id TEXT)", [])
        .unwrap();
    assert!(matches!(
        Ledger::open(path, OWNER),
        Err(Error::ForeignDatabase)
    ));
    let path = tmp.path().join("local.db");
    let mut l = setup(&path);
    assert!(matches!(
        Ledger::open(path, "@mallory:example.org"),
        Err(Error::Unauthorized)
    ));
    let mut s = scope();
    s.agent = "other".into();
    assert!(matches!(
        l.register_binding(OWNER, &s),
        Err(Error::Unauthorized)
    ));
    assert!(
        l.db.execute("UPDATE identity SET owner='@mallory:example.org'", [])
            .is_err()
    );
    assert!(l.db.execute("DELETE FROM bindings", []).is_err());
}
#[test]
fn zero_unset_overflow_and_provider_breach_fail_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = setup(&tmp.path().join("local.db"));
    let s = scope();
    let mut p = config(0, Period::UtcDay);
    l.set_policy(OWNER, &s, Layer::Room, 1, &p).unwrap();
    assert!(matches!(l.reserve(&s, "c", "d", 1, 10), Err(Error::Budget)));
    p.budget.limit = Limit::Unset;
    l.set_policy(OWNER, &s, Layer::Room, 2, &p).unwrap();
    assert!(matches!(l.reserve(&s, "c", "d", 1, 10), Err(Error::Budget)));
    p.budget.limit = Limit::Unlimited;
    l.set_policy(OWNER, &s, Layer::Room, 3, &p).unwrap();
    assert!(l.reserve(&s, "over", "d", u64::MAX, 10).is_err());
    l.reserve(&s, "c", "d", 10, 10).unwrap();
    l.settle(&s, "c", &usage(11)).unwrap();
    assert_eq!(
        l.account(&s, Layer::Room, Period::UtcDay, 10).unwrap(),
        (11, 0)
    );
    assert!(matches!(
        l.reserve(&s, "next", "d", 1, 86410),
        Err(Error::Unknown)
    ));
}
#[test]
fn policy_widening_does_not_upgrade_an_existing_dispatch() {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = setup(&tmp.path().join("local.db"));
    let s = scope();
    l.set_policy(OWNER, &s, Layer::Agent, 0, &config(1000, Period::Lifetime))
        .unwrap();
    let mut p = config(20, Period::Lifetime);
    p.high_risk = ToolPolicy::Deny;
    l.set_policy(OWNER, &s, Layer::Requester, 0, &p).unwrap();
    l.reserve(&s, "c", "old", 10, 10).unwrap();
    p.budget.limit = Limit::Unlimited;
    p.high_risk = ToolPolicy::AskOwner;
    l.set_policy(OWNER, &s, Layer::Requester, 1, &p).unwrap();
    assert!(matches!(
        l.reserve(&s, "c2", "old", 15, 10),
        Err(Error::Budget)
    ));
    l.reserve(&s, "c3", "new", 15, 10).unwrap();
    let proposal = ToolProposal {
        scope: s,
        dispatch: "old".into(),
        tool: "shell".into(),
        arguments: serde_json::json!({}),
        canonical_directory: "/tmp/task".into(),
        risk: "high".into(),
        policy_revision: [1, 1, 2],
        expires: 100,
    };
    assert!(matches!(
        l.approve_tool(OWNER, &proposal, 10),
        Err(Error::Denied)
    ));
}
#[test]
fn local_model_profile_contains_only_a_keychain_reference() {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = setup(&tmp.path().join("local.db"));
    let s = scope();
    let mut profile = ModelProfile {
        model: "codex".into(),
        credential_ref: "raw-secret".into(),
        workspace_root: "/tmp/task".into(),
    };
    assert!(l.set_model_profile(OWNER, &s, &profile).is_err());
    profile.credential_ref = "keychain:hagency/codex/alice".into();
    l.set_model_profile(OWNER, &s, &profile).unwrap();
    assert_eq!(l.model_profile(&s).unwrap(), Some(profile));
}
#[test]
fn concurrent_tool_consumers_cannot_reuse_one_confirmation() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("local.db");
    let mut l = setup(&path);
    let s = scope();
    l.set_policy(OWNER, &s, Layer::Agent, 0, &config(1000, Period::Lifetime))
        .unwrap();
    l.set_policy(
        OWNER,
        &s,
        Layer::Requester,
        0,
        &config(100, Period::Lifetime),
    )
    .unwrap();
    l.reserve(&s, "c", "d", 1, 10).unwrap();
    let proposal = ToolProposal {
        scope: s,
        dispatch: "d".into(),
        tool: "shell".into(),
        arguments: serde_json::json!({"command":"pwd"}),
        canonical_directory: "/tmp/task".into(),
        risk: "high".into(),
        policy_revision: [1, 1, 1],
        expires: 100,
    };
    l.approve_tool(OWNER, &proposal, 10).unwrap();
    drop(l);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(6));
    let handles: Vec<_> = (0..6)
        .map(|_| {
            let path = path.clone();
            let p = proposal.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut l = Ledger::open(path, OWNER).unwrap();
                barrier.wait();
                l.authorize_tool(&p, 10).is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|ok| *ok)
            .count(),
        1
    );
}

#[test]
fn unresolved_provider_charge_stops_late_tool_execution() {
    let tmp = tempfile::tempdir().unwrap();
    let mut l = setup(&tmp.path().join("local.db"));
    let s = scope();
    for layer in [Layer::Agent, Layer::Requester] {
        l.set_policy(OWNER, &s, layer, 0, &config(100, Period::Lifetime))
            .unwrap();
    }
    l.reserve(&s, "c", "d", 1, 10).unwrap();
    let proposal = ToolProposal {
        scope: s.clone(),
        dispatch: "d".into(),
        tool: "shell".into(),
        arguments: serde_json::json!({"command":"pwd"}),
        canonical_directory: "/tmp/task".into(),
        risk: "high".into(),
        policy_revision: [1, 1, 1],
        expires: 100,
    };
    l.approve_tool(OWNER, &proposal, 10).unwrap();
    l.mark_unknown(&s, "c").unwrap();
    assert!(matches!(
        l.authorize_tool(&proposal, 11),
        Err(Error::Unknown)
    ));
    l.settle(&s, "c", &usage(1)).unwrap();
    l.authorize_tool(&proposal, 12).unwrap();
}

#[test]
fn exact_request_confirmation_cannot_authorize_changed_policy_or_other_requester() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("local.db");
    let mut ledger = setup(&path);
    let s = scope();
    let mut ask = config(100, Period::Lifetime);
    ask.requests = RequestPolicy::AskOwner;
    ledger
        .set_policy(OWNER, &s, Layer::Requester, 0, &ask)
        .unwrap();
    assert_eq!(
        ledger.request_disposition(&s, "dispatch").unwrap(),
        RequestPolicy::AskOwner
    );
    let revisions = ledger.policy_snapshot(&s).unwrap().map(|v| v.revision);
    let mut concurrent = Ledger::open(&path, OWNER).unwrap();
    ask.budget.limit = Limit::Tokens(200);
    concurrent
        .set_policy(OWNER, &s, Layer::Requester, 1, &ask)
        .unwrap();
    assert!(matches!(
        ledger.approve_request_exact(OWNER, &s, "dispatch", revisions, 100, 10),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        ledger.reserve(&s, "call", "dispatch", 10, 10),
        Err(Error::Denied)
    ));
    let revisions = ledger.policy_snapshot(&s).unwrap().map(|v| v.revision);
    ledger
        .approve_request_exact(OWNER, &s, "dispatch", revisions, 100, 10)
        .unwrap();
    ledger.reserve(&s, "call", "dispatch", 10, 10).unwrap();
    let mut other = s.clone();
    other.requester = "@other:example.org".into();
    assert!(matches!(
        ledger.reserve(&other, "other-call", "dispatch", 10, 10),
        Err(Error::Unauthorized)
    ));
    ask.requests = RequestPolicy::Deny;
    concurrent
        .set_policy(OWNER, &s, Layer::Requester, 2, &ask)
        .unwrap();
    assert_eq!(
        ledger.request_disposition(&s, "dispatch").unwrap(),
        RequestPolicy::Deny
    );
    let revisions = ledger.policy_snapshot(&s).unwrap().map(|v| v.revision);
    assert!(matches!(
        ledger.approve_request_exact(OWNER, &s, "dispatch", revisions, 100, 10),
        Err(Error::Denied)
    ));
}

#[test]
fn profile_identity_survives_restore_and_rejects_same_mxid_foreign_profile_copy() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("a.sqlite");
    let copy = temp.path().join("b.sqlite");
    let a = "a".repeat(64);
    let b = "b".repeat(64);
    let mut ledger = Ledger::open_scoped(&original, OWNER, &a).unwrap();
    ledger.register_binding(OWNER, &scope()).unwrap();
    ledger
        .set_policy(
            OWNER,
            &scope(),
            Layer::Room,
            0,
            &config(40, Period::Lifetime),
        )
        .unwrap();
    drop(ledger);
    std::fs::copy(&original, &copy).unwrap();
    assert!(matches!(
        Ledger::open_scoped(&copy, OWNER, &b),
        Err(Error::Unauthorized)
    ));
    let restored = Ledger::open_scoped(&copy, OWNER, &a).unwrap();
    assert_eq!(
        restored.policy_snapshot(&scope()).unwrap()[1]
            .policy
            .budget
            .limit,
        Limit::Tokens(40)
    );
    assert!(
        restored
            .db
            .execute("UPDATE profile_identity SET digest=?", [b])
            .is_err()
    );
    assert!(
        restored
            .db
            .execute("DELETE FROM profile_identity", [])
            .is_err()
    );
    let unscoped = temp.path().join("unscoped.sqlite");
    let mut old = Ledger::open(&unscoped, OWNER).unwrap();
    old.register_binding(OWNER, &scope()).unwrap();
    drop(old);
    assert!(matches!(
        Ledger::open_scoped(&unscoped, OWNER, &a),
        Err(Error::ForeignDatabase)
    ));
}

#[test]
fn simultaneous_fresh_profile_openers_share_one_initialized_schema() {
    let tmp = tempfile::tempdir().unwrap();
    for attempt in 0..12 {
        let path = tmp.path().join(format!("fresh-{attempt}.sqlite"));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    Ledger::open_scoped(path, OWNER, &"a".repeat(64)).map(|_| ())
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap().unwrap();
        }
        let l = Ledger::open_scoped(&path, OWNER, &"a".repeat(64)).unwrap();
        assert_eq!(
            l.db.query_row("SELECT count(*) FROM identity", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            l.db.query_row("SELECT count(*) FROM profile_identity", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(matches!(
            Ledger::open_scoped(&path, OWNER, &"b".repeat(64)),
            Err(Error::Unauthorized)
        ));
    }
}
