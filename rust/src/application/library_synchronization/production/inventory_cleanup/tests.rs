use std::sync::atomic::AtomicUsize;

use super::*;

mod production_debt;

fn completed(has_more: bool) -> Result<InventoryCleanupOutcome, ScanError> {
    Ok(InventoryCleanupOutcome::Completed(
        MetadataInventoryCleanupReport {
            has_more,
            ..MetadataInventoryCleanupReport::default()
        },
    ))
}

fn await_worker(owner: &mut InventoryCleanupOwner) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while owner.task.as_ref().is_some_and(|task| {
        task.worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
    }) && Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(1));
    }
    assert!(
        owner
            .task
            .as_ref()
            .is_some_and(|task| { task.worker.as_ref().is_some_and(JoinHandle::is_finished) }),
        "bounded cleanup fixture could not confirm worker retirement",
    );
}

fn finish(owner: &mut InventoryCleanupOwner) {
    owner
        .finish_stopping_until(Instant::now() + Duration::from_secs(3))
        .expect("confirm cleanup owner retirement before fixture disposal");
}

#[test]
fn bounded_cleanup_advances_once_per_poll_then_rechecks_idle_storage() {
    let mut owner = InventoryCleanupOwner::default();
    let cancelled = Arc::new(AtomicBool::new(false));
    let now = Instant::now();
    let count = Arc::new(AtomicUsize::new(0));
    let first_count = Arc::clone(&count);
    owner
        .poll_with(now, Arc::clone(&cancelled), move |_| {
            first_count.fetch_add(1, Ordering::Relaxed);
            completed(true)
        })
        .expect("first bounded batch");
    await_worker(&mut owner);
    assert_eq!(
        count.load(Ordering::Relaxed),
        1,
        "has_more cannot drain inside the worker"
    );
    let second_count = Arc::clone(&count);
    owner
        .poll_with(now, Arc::clone(&cancelled), move |_| {
            second_count.fetch_add(1, Ordering::Relaxed);
            completed(false)
        })
        .expect("next poll owns the next batch");
    await_worker(&mut owner);
    owner.reap_finished(now).expect("empty result");
    let idle_count = Arc::clone(&count);
    owner
        .poll_with(now, Arc::clone(&cancelled), move |_| {
            idle_count.fetch_add(1, Ordering::Relaxed);
            completed(false)
        })
        .expect("idle cooldown");
    assert!(owner.task.is_none());
    assert_eq!(count.load(Ordering::Relaxed), 2);
    let future_count = Arc::clone(&count);
    owner
        .poll_with(now + IDLE_CHECK_INTERVAL, cancelled, move |_| {
            future_count.fetch_add(1, Ordering::Relaxed);
            completed(false)
        })
        .expect("new terminal work and newly expired summaries are checked again");
    await_worker(&mut owner);
    finish(&mut owner);
    assert_eq!(count.load(Ordering::Relaxed), 3);
}

#[test]
fn bounded_cleanup_contention_defers_without_consuming_the_next_attempt() {
    let mut owner = InventoryCleanupOwner::default();
    let cancelled = Arc::new(AtomicBool::new(false));
    let now = Instant::now();
    owner
        .poll_with(now, Arc::clone(&cancelled), |_| {
            Err(ScanError::new("catalog_database_busy", "owned writer"))
        })
        .expect("admit bounded attempt");
    await_worker(&mut owner);
    owner
        .reap_finished(now)
        .expect("busy is a deferred attempt");
    assert_eq!(owner.next_check, Some(now + RETRY_INITIAL_DELAY));
    owner
        .poll_with(now, Arc::clone(&cancelled), |_| completed(false))
        .expect("no busy retry loop");
    assert!(owner.task.is_none());
    owner
        .poll_with(now + RETRY_INITIAL_DELAY, cancelled, |_| completed(false))
        .expect("retry the persisted debt");
    await_worker(&mut owner);
    owner
        .reap_finished(now + RETRY_INITIAL_DELAY)
        .expect("successful retry");
    assert!(owner.retry_delay.is_none());
    finish(&mut owner);
}

#[test]
fn bounded_cleanup_panic_is_reported_and_does_not_lose_retry_ownership() {
    let mut owner = InventoryCleanupOwner::default();
    let cancelled = Arc::new(AtomicBool::new(false));
    let now = Instant::now();
    owner
        .poll_with(now, Arc::clone(&cancelled), |_| {
            panic!("controlled cleanup panic")
        })
        .expect("start controlled failure");
    await_worker(&mut owner);
    let error = owner
        .reap_finished(now)
        .expect_err("panic remains observable");
    assert_eq!(error.code, "metadata_inventory_cleanup_worker_panicked");
    assert!(owner.task.is_none());
    owner
        .poll_with(now + RETRY_INITIAL_DELAY, cancelled, |_| completed(false))
        .expect("later attempt remains owned");
    await_worker(&mut owner);
    finish(&mut owner);
}

#[test]
fn bounded_cleanup_stop_retains_the_worker_under_the_original_deadline() {
    let mut owner = InventoryCleanupOwner::default();
    let cancelled = Arc::new(AtomicBool::new(false));
    let (entered_sender, entered_receiver) = mpsc::sync_channel(1);
    let (release_sender, release_receiver) = mpsc::sync_channel(1);
    owner
        .poll_with(Instant::now(), Arc::clone(&cancelled), move |cancelled| {
            let _ = entered_sender.send(());
            release_receiver
                .recv_timeout(Duration::from_secs(3))
                .map_err(|_| {
                    ScanError::new(
                        "cleanup_fixture_release_timeout",
                        "fixture release was not confirmed",
                    )
                })?;
            assert!(cancelled.load(Ordering::Acquire));
            completed(true)
        })
        .expect("start gated cleanup");
    entered_receiver
        .recv_timeout(Duration::from_secs(3))
        .expect("worker entered");
    let deadline = Instant::now();
    let first_stop = owner.finish_stopping_until(deadline);
    let retained = owner.task.is_some();
    let second_stop = owner.finish_stopping_until(deadline);
    release_sender.send(()).expect("release owned worker");
    await_worker(&mut owner);
    owner
        .finish_stopping_until(deadline)
        .expect("retire a finished worker without a new deadline");
    assert_eq!(
        first_stop.expect_err("deadline expired").code,
        "metadata_inventory_cleanup_stop_timeout"
    );
    assert_eq!(
        second_stop.expect_err("same deadline remains expired").code,
        "metadata_inventory_cleanup_stop_timeout"
    );
    assert!(retained);
    assert!(cancelled.load(Ordering::Acquire));
    owner
        .poll_with(Instant::now(), Arc::new(AtomicBool::new(false)), |_| {
            completed(true)
        })
        .expect("stopped owner cannot restart");
    assert!(
        owner.task.is_none(),
        "shutdown does not drain the remaining debt"
    );
}

#[test]
fn bounded_cleanup_pre_cancelled_epoch_never_starts_work() {
    let mut owner = InventoryCleanupOwner::default();
    owner
        .poll_with(Instant::now(), Arc::new(AtomicBool::new(true)), |_| {
            completed(false)
        })
        .expect("late scheduling observes the epoch stop fence");
    assert!(owner.task.is_none());
    finish(&mut owner);
}
