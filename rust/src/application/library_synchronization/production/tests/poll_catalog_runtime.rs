use super::*;

#[test]
fn retained_poll_catalog_close_obeys_the_epoch_deadline_and_blocks_replacement() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ame.sqlite3");
    let runtime = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    drop(runtime.poll_catalog.checkout(&path).unwrap());
    let owner = runtime.poll_catalog.clone();
    let (entered_sender, entered) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    owner.before_close(move || {
        entered_sender.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(5)).unwrap();
    });
    let registry = ready_test_registry(runtime);
    let started = Instant::now();
    let result = stop_runtime_with_timeout(&registry, Duration::from_millis(100));
    let elapsed = started.elapsed();
    let reached_close = entered.recv_timeout(Duration::from_secs(1));
    let counts_during_close = owner.connection_counts();
    let is_draining = matches!(
        registry.state.lock().unwrap().entry,
        SynchronizationRuntimeEntry::Draining { .. }
    );
    let replacement = directory.path().join("retired.sqlite3");
    let blocked_replacement = std::fs::rename(&path, &replacement);
    // Release before assertions so a failed assertion cannot strand a fixture worker.
    release.send(()).unwrap();
    wait_for_retained_journal_close_worker(&registry, Duration::from_secs(2));
    let reaped = stop_runtime(&registry);

    assert_eq!(
        result.unwrap_err().code,
        "library_synchronization_stop_timeout"
    );
    assert!(elapsed < Duration::from_millis(500));
    assert!(reached_close.is_ok());
    assert_eq!(counts_during_close, (1, 0));
    assert!(is_draining);
    assert!(blocked_replacement.is_err());
    reaped.unwrap();
    assert_eq!(owner.connection_counts(), (1, 1));
    assert!(matches!(
        registry.state.lock().unwrap().entry,
        SynchronizationRuntimeEntry::Empty
    ));
    std::fs::rename(&path, &replacement).unwrap();
}

#[test]
fn journal_close_failure_still_retires_the_poll_catalog_once() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ame.sqlite3");
    let mut runtime = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
            close_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        })),
    );
    drop(runtime.poll_catalog.checkout(&path).unwrap());
    for _ in 0..2 {
        let error = runtime
            .finish_stopping_until(Instant::now() + Duration::from_secs(1))
            .unwrap_err();
        assert_eq!(error.code, "persistent_change_journal_close_failed");
        assert_eq!(runtime.poll_catalog.connection_counts(), (1, 1));
    }
    std::fs::rename(path, directory.path().join("retired.sqlite3")).unwrap();
}
