use super::*;

#[test]
fn deferred_live_reservation_prevents_recovery_relay_starvation() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let owner = admission.acquire(LibraryChangeLane::Recovery);
    let (order_tx, order_rx) = mpsc::channel();
    let relay = GatedWriter::spawn(&admission, LibraryChangeLane::Recovery, 1, order_tx);
    let mut live = admission.reserve(LibraryChangeLane::Live);
    assert!(live.try_acquire().is_none());
    drop(owner);
    relay
        .release
        .send(())
        .expect("wake the queued recovery relay");
    let premature = order_rx.recv_timeout(Duration::from_millis(50));
    let permit = live.try_acquire();
    let admitted = permit.is_some();
    drop(permit);
    let after_rollback = order_rx.recv_timeout(Duration::from_millis(50));
    let retry = live.try_acquire();
    let retried = retry.is_some();
    drop(retry);
    drop(live);
    let recovered = order_rx.recv_timeout(Duration::from_secs(5));
    let joined = join_writer(relay.worker);
    assert!(
        premature.is_err(),
        "P2 bypassed a deferred live reservation"
    );
    assert!(admitted);
    assert!(
        after_rollback.is_err(),
        "a rolled-back attempt lost its P0 position"
    );
    assert!(retried);
    assert_eq!(recovered.expect("P2 progresses after the plan retires"), 1);
    assert!(joined);
    assert_idle(&admission);
}

#[test]
fn reservations_preserve_fifo_without_blocking_interactive_priority() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let mut first = admission.reserve(LibraryChangeLane::Live);
    let mut second = admission.reserve(LibraryChangeLane::Live);
    assert!(second.try_acquire().is_none());
    let interactive = admission
        .acquire_user_interactive_for(Duration::ZERO)
        .expect("interactive priority");
    assert!(first.try_acquire().is_none());
    drop(interactive);
    let first_permit = first.try_acquire().expect("oldest live plan");
    assert!(second.try_acquire().is_none());
    drop(first_permit);
    assert!(second.try_acquire().is_none());
    drop(first);
    let second_permit = second.try_acquire().expect("next live plan");
    drop(second);
    assert!(
        admission.try_acquire(LibraryChangeLane::Recovery).is_none(),
        "retiring a position cannot revoke an active permit"
    );
    drop(second_permit);
    assert_idle(&admission);
}
