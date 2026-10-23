use super::*;
use crate::domain::LibraryChangeQueuePolicy;

struct FailingJournalSession {
    healthy: NoChangeJournalSession,
    fail: Arc<AtomicBool>,
    attempts: Arc<std::sync::atomic::AtomicUsize>,
}

impl PersistentChangeJournalSession for FailingJournalSession {
    fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        self.healthy.register_root(request)
    }
    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        self.attempts.fetch_add(1, Ordering::AcqRel);
        if self.fail.load(Ordering::Acquire) {
            Err(PersistentChangeJournalOperationError::TransportUnavailable)
        } else {
            self.healthy.query_journal(request)
        }
    }
    fn begin_read_range(
        &self,
        request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        self.healthy.begin_read_range(request)
    }
    fn begin_read_volume(
        &self,
        request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        self.healthy.begin_read_volume(request)
    }
    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        Ok(())
    }
}

fn runtime(
    fail: &Arc<AtomicBool>,
    attempts: &Arc<std::sync::atomic::AtomicUsize>,
) -> ProductionSynchronization {
    let count = || Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut runtime = fast_gap_runtime(
        QueuedSourceFactory::default(),
        PersistentChangeJournalConnection::Connected(Arc::new(FailingJournalSession {
            healthy: NoChangeJournalSession {
                register_count: count(),
                query_count: count(),
                shared_read_count: count(),
                close_count: count(),
                client_instances: Arc::new(Mutex::new(Vec::new())),
            },
            fail: Arc::clone(fail),
            attempts: Arc::clone(attempts),
        })),
    );
    runtime.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [22; 16],
    });
    runtime
}

fn inventory_count(fixture: &ProductionGapFixture) -> i64 {
    rusqlite::Connection::open(&fixture.storage.catalog_path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_runs",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

fn start_recovery(
    fixture: &ProductionGapFixture,
    kind: PersistentJournalRootFailureKind,
) -> crate::domain::LibraryChangeId {
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let checkpoint = catalog
        .load_persistent_journal_checkpoint(&fixture.root_id, LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();
    catalog
        .persist_persistent_journal_root_failure(
            &PersistentJournalRootFailure {
                root_id: fixture.root_id.clone(),
                root_generation: checkpoint.root_generation,
                kind,
                failure: PersistentJournalFailure {
                    code: "journal_trim".into(),
                    message: "Retained inventory from an older client".into(),
                },
                opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                    volume: checkpoint.volume,
                    root_file_reference: checkpoint.root_file_reference,
                    journal_id: checkpoint.journal_id,
                    next_usn: checkpoint.next_unread_usn,
                    protocol_version: checkpoint.protocol_version,
                    contract_version: checkpoint.contract_version,
                }),
            },
            2_000,
            LibraryChangeQueuePolicy::default(),
        )
        .unwrap()
        .unwrap()
}

#[test]
fn failed_closing_boundary_keeps_inventory_and_completes_without_journal_replay() {
    let fixture = ProductionGapFixture::new("fallback-closing", 18);
    seed_current_gap_journal(&fixture);
    let control = start_recovery(&fixture, PersistentJournalRootFailureKind::JournalTrim);
    let fail = Arc::new(AtomicBool::new(true));
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut production = runtime(&fail, &attempts);
    let completed = drive_gap_to_synchronized(&mut production, &fixture);
    assert_eq!(
        completed.roots[0].continuity,
        PersistentJournalContinuityState::LiveOnly
    );
    production.stop().unwrap();
    assert_eq!(attempts.load(Ordering::Acquire), 3);
    assert_eq!(inventory_count(&fixture), 1);
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let authority = catalog
        .load_metadata_inventory_recovery_authority(control)
        .unwrap()
        .unwrap();
    let run = catalog
        .load_metadata_inventory_run(&authority.run_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        run.status,
        crate::domain::MetadataInventoryRunStatus::Completed
    );
    assert_eq!(run.staged_entry_count, 18);
    assert!(
        catalog
            .load_persistent_journal_baselines()
            .unwrap()
            .is_empty()
    );
    fixture.assert_no_automatic_full_scan();
}

#[test]
fn permanent_journal_failure_finishes_offline_changes_and_retries_only_after_restart() {
    let fixture = ProductionGapFixture::new_with_media_baseline("independent-journal-fallback");
    seed_current_gap_journal(&fixture);
    std::fs::remove_file(fixture.source_root.join("removed.png")).unwrap();
    image::RgbImage::from_pixel(3, 4, image::Rgb([70, 100, 200]))
        .save(fixture.source_root.join("unchanged.png"))
        .unwrap();
    image::RgbImage::from_pixel(5, 6, image::Rgb([50, 70, 150]))
        .save(fixture.source_root.join("added.png"))
        .unwrap();
    let before: Vec<_> = ["unchanged.png", "added.png"]
        .into_iter()
        .map(|name| (name, std::fs::read(fixture.source_root.join(name)).unwrap()))
        .collect();
    let fail = Arc::new(AtomicBool::new(true));
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut production = runtime(&fail, &attempts);
    // Enter the real scheduler first; an empty queue alone must not end startup.
    for _ in 0..1000 {
        poll_runtime_with_storage(&mut production, &fixture.storage).unwrap();
        if attempts.load(Ordering::Acquire) >= 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let completed = drive_gap_to_synchronized(&mut production, &fixture);
    assert_eq!(
        completed.roots[0].continuity,
        PersistentJournalContinuityState::LiveOnly
    );
    assert_eq!(attempts.load(Ordering::Acquire), 1);
    assert_eq!(
        completed.roots[0].last_issue_code.as_deref(),
        Some(crate::domain::persistent_journal::METADATA_FALLBACK_CODE)
    );
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    for (name, dimensions) in [("unchanged.png", (3, 4)), ("added.png", (5, 6))] {
        let location = catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, name)
            .unwrap()
            .unwrap();
        assert_eq!((location.width, location.height), dimensions);
    }
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "removed.png")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint(&fixture.root_id, LibraryRootGeneration::initial())
            .unwrap()
            .unwrap()
            .next_unread_usn
            .value(),
        20
    );
    drop(catalog);
    for _ in 0..10 {
        poll_runtime_with_storage(&mut production, &fixture.storage).unwrap();
    }
    assert_eq!(attempts.load(Ordering::Acquire), 1);
    production.stop().unwrap();
    let count = inventory_count(&fixture);
    fail.store(false, Ordering::Release);
    let mut restarted = runtime(&fail, &attempts);
    let current = drive_gap_to_synchronized(&mut restarted, &fixture);
    assert_eq!(
        current.roots[0].continuity,
        PersistentJournalContinuityState::Current
    );
    restarted.stop().unwrap();
    assert!(attempts.load(Ordering::Acquire) > 1);
    assert_eq!(inventory_count(&fixture), count);
    for (name, bytes) in before {
        assert_eq!(
            std::fs::read(fixture.source_root.join(name)).unwrap(),
            bytes
        );
    }
    fixture.assert_no_automatic_full_scan();
}

