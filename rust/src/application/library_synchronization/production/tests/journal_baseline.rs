use super::*;

#[cfg(windows)]
#[test]
fn production_baseline_brackets_inventory_and_publishes_current_atomically() {
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
        scan_id: "initial-production-baseline".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan(&request, &root_id, &root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove baseline fixture first-import handoff");
    catalog
        .publish_scan(&request.scan_id, &root_id, 0, 0)
        .expect("publish initial scan");
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: 1,
        })
        .expect("require an existing-root production baseline");
    drop(catalog);

    let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(NoChangeJournalSession {
        register_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        query_count: Arc::clone(&query_count),
        shared_read_count: Arc::clone(&shared_read_count),
        close_count: Arc::clone(&close_count),
        client_instances: Arc::new(Mutex::new(Vec::new())),
    });
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(session),
    );
    production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [8; 16],
    });

    let mut saw_active_baseline = false;
    let mut completed = false;
    for _ in 0..300 {
        poll_runtime_with_storage(&mut production, &storage).expect("poll baseline lifecycle");
        let catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect baseline catalog");
        saw_active_baseline |= !catalog
            .load_persistent_journal_baselines()
            .expect("load active baselines")
            .is_empty();
        let checkpoint = catalog
            .load_persistent_journal_checkpoint(&root_id, LibraryRootGeneration::initial())
            .expect("load baseline checkpoint");
        drop(catalog);
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open baseline assertion catalog");
        let completed_baselines: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM library_persistent_journal_baselines
                 WHERE root_id = ?1 AND phase = 'completed'",
                [&root_id],
                |row| row.get(0),
            )
            .expect("count completed baselines");
        let retired_authorities: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM library_persistent_journal_baselines AS baseline
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = baseline.change_id
                 WHERE baseline.root_id = ?1
                   AND authority.retired_unix_ms IS NOT NULL",
                [&root_id],
                |row| row.get(0),
            )
            .expect("count retired baseline authorities");
        completed = completed_baselines == 1
            && retired_authorities == 1
            && checkpoint.is_some_and(|checkpoint| {
                checkpoint.continuity == PersistentJournalContinuityState::Current
            });
        if completed {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(saw_active_baseline);
    assert!(completed);
    assert!(query_count.load(Ordering::Acquire) >= 2);
    assert_eq!(shared_read_count.load(Ordering::Acquire), 1);
    production.stop().expect("stop production baseline runtime");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[cfg(windows)]
#[test]
fn production_baseline_replays_changes_during_inventory_before_publishing_current() {
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
        scan_id: "initial-production-advancing-baseline".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan(&request, &root_id, &root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove advancing-baseline fixture first-import handoff");
    catalog
        .publish_scan(&request.scan_id, &root_id, 0, 0)
        .expect("publish initial scan");
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: 1,
        })
        .expect("require an advancing existing-root production baseline");
    drop(catalog);

    let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let read_release = Arc::new(AtomicBool::new(false));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(AdvancingJournalSession {
        boundaries: Mutex::new(std::collections::VecDeque::from([20, 21])),
        query_count: Arc::clone(&query_count),
        shared_read_count: Arc::clone(&shared_read_count),
        read_release: Arc::clone(&read_release),
        close_count: Arc::clone(&close_count),
    });
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(session),
    );
    production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [5; 16],
    });

    let mut saw_catching_up = false;
    let mut completed = false;
    for _ in 0..300 {
        poll_runtime_with_storage(&mut production, &storage)
            .expect("poll advancing baseline lifecycle");
        let catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("inspect baseline catalog");
        let checkpoint = catalog
            .load_persistent_journal_checkpoint(&root_id, LibraryRootGeneration::initial())
            .expect("load advancing checkpoint");
        saw_catching_up |= checkpoint.as_ref().is_some_and(|checkpoint| {
            checkpoint.next_unread_usn.value() >= 21
                && checkpoint.continuity == PersistentJournalContinuityState::CatchingUp
        });
        drop(catalog);
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open advancing assertion catalog");
        let evidence: (i64, i64, i64) = connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*)
                    FROM library_persistent_journal_baselines
                    WHERE root_id = ?1 AND phase = 'completed'),
                   (SELECT COUNT(*)
                    FROM library_persistent_journal_source_ranges AS ranges
                    JOIN library_persistent_journal_range_lifecycle AS lifecycle
                      ON lifecycle.source_range_id = ranges.id
                    WHERE ranges.root_id = ?1
                      AND ranges.requested_start_usn = '20'
                      AND ranges.covered_until_usn = '21'
                      AND lifecycle.lifecycle_state = 'completed'),
                   (SELECT COUNT(*)
                    FROM library_persistent_journal_baselines AS baseline
                    JOIN library_recovery_authorities AS authority
                      ON authority.change_id = baseline.change_id
                    WHERE baseline.root_id = ?1
                      AND authority.retired_unix_ms IS NOT NULL)",
                [&root_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load advancing baseline evidence");
        if shared_read_count.load(Ordering::Acquire) == 1 && !read_release.load(Ordering::Acquire) {
            saw_catching_up |= checkpoint.as_ref().is_some_and(|checkpoint| {
                checkpoint.next_unread_usn.value() == 20
                    && checkpoint.captured_exclusive_end.value() == 21
                    && checkpoint.continuity == PersistentJournalContinuityState::CatchingUp
            });
            assert_eq!(evidence.0, 0);
            read_release.store(true, Ordering::Release);
        }
        completed = evidence == (1, 1, 1)
            && checkpoint.is_some_and(|checkpoint| {
                checkpoint.continuity == PersistentJournalContinuityState::Current
                    && checkpoint.next_unread_usn.value() == 21
            });
        if completed {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    read_release.store(true, Ordering::Release);
    assert!(saw_catching_up);
    assert!(completed);
    assert!(query_count.load(Ordering::Acquire) >= 3);
    assert_eq!(shared_read_count.load(Ordering::Acquire), 1);
    production
        .stop()
        .expect("stop advancing production baseline runtime");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}
