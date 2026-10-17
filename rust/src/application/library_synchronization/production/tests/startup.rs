use super::*;

#[cfg(windows)]
#[test]
fn unsigned_production_defers_the_journal_factory_until_after_watcher_start() {
    let connect_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let factory = Arc::new(CountingJournalFactory {
        connect_count: Arc::clone(&connect_count),
        close_count: Arc::clone(&close_count),
        close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    });
    let mut production = new_production_synchronization_with_factory(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        factory,
    );

    assert_eq!(connect_count.load(Ordering::Acquire), 0);
    production.open_persistent_change_journal_after_watcher();
    assert_eq!(connect_count.load(Ordering::Acquire), 1);
    production.stop().expect("stop unsigned runtime");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[cfg(windows)]
#[test]
fn production_poll_starts_the_watcher_before_opening_the_journal_boundary() {
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
        scan_id: "initial-ordering-scan".to_owned(),
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
        .expect("prove ordering fixture first-import handoff");
    catalog
        .publish_scan(&request.scan_id, &root_id, 0, 0)
        .expect("publish initial scan");
    drop(catalog);

    let events = Arc::new(Mutex::new(Vec::new()));
    let probe = OrderingProbeFactory {
        events: Arc::clone(&events),
    };
    let mut production = new_production_synchronization_with_factory(
        crate::ports::erase_library_change_source_factory(probe.clone()),
        Arc::new(probe),
    );

    let snapshot =
        poll_runtime_with_storage(&mut production, &storage).expect("poll synchronization");

    assert_eq!(
        *events.lock().expect("ordering evidence"),
        ["watcher", "journal"]
    );
    let status = snapshot.roots.first().expect("root synchronization status");
    assert_eq!(
        status.continuity,
        PersistentJournalContinuityState::LiveOnly
    );
    assert_eq!(
        status.source_health,
        crate::domain::LibraryChangeSourceHealth::Healthy
    );
    assert_eq!(
        status.freshness,
        crate::domain::CatalogFreshnessState::Synchronized
    );
    production.stop().expect("stop synchronization");
}

#[cfg(windows)]
#[test]
fn one_runtime_validates_the_catalog_once_across_one_hundred_polls() {
    let directory = tempfile::tempdir().expect("runtime catalog directory");
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    crate::adapters::reset_full_schema_validation_count(&storage.catalog_path);
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );

    for _ in 0..100 {
        poll_runtime_with_storage(&mut production, &storage).expect("production poll");
    }

    assert_eq!(
        crate::adapters::full_schema_validation_count(&storage.catalog_path),
        1,
        "the production epoch must run O(N) schema validation only once"
    );
    assert_eq!(production.poll_catalog.connection_counts(), (1, 0));
    production.stop().expect("stop production runtime");
    assert_eq!(production.poll_catalog.connection_counts(), (1, 1));
}

