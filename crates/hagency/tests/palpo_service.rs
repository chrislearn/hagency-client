#[path = "palpo_service/fixture.rs"]
mod fixture;
#[allow(dead_code)] // Reuse the existing bounded real HTTPS peer and trust root.
#[path = "../../hagency-palpo/tests/common/mod.rs"]
mod peer;
use fixture::*;
use hagency::bootstrap::{Bootstrap, Options};
use hagency_palpo::CancellationToken;
use std::time::Duration;

#[tokio::test]
async fn native_palpo_service_cancel_custody() {
    let mut f = Fixture::new(true, true).await;
    let mut bootstrap = Bootstrap::open_with_options(
        &f.state,
        f.address,
        16,
        Options {
            development_driver: false,
            agent_driver: false,
            palpo_transport: true,
        },
    )
    .unwrap();
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    let serving = tokio::spawn(Box::pin(async move {
        let result = bootstrap.serve(&signal).await;
        (result, bootstrap)
    }));
    let held = f.publication().await;
    check(&held, 1);
    // Hold one original request from each inbound lane too. Neither can issue
    // another poll while its own request is unacknowledged by this HTTPS peer.
    let matrix_or_work = f.fake.next().await;
    let other_lane = f.fake.next().await;
    assert!(matrix_or_work.target.contains("/poll?"));
    assert!(other_lane.target.contains("/poll?"));
    assert_ne!(matrix_or_work.target, other_lane.target);
    let original = f.pending();
    assert_eq!(original.3, "unknown");
    cancel.cancel();
    let (result, mut original_owner) = tokio::time::timeout(Duration::from_secs(10), serving)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(original_owner.close().await, Ok(()));
    assert_eq!(f.pending(), original); // cancellation cannot acknowledge sent bytes
    f.reopen();
    // Sequencing point: `serving` completed above, so every transport loop
    // has unwound and no further request can be initiated. Admissions that
    // started parsing before cancellation land inside the derived window;
    // the counter proves none of them is a new send after this point.
    let settled = f.fake.requests();
    f.fake.quiesced(settled).await;
    drop(held);
    drop(matrix_or_work);
    drop(other_lane);
    f.fake.close().await;
}
