use super::*;

#[cfg(windows)]
#[test]
fn newer_continuity_revision_cancels_an_older_automatic_worker() {
    let directory = tempfile::tempdir().expect("catalog directory");
    let source = tempfile::tempdir().expect("source directory");
    let root_path = source.path().to_string_lossy().into_owned();
    let root_id = "root-a";
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: "scan-a".to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            },
            root_id,
            &root_path,
        )
        .expect("begin root scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("scan-a")
        .expect("prove continuity revision fixture first-import handoff");
    catalog
        .publish_scan("scan-a", root_id, 0, 0)
        .expect("publish root scan");
    let mut production = ProductionSynchronization {
        runtime: LibrarySynchronizationRuntime::new_erased(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
        ),
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
        recovery: None,
        recovery_sources: RecoverySources::default(),
        indexed_startup: IndexedStartupOwner::default(),
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
    production
        .runtime
        .poll_without_authoritative_recovery(&mut catalog, 1_000, |_| {
            crate::domain::LibraryRootAvailability::Available
        })
        .expect("establish first continuity epoch");
    let first_revision = production
        .runtime
        .root_continuity_revision(root_id)
        .expect("first continuity revision");
    let cancelled = Arc::new(AtomicBool::new(false));
    let (_sender, receiver) = mpsc::sync_channel(1);
    production.recovery = Some(RecoveryTask::from_test_parts(
        root_id.to_owned(),
        RecoveryTaskKind::MetadataInventory {
            continuity_revision: first_revision,
            change_id: LibraryChangeId::new(1).expect("test recovery change ID"),
        },
        Arc::new(Mutex::new(
            LibrarySynchronizationPhase::InventoryEnumeration,
        )),
        Arc::clone(&cancelled),
        receiver,
        None,
    ));
    production
        .runtime
        .roots
        .get_mut(root_id)
        .expect("runtime root")
        .needs_continuity_gap = true;
    production
        .runtime
        .poll_without_authoritative_recovery(&mut catalog, 2_000, |_| {
            crate::domain::LibraryRootAvailability::Available
        })
        .expect("establish newer continuity epoch");

    production.cancel_stale_automatic_recovery();

    assert!(cancelled.load(Ordering::Acquire));
    assert!(
        production
            .runtime
            .root_continuity_revision(root_id)
            .expect("newer continuity revision")
            > first_revision
    );
}

#[cfg(windows)]
#[test]
fn bounded_authoritative_success_clears_only_its_root_retry_history() {
    let factory = crate::adapters::production_library_change_source_factory();
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
        recovery: None,
        recovery_sources: RecoverySources::default(),
        indexed_startup: IndexedStartupOwner::default(),
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
    production.recovery_lane.record_failure("root-a", 1_000);
    production.recovery_lane.record_failure("root-b", 5_000);
    let root_b_retry = production
        .recovery_lane
        .retry_for_test("root-b")
        .expect("root-b retry");

    let (sender, receiver) = mpsc::sync_channel(1);
    sender
        .send(Ok(RecoveryTaskOutcome::Authoritative(
            AuthoritativeLibraryChangeReport::default(),
        )))
        .expect("bounded recovery result");
    production.recovery = Some(RecoveryTask::from_test_parts(
        "root-a".to_owned(),
        RecoveryTaskKind::BoundedAuthoritative {
            continuity_revision: 0,
        },
        Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
        Arc::new(AtomicBool::new(false)),
        receiver,
        None,
    ));

    production
        .poll_recovery(2_000)
        .expect("bounded recovery succeeds");
    assert!(production.recovery_lane.retry_for_test("root-a").is_none());
    assert_eq!(
        production
            .recovery_lane
            .retry_for_test("root-b")
            .expect("root-b retry"),
        root_b_retry
    );
}

#[cfg(windows)]
#[test]
fn production_does_not_mint_recovery_authority_for_an_ordinary_consistency_audit() {
    let directory = tempfile::tempdir().expect("test directory");
    let source = directory.path().join("source");
    std::fs::create_dir_all(&source).expect("source root");
    let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
        .expect("ordinary audit root discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical ordinary audit root")
        .to_string_lossy()
        .into_owned();
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("ordinary audit root identity query")
        .expect("ordinary audit root stable identity");
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let generation = crate::domain::LibraryRootGeneration::initial();
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan_with_publication_namespace(
            &ScanRequest {
                scan_id: "ordinary-audit-scan".to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            },
            &root_id,
            &root_path,
            &publication_identity,
        )
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("ordinary-audit-scan")
        .expect("prove audit fixture first-import handoff");
    catalog
        .publish_scan("ordinary-audit-scan", &root_id, 0, 0)
        .expect("publish initial scan");
    catalog
        .enqueue_library_change_intents(
            &[crate::domain::LibraryChangeIntent {
                root_id: root_id.clone(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                scope: crate::domain::LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 1,
                most_recent_observed_unix_ms: 1,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            1,
            policy,
        )
        .expect("enqueue ordinary audit");
    drop(catalog);

    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.runtime.queue_policy = policy;
    poll_runtime_with_storage(&mut production, &storage).expect("poll production runtime");

    assert!(matches!(
        production.recovery.as_ref().map(|task| task.kind()),
        Some(RecoveryTaskKind::LegacyUnownedDrain { .. })
    ));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        std::thread::sleep(Duration::from_millis(5));
        poll_runtime_with_storage(&mut production, &storage)
            .expect("finish blocking the unowned ordinary audit");
        if production.recovery.is_none() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the unowned ordinary audit did not reach its durable blocked state"
        );
    }
    let connection =
        rusqlite::Connection::open(&storage.catalog_path).expect("open authority evidence catalog");
    let authority_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_recovery_authorities",
            [],
            |row| row.get(0),
        )
        .expect("count recovery authorities");
    assert_eq!(authority_count, 0, "worker must never mint authority");
    let blocked: (String, i64, Option<i64>, Option<String>) = connection
        .query_row(
            "SELECT status, attempt_count, next_retry_unix_ms, last_failure_code
             FROM library_change_queue WHERE root_id = ?1",
            [&root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("load blocked ordinary audit");
    assert_eq!(
        blocked,
        (
            "retry_wait".to_owned(),
            i64::from(policy.max_attempts),
            None,
            Some("legacy_recovery_authority_missing".to_owned())
        )
    );
    poll_runtime_with_storage(&mut production, &storage)
        .expect("blocked ordinary audit remains idempotent");
    assert!(
        production.recovery.is_none(),
        "a blocked unowned audit must not restart the worker"
    );
    production.stop().expect("stop production synchronization");
}
