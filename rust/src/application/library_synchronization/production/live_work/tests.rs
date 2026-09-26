use std::time::Duration;

use super::*;

#[test]
fn stop_timeout_retains_the_same_worker_until_its_original_lifetime_finishes() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (release, released) = mpsc::sync_channel(1);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        released.recv().expect("release owned worker");
        sender
            .send(LiveWorkOutcome::default())
            .expect("owned receiver");
    });
    let mut task = LiveTask::from_test_parts(
        "owned-root".to_owned(),
        Arc::clone(&cancelled),
        receiver,
        worker,
    );
    let error = task
        .finish_stopping_until(Instant::now())
        .expect_err("expired deadline");
    assert_eq!(error.code, "live_reconciliation_stop_timeout");
    assert!(cancelled.load(Ordering::Acquire));
    assert!(task.worker.is_some());
    release.send(()).expect("release original worker");
    task.finish_stopping_until(Instant::now() + Duration::from_secs(1))
        .expect("retire original worker");
    assert!(task.worker.is_none());
}

#[test]
fn disconnected_worker_reports_failure_and_is_joined() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || drop(sender));
    let mut task = LiveTask::from_test_parts(
        "disconnected-root".to_owned(),
        Arc::new(AtomicBool::new(false)),
        receiver,
        worker,
    );
    let deadline = Instant::now() + Duration::from_secs(1);
    let outcome = loop {
        if let Some(outcome) = task.poll() {
            break outcome;
        }
        assert!(Instant::now() < deadline, "owned worker did not finish");
        thread::yield_now();
    };
    assert_eq!(
        outcome.failure.expect("failure").code,
        "live_reconciliation_worker_disconnected"
    );
    assert!(task.worker.is_none());
}