#[cfg(windows)]
#[test]
fn one_hundred_no_change_startups_complete_the_production_observer_path_without_scanning() {
    let directory = tempfile::tempdir().expect("test directory");
    let source_root = directory.path().join("source");
    std::fs::create_dir_all(&source_root).expect("source root");
    std::fs::write(source_root.join("sentinel.txt"), b"must not be enumerated")
        .expect("source-root enumeration sentinel");
    let root_path = source_root.to_string_lossy().into_owned();
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let request = ScanRequest {
        scan_id: "initial-p1-scan".to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 512,
    };
    let registration = describe_production_persistent_journal_root(
        &root_id,
        LibraryRootGeneration::initial().value(),
        &source_root,
    )
    .expect("describe disposable P1 root");
    let root_reference =
        crate::domain::JournalFileReference::from_bytes(&registration.authorization.root_identity)
            .expect("root file reference");
    let volume = crate::domain::PersistentJournalVolumeIdentity {
        volume_guid: registration.authorization.volume_id.clone(),
        volume_serial: registration.volume_serial,
    };
    let journal_id = crate::domain::JournalIdentifier::new(44).expect("journal ID");
    let usn = crate::domain::JournalUsn::new(20).expect("USN");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan(&request, &root_id, &root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove journal fixture first-import handoff");
    catalog
        .publish_scan(&request.scan_id, &root_id, 0, 0)
        .expect("publish initial scan");
    let root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load root")
        .expect("published root");
    catalog
        .save_persistent_journal_capability(&crate::domain::PersistentJournalCapability {
            root_id: root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: crate::domain::PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::Supported,
            continuity: crate::domain::PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 1_000,
        })
        .expect("save supported capability");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
            root_id: root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            volume,
            root_file_reference: root_reference,
            journal_id,
            next_unread_usn: usn,
            captured_exclusive_end: usn,
            covered_catalog_revision: root.catalog_revision,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: crate::domain::PERSISTENT_JOURNAL_CONTRACT_VERSION,
            continuity: crate::domain::PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 1_000,
        })
        .expect("seed established checkpoint");
    drop(catalog);

    let scan_rows_before = {
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open scan-row assertion catalog");
        connection
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count initial scan rows")
    };
    let register_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let query_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let shared_read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let client_instances = Arc::new(Mutex::new(Vec::new()));
    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
    crate::adapters::reset_source_root_entry_enumeration_count(&root_path);
    crate::adapters::reset_configured_root_open_instrumentation(&root_path);
    crate::adapters::reset_source_content_open_instrumentation(&root_path);
    crate::adapters::reset_observer_root_handle_open_count(&root_path);
    crate::adapters::reset_root_availability_metadata_probe_instrumentation(&root_path);
    let mut availability_poll_count = 0_u64;

    for startup_index in 0..100_usize {
        let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(NoChangeJournalSession {
            register_count: Arc::clone(&register_count),
            query_count: Arc::clone(&query_count),
            shared_read_count: Arc::clone(&shared_read_count),
            close_count: Arc::clone(&close_count),
            client_instances: Arc::clone(&client_instances),
        });
        let mut production = new_production_synchronization_with_connection(
            crate::adapters::production_library_change_source_factory(),
            PersistentChangeJournalConnection::Connected(session),
        );
        let client_seed = u8::try_from(startup_index + 1).expect("startup client instance");
        production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
            process_id: std::process::id(),
            session_id: 1,
            client_instance: [client_seed; 16],
        });

        let expected_queries = startup_index + 1;
        availability_poll_count = availability_poll_count.saturating_add(1);
        let first_snapshot = poll_runtime_with_storage(&mut production, &storage)
            .expect("start production observer and no-change query");
        let first_status = first_snapshot
            .roots
            .first()
            .expect("root synchronization status");
        assert_eq!(
            first_status.continuity,
            PersistentJournalContinuityState::CatchingUp
        );
        assert_eq!(
            first_status.freshness,
            crate::domain::CatalogFreshnessState::Updating
        );
        assert_eq!(
            first_status.phase,
            LibrarySynchronizationPhase::Reconciliation
        );
        assert_eq!(
            first_status.source_health,
            crate::domain::LibraryChangeSourceHealth::Healthy
        );
        let query_deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < query_deadline {
            if query_count.load(Ordering::Acquire) == expected_queries {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(query_count.load(Ordering::Acquire), expected_queries);
        let harvest_deadline = std::time::Instant::now() + Duration::from_secs(5);
        let harvested = loop {
            availability_poll_count = availability_poll_count.saturating_add(1);
            let snapshot = poll_runtime_with_storage(&mut production, &storage)
                .expect("harvest completed no-change query");
            if production.journal.is_none() {
                break snapshot;
            }
            assert!(
                std::time::Instant::now() < harvest_deadline,
                "no-change query was not harvested by the production coordinator"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        let harvested_status = harvested
            .roots
            .first()
            .expect("harvested root synchronization status");
        assert_eq!(
            harvested_status.continuity,
            PersistentJournalContinuityState::Current
        );
        assert_eq!(
            harvested_status.freshness,
            crate::domain::CatalogFreshnessState::Synchronized
        );
        assert_eq!(register_count.load(Ordering::Acquire), expected_queries);
        assert_eq!(query_count.load(Ordering::Acquire), expected_queries);
        assert_eq!(shared_read_count.load(Ordering::Acquire), 0);
        assert_eq!(
            crate::adapters::observer_root_handle_open_count(&root_path),
            u64::try_from(expected_queries).expect("watcher root handles")
        );
        assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
        assert_eq!(crate::adapters::source_spool_open_count(&root_path), 0);
        assert_eq!(
            crate::adapters::source_root_entry_enumeration_count(&root_path),
            0
        );
        assert_eq!(crate::adapters::source_content_open_count(&root_path), 0);
        assert_eq!(
            crate::adapters::root_availability_metadata_probe_count(&root_path),
            availability_poll_count,
            "every production poll must perform exactly one metadata-only root availability probe"
        );
        assert_eq!(
            crate::adapters::configured_root_open_count(&root_path, true),
            0
        );
        assert_eq!(
            crate::adapters::configured_root_open_count(&root_path, false),
            0
        );
        production.stop().expect("stop no-change P1 scheduler");
        assert_eq!(close_count.load(Ordering::Acquire), expected_queries);
    }

    assert_eq!(register_count.load(Ordering::Acquire), 100);
    assert_eq!(query_count.load(Ordering::Acquire), 100);
    assert_eq!(shared_read_count.load(Ordering::Acquire), 0);
    assert_eq!(close_count.load(Ordering::Acquire), 100);
    let client_instances = client_instances.lock().expect("no-change client instances");
    assert_eq!(client_instances.len(), 100);
    assert!(
        client_instances.iter().all(|instance| *instance != [0; 16]),
        "every production startup must use a nonzero instance identity"
    );
    let unique_instances = client_instances
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique_instances.len(), 100);
    let connection = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open inventory assertion catalog");
    let inventory_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_runs",
            [],
            |row| row.get(0),
        )
        .expect("inventory count");
    assert_eq!(inventory_count, 0);
    let scan_rows_after = connection
        .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("count final scan rows");
    assert_eq!(scan_rows_after, scan_rows_before);
    assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
    assert_eq!(crate::adapters::source_spool_open_count(&root_path), 0);
    assert_eq!(
        crate::adapters::source_root_entry_enumeration_count(&root_path),
        0,
        "a continuous no-change startup must enumerate zero source-root entries"
    );
    assert_eq!(
        crate::adapters::configured_root_open_count(&root_path, true),
        0,
        "a continuous no-change startup must not construct a discovery root handle"
    );
    assert_eq!(
        crate::adapters::configured_root_open_count(&root_path, false),
        0,
        "a continuous no-change startup must not construct a publication guard"
    );
    assert_eq!(
        crate::adapters::source_content_open_count(&root_path),
        0,
        "a continuous no-change startup must open zero media content"
    );
    let availability_metadata_probes =
        crate::adapters::root_availability_metadata_probe_count(&root_path);
    assert_eq!(availability_metadata_probes, availability_poll_count);
    assert!(
        (200..=300).contains(&availability_metadata_probes),
        "100 no-change startups must remain within a bounded metadata-only availability-probe envelope; observed {availability_metadata_probes}"
    );
    eprintln!(
        "R2c-R no-change startups=100 production_polls={} production_observer_starts=100 watcher_root_handles={} nonzero_unique_client_instances={} journal_registers={} journal_queries={} journal_reads={} journal_closes={} source_enumeration_spool_opens=0 source_entry_reads=0 source_root_entries=0 inventory_runs=0 full_scan_rows_added=0 media_content_opens=0 availability_metadata_probes={} discovery_root_handles=0 publication_guard_root_handles=0",
        availability_poll_count,
        crate::adapters::observer_root_handle_open_count(&root_path),
        unique_instances.len(),
        register_count.load(Ordering::Acquire),
        query_count.load(Ordering::Acquire),
        shared_read_count.load(Ordering::Acquire),
        close_count.load(Ordering::Acquire),
        availability_metadata_probes,
    );
    drop(connection);
}