#[test]
fn interrupted_inventory_checks_the_closed_interval_in_a_fresh_pass() {
    let fixture = ProductionGapFixture::new("fallback-interrupted", 0);
    seed_current_gap_journal(&fixture);
    let control = start_recovery(&fixture, PersistentJournalRootFailureKind::JournalTrim);
    let policy = LibraryChangeQueuePolicy::default();
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let root = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .unwrap()
        .unwrap();
    let mut reached_closing = false;
    for step in 0..8_i64 {
        let now = 3_000 + step * 1_000;
        let leased = catalog
            .lease_metadata_inventory_recovery(&fixture.root_id, root.root_generation, now, policy)
            .unwrap()
            .unwrap();
        let report =
            crate::application::metadata_inventory::process_leased_metadata_inventory_change(
                &mut catalog,
                &root,
                &leased,
                now,
                64,
                policy,
                &AtomicBool::new(false),
            )
            .unwrap();
        assert!(!report.inventory.is_complete);
        if report.inventory.awaiting_closing_boundary {
            reached_closing = true;
            break;
        }
    }
    assert!(
        reached_closing,
        "retained inventory must have enumerated before shutdown"
    );
    let authority = catalog
        .load_metadata_inventory_recovery_authority(control)
        .unwrap()
        .unwrap();
    let retained = catalog
        .load_metadata_inventory_run(&authority.run_id)
        .unwrap()
        .unwrap();
    assert!(retained.enumeration_complete);
    assert!(!retained.absence_authority);
    assert_eq!(inventory_count(&fixture), 1);
    drop(catalog);
    assert_eq!(
        start_recovery(
            &fixture,
            PersistentJournalRootFailureKind::AutomaticMetadataFallback(
                crate::domain::LibraryRecoveryAuthorityReason::JournalTrim,
            ),
        ),
        control
    );

    image::RgbImage::from_pixel(7, 8, image::Rgb([40, 90, 120]))
        .save(fixture.source_root.join("closed-interval.png"))
        .unwrap();
    let bytes = std::fs::read(fixture.source_root.join("closed-interval.png")).unwrap();
    let fail = Arc::new(AtomicBool::new(true));
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut production = runtime(&fail, &attempts);
    let completed = drive_gap_to_synchronized(&mut production, &fixture);
    assert_eq!(
        completed.roots[0].continuity,
        PersistentJournalContinuityState::LiveOnly
    );
    production.stop().unwrap();
    assert_eq!(attempts.load(Ordering::Acquire), 0);
    assert_eq!(inventory_count(&fixture), 2);
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let location = catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "closed-interval.png")
        .unwrap()
        .unwrap();
    assert_eq!((location.width, location.height), (7, 8));
    assert_eq!(
        std::fs::read(fixture.source_root.join("closed-interval.png")).unwrap(),
        bytes
    );
    fixture.assert_no_automatic_full_scan();
}

#[test]
fn absent_broker_uses_verification_at_each_startup_without_a_journal_wait() {
    let fixture = ProductionGapFixture::new("fallback-absent-broker", 0);
    seed_current_gap_journal(&fixture);
    for round in 1..=2 {
        if round == 2 {
            image::RgbImage::from_pixel(9, 10, image::Rgb([60, 90, 120]))
                .save(fixture.source_root.join("closed.png"))
                .unwrap();
        }
        let mut production = fast_gap_runtime(
            QueuedSourceFactory::default(),
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
            ),
        );
        for _ in 0..1_000 {
            poll_runtime_with_storage(&mut production, &fixture.storage).unwrap();
            if inventory_count(&fixture) >= round {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let completed = drive_gap_to_synchronized(&mut production, &fixture);
        assert_eq!(
            completed.roots[0].last_issue_code.as_deref(),
            Some(crate::domain::persistent_journal::METADATA_FALLBACK_CODE)
        );
        assert_eq!(
            completed.roots[0].continuity,
            PersistentJournalContinuityState::LiveOnly
        );
        production.stop().unwrap();
        assert_eq!(inventory_count(&fixture), round);
    }
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let location = catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "closed.png")
        .unwrap()
        .unwrap();
    assert_eq!((location.width, location.height), (9, 10));
    fixture.assert_no_automatic_full_scan();
}
