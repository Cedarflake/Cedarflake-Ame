use super::*;

#[cfg(windows)]
#[test]
fn stale_live_only_first_import_capability_is_refreshed_once() {
    let directory = tempfile::tempdir().expect("test directory");
    let source_root = directory.path().join("source");
    std::fs::create_dir_all(&source_root).expect("source root");
    let root_path = source_root.to_string_lossy().into_owned();
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let request = ScanRequest {
        scan_id: "stale-live-only-first-import".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    let generation = LibraryRootGeneration::initial();
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan(&request, &root_id, &root_path)
        .expect("begin first import");
    let (_, started_unix_ms) = catalog
        .pristine_first_import_scan(&root_id, generation)
        .expect("load pristine first import")
        .expect("pristine first import");
    let stale_unix_ms = started_unix_ms
        .checked_sub(1)
        .expect("fixture scan starts after the Unix epoch");
    catalog
        .save_persistent_journal_capability(
            &crate::application::library_synchronization::journal_baseline::live_only_capability(
                &root_id,
                generation,
                stale_unix_ms,
            ),
        )
        .expect("seed stale LiveOnly capability");
    // Capability writes keep time monotonic; restore a genuinely older retained observation.
    let retained = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open retained capability fixture");
    assert_eq!(
        retained
            .execute(
                "UPDATE library_persistent_journal_root_state SET updated_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = 1
               AND capability_state = 'live_only' AND continuity_state = 'live_only'",
                rusqlite::params![stale_unix_ms, root_id],
            )
            .expect("restore retained capability time"),
        1,
    );
    drop(retained);
    assert!(
        !catalog
            .first_import_change_capture_is_ready(&request.scan_id, &root_id, generation)
            .expect("inspect stale first-import handoff")
    );
    drop(catalog);
    crate::application::catalog_session::open_catalog(
        &storage.catalog_path,
        LibraryChangeLane::Recovery,
    )
    .expect("prime protected catalog session");
    let _capture = crate::application::scan_library::hold_first_import_capture(
        &request.scan_id,
        &storage.catalog_path,
        &root_id,
        generation,
    )
    .expect("executing first import");

    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    let snapshot = poll_runtime_with_storage(&mut production, &storage)
        .expect("refresh stale first-import handoff");
    assert_eq!(
        snapshot.roots[0].source_health,
        crate::domain::LibraryChangeSourceHealth::Healthy
    );

    let catalog =
        SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect refreshed catalog");
    assert!(
        catalog
            .first_import_change_capture_is_ready(&request.scan_id, &root_id, generation)
            .expect("inspect refreshed first-import handoff")
    );
    let refreshed_unix_ms = catalog
        .load_persistent_journal_capabilities()
        .expect("load refreshed capability")
        .into_iter()
        .find(|capability| {
            capability.root_id == root_id && capability.root_generation == generation
        })
        .expect("refreshed LiveOnly capability")
        .updated_unix_ms;
    assert!(refreshed_unix_ms >= started_unix_ms);
    drop(catalog);

    poll_runtime_with_storage(&mut production, &storage)
        .expect("poll after first-import handoff became current");
    let catalog =
        SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect stable catalog");
    let stable_unix_ms = catalog
        .load_persistent_journal_capabilities()
        .expect("load stable capability")
        .into_iter()
        .find(|capability| {
            capability.root_id == root_id && capability.root_generation == generation
        })
        .expect("stable LiveOnly capability")
        .updated_unix_ms;
    assert_eq!(stable_unix_ms, refreshed_unix_ms);
    production
        .stop()
        .expect("stop stale LiveOnly production runtime");
}

#[cfg(windows)]
#[test]
fn failed_first_import_retries_with_fresh_generation_and_capture_owner() {
    let directory = tempfile::tempdir().expect("test directory");
    let source_root = directory.path().join("source");
    std::fs::create_dir_all(&source_root).expect("source root");
    let root_path = source_root.to_string_lossy().into_owned();
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let generation = LibraryRootGeneration::initial();
    let failed_request = ScanRequest {
        scan_id: "failed-first-import".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    let retry_request = ScanRequest {
        scan_id: "retried-first-import".to_owned(),
        ..failed_request.clone()
    };
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan(&failed_request, &root_id, &root_path)
        .expect("begin failed first import");
    let (_, failed_started_unix_ms) = catalog
        .pristine_first_import_scan(&root_id, generation)
        .expect("load failed first import")
        .expect("failed first import is pristine");
    let stale_unix_ms = failed_started_unix_ms
        .checked_sub(1)
        .expect("fixture scan starts after the Unix epoch");
    catalog
        .save_persistent_journal_capability(
            &crate::application::library_synchronization::journal_baseline::live_only_capability(
                &root_id,
                generation,
                stale_unix_ms,
            ),
        )
        .expect("seed failed import capability");
    catalog
        .abandon_scan(&failed_request.scan_id, "failed", 0)
        .expect("abandon first import");
    catalog
        .begin_scan(&retry_request, &root_id, &root_path)
        .expect("begin explicit retry in a fresh generation");
    let root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load retry root")
        .expect("retry root");
    let generation = generation.next().expect("fresh generation");
    assert_eq!(root.root_generation, generation);
    assert!(
        !catalog
            .first_import_change_capture_is_ready(&retry_request.scan_id, &root_id, generation,)
            .expect("inspect stale retry handoff")
    );
    drop(catalog);
    crate::application::catalog_session::open_catalog(
        &storage.catalog_path,
        LibraryChangeLane::Recovery,
    )
    .expect("prime protected retry catalog session");
    let _capture = crate::application::scan_library::hold_first_import_capture(
        &retry_request.scan_id,
        &storage.catalog_path,
        &root_id,
        generation,
    )
    .expect("executing explicit retry");

    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    poll_runtime_with_storage(&mut production, &storage).expect("refresh explicit retry handoff");
    let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect retry catalog");
    assert!(
        catalog
            .first_import_change_capture_is_ready(&retry_request.scan_id, &root_id, generation,)
            .expect("inspect refreshed retry handoff")
    );
    production
        .stop()
        .expect("stop failed-import retry production runtime");
}

#[cfg(windows)]
#[test]
fn startup_retires_legacy_full_scan_into_bounded_inventory_without_new_scan() {
    let directory = tempfile::tempdir().expect("test directory");
    let source_root = directory.path().join("source");
    std::fs::create_dir_all(&source_root).expect("source root");
    let root_path = source_root.to_string_lossy().into_owned();
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let initial = ScanRequest {
        scan_id: "initial-scan".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    let legacy = ScanRequest {
        scan_id: "legacy-automatic-full-scan".to_owned(),
        ..initial.clone()
    };
    let generation = crate::domain::LibraryRootGeneration::initial();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan(&initial, &root_id, &root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove automatic-recovery fixture first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, &root_id, 0, 0)
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
                origin: crate::domain::LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 1_000,
                most_recent_observed_unix_ms: 1_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            1_000,
            policy,
        )
        .expect("enqueue legacy recovery evidence");
    catalog
        .begin_authoritative_scan(&legacy, &root_id, &root_path)
        .expect("begin legacy automatic full scan");
    drop(catalog);

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
    poll_runtime_with_storage(&mut production, &storage).expect("start production synchronization");
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(25));
        poll_runtime_with_storage(&mut production, &storage)
            .expect("advance production synchronization");
    }

    assert!(production.legacy_automatic_scans_retired);

    production.stop().expect("stop production synchronization");
    let catalog = SqliteCatalog::open(storage.catalog_path).expect("reopen catalog");
    let active_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load active root")
        .expect("active root");

    assert!(
        catalog
            .load_authoritative_recoverable_scan_after(None)
            .expect("legacy recoverable scan")
            .is_none()
    );
    assert_eq!(
        active_root.active_scan_id.as_deref(),
        Some(initial.scan_id.as_str())
    );
    let connection = rusqlite::Connection::open(catalog.catalog_path())
        .expect("open catalog for inventory assertion");
    let recovery_evidence: (i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_metadata_inventory_runs),
               (SELECT COUNT(*) FROM scan_runs),
               (SELECT COUNT(*) FROM library_change_queue_lanes
                WHERE lane = 'p2_recovery')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("count bounded recovery and full-scan rows");
    assert_eq!(recovery_evidence, (0, 2, 1));
}
