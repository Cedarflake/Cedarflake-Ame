use super::super::runtime_test_support::*;
use super::super::*;
use super::*;

#[test]
fn stopping_bounded_recovery_signals_and_joins_the_background_worker() {
    let factory = crate::adapters::production_library_change_source_factory();
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        while !worker_cancelled.load(Ordering::Acquire) {
            thread::yield_now();
        }
        let _ = sender.send(Ok(RecoveryTaskOutcome::Authoritative(
            AuthoritativeLibraryChangeReport::default(),
        )));
    });
    let mut production = ProductionSynchronization {
        runtime: LibrarySynchronizationRuntime::new_erased(factory),
        catalog_session: None,
        poll_catalog: PollCatalogOwner::default(),
        persistent_change_journal_factory: None,
        _persistent_change_journal: test_live_only_connection(),
        persistent_change_journal_opened: true,
        persistent_change_journal_caller: None,
        journal: None,
        journal_status: JournalStatusOwner::default(),
        journal_lane: JournalLane::default(),
        live: None,
        live_root_cursor: None,
        recovery: Some(RecoveryTask::from_test_parts(
            "root-a".to_owned(),
            RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision: 0,
            },
            Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
            Arc::clone(&cancelled),
            receiver,
            Some(worker),
        )),
        recovery_sources: RecoverySources::default(),
        metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
        recovery_lane: RecoveryLane::default(),
        legacy_automatic_scans_retired: false,
        inventory_cleanup: InventoryCleanupOwner::default(),
        root_location: RootLocationRecoveryOwner::default(),
        is_stopping: false,
        stop_requested: Arc::new(AtomicBool::new(false)),
        core_stopped: false,
        journal_closed: false,
        journal_close: None,
        drain_panic: None,
    };

    production.stop_recovery().expect("stop bounded recovery");

    assert!(cancelled.load(Ordering::Acquire));
    assert!(production.recovery.is_none());
}

#[test]
fn recovery_stop_timeout_retains_worker_ownership_until_a_later_join() {
    let factory = crate::adapters::production_library_change_source_factory();
    let cancelled = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let worker_release = Arc::clone(&release);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        while !worker_release.load(Ordering::Acquire) {
            thread::yield_now();
        }
        let _ = sender.send(Ok(RecoveryTaskOutcome::Authoritative(
            AuthoritativeLibraryChangeReport::default(),
        )));
    });
    let mut production = ProductionSynchronization {
        runtime: LibrarySynchronizationRuntime::new_erased(factory),
        catalog_session: None,
        poll_catalog: PollCatalogOwner::default(),
        persistent_change_journal_factory: None,
        _persistent_change_journal: test_live_only_connection(),
        persistent_change_journal_opened: true,
        persistent_change_journal_caller: None,
        journal: None,
        journal_status: JournalStatusOwner::default(),
        journal_lane: JournalLane::default(),
        live: None,
        live_root_cursor: None,
        recovery: Some(RecoveryTask::from_test_parts(
            "root-a".to_owned(),
            RecoveryTaskKind::BoundedAuthoritative {
                continuity_revision: 0,
            },
            Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
            Arc::clone(&cancelled),
            receiver,
            Some(worker),
        )),
        recovery_sources: RecoverySources::default(),
        metadata_inventory_page_entries: METADATA_INVENTORY_WORK_PAGE_ENTRIES,
        recovery_lane: RecoveryLane::default(),
        legacy_automatic_scans_retired: false,
        inventory_cleanup: InventoryCleanupOwner::default(),
        root_location: RootLocationRecoveryOwner::default(),
        is_stopping: true,
        stop_requested: Arc::new(AtomicBool::new(true)),
        core_stopped: false,
        journal_closed: false,
        journal_close: None,
        drain_panic: None,
    };

    let error = production
        .stop_recovery_with_timeout(Duration::from_millis(10))
        .expect_err("uncooperative worker must exceed the short stop window");

    assert_eq!(error.code, "authoritative_recovery_stop_timeout");
    assert!(cancelled.load(Ordering::Acquire));
    assert!(production.recovery.is_some());
    release.store(true, Ordering::Release);
    production
        .stop_recovery_with_timeout(Duration::from_secs(1))
        .expect("later stop joins retained worker");
    assert!(production.recovery.is_none());
}