#[cfg(windows)]
#[test]
fn broker_absence_and_mismatch_remain_live_only_without_inventory() {
    for reason in [
        PersistentChangeJournalLiveOnlyReason::BrokerAbsent,
        PersistentChangeJournalLiveOnlyReason::ProtocolMismatch,
    ] {
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
            scan_id: format!("initial-{reason:?}"),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin initial scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove LiveOnly fixture first-import handoff");
        catalog
            .publish_scan(&request.scan_id, &root_id, 0, 0)
            .expect("publish initial scan");
        drop(catalog);

        let mut production = new_production_synchronization_with_connection(
            crate::ports::erase_library_change_source_factory(HealthyFactory),
            PersistentChangeJournalConnection::LiveOnly(reason),
        );
        let snapshot = poll_runtime_with_storage(&mut production, &storage)
            .expect("poll LiveOnly production synchronization");

        assert!(matches!(
            &production._persistent_change_journal,
            PersistentChangeJournalConnection::LiveOnly(actual) if *actual == reason
        ));
        assert!(production.recovery.is_none());
        assert_eq!(snapshot.roots.len(), 1);
        assert_eq!(
            snapshot.roots[0].source_health,
            crate::domain::LibraryChangeSourceHealth::Healthy
        );
        assert_eq!(
            snapshot.roots[0].freshness,
            crate::domain::CatalogFreshnessState::Synchronized
        );
        assert_eq!(snapshot.roots[0].pending_change_count, 0);
        assert_eq!(snapshot.roots[0].freshness_unknown_count, 0);

        let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("reopen catalog");
        let metrics = catalog
            .load_library_change_root_queue_metrics(
                &root_id,
                crate::domain::LibraryRootGeneration::initial(),
                now_unix_ms().expect("metrics time"),
                crate::domain::LibraryChangeQueuePolicy::default(),
            )
            .expect("load queue metrics");
        assert_eq!(metrics.pending_count, 0);
        assert_eq!(metrics.leased_count, 0);
        assert_eq!(metrics.retry_wait_count, 0);
        drop(catalog);
        let connection = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open catalog for inventory assertion");
        let inventory_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM library_metadata_inventory_runs",
                [],
                |row| row.get(0),
            )
            .expect("count metadata inventory runs");
        assert_eq!(inventory_count, 0);

        production.stop().expect("stop production synchronization");
    }
}
