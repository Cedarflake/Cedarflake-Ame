use super::*;

#[cfg(windows)]
fn ordinary_user_test_directory() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::current_dir().expect("current directory"))
        .expect("ordinary-user disposable test directory")
}

#[cfg(windows)]
fn assert_production_p0_catalog_commit_guard(
    scope: crate::domain::LibraryChangeScope,
    relative_path: &str,
    guard_ancestor: bool,
    with_running_scan: bool,
) {
    let directory = ordinary_user_test_directory();
    let namespace = directory.path().join("namespace");
    let source = namespace.join("source");
    std::fs::create_dir_all(source.join("album")).expect("source namespace");
    let changed_relative_path = if scope == crate::domain::LibraryChangeScope::Root {
        "created.png"
    } else {
        "album/created.png"
    };
    image::RgbImage::from_pixel(2, 2, image::Rgb([40, 80, 160]))
        .save_with_format(source.join(changed_relative_path), image::ImageFormat::Png)
        .expect("authoritative source fixture");
    let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
        .expect("authoritative root discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical authoritative root")
        .to_string_lossy()
        .into_owned();
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("authoritative root identity query")
        .expect("authoritative stable root identity");
    drop(discovery);
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let generation = LibraryRootGeneration::initial();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan_with_publication_namespace(
            &ScanRequest {
                scan_id: "p0-commit-window-scan".to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            },
            &root_id,
            &root_path,
            &publication_identity,
        )
        .expect("begin proven initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("p0-commit-window-scan")
        .expect("prove P0 commit-window fixture first-import handoff");
    catalog
        .publish_scan("p0-commit-window-scan", &root_id, 0, 0)
        .expect("publish initial scan");
    let initial_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load initial root")
        .expect("published initial root");
    if with_running_scan {
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: "p0-concurrent-full-scan".to_owned(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                &root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin concurrent full scan");
    }
    catalog
        .enqueue_library_change_intents(
            &[crate::domain::LibraryChangeIntent {
                root_id: root_id.clone(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                scope,
                relative_path: relative_path.to_owned(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: base_unix_ms,
                most_recent_observed_unix_ms: base_unix_ms,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            base_unix_ms,
            policy,
        )
        .expect("enqueue P0 authoritative work");
    drop(catalog);

    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
    let guarded_path = if guard_ancestor {
        namespace.clone()
    } else {
        source.clone()
    };
    let moved_path = guarded_path.with_file_name(if guard_ancestor {
        "held-p0-ancestor"
    } else {
        "held-p0-root"
    });
    let rename_error = Arc::new(std::sync::atomic::AtomicI32::new(i32::MIN));
    let enumerated_entries = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let hook_rename_error = Arc::clone(&rename_error);
    let hook_enumerated_entries = Arc::clone(&enumerated_entries);
    let hook_root_path = root_path.clone();
    let hook_guarded_path = guarded_path.clone();
    let hook_moved_path = moved_path.clone();
    let _commit_hook = crate::adapters::set_before_catalog_delta_commit_hook(&root_id, move || {
        hook_enumerated_entries.store(
            crate::adapters::source_entry_read_count(&hook_root_path),
            Ordering::Release,
        );
        let error = std::fs::rename(&hook_guarded_path, &hook_moved_path)
            .expect_err("the production publication guard must reject the rename");
        hook_rename_error.store(error.raw_os_error().unwrap_or(-1), Ordering::Release);
        Err(ScanError::new(
            "root_publication_namespace_guard_unsupported",
            "The commit-window guard capability became unavailable",
        ))
    });

    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.runtime.queue_policy = policy;
    poll_runtime_with_storage(&mut production, &storage)
        .expect("schedule production P0 authoritative work");
    assert!(
        production.live.is_some(),
        "the production P0 worker must start"
    );

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        std::thread::sleep(Duration::from_millis(5));
        poll_runtime_with_storage(&mut production, &storage)
            .expect("poll production P0 authoritative work");
        let evidence: (String, Option<String>, Option<String>, Option<i64>) =
            rusqlite::Connection::open(&storage.catalog_path)
                .expect("open P0 commit-window evidence")
                .query_row(
                    "SELECT status, last_failure_code, last_failure_message,
                            catalog_revision_at_success
                     FROM library_change_queue
                     WHERE root_id = ?1 AND root_generation = ?2
                       AND origin = 'live_notification'",
                    rusqlite::params![
                        root_id,
                        i64::try_from(generation.value()).expect("root generation")
                    ],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("load P0 commit-window evidence");
        if evidence.0 == "retry_wait" {
            assert_eq!(
                evidence.1.as_deref(),
                Some("root_publication_namespace_guard_unsupported")
            );
            assert_eq!(evidence.3, None);
            assert_ne!(
                rename_error.load(Ordering::Acquire),
                i32::MIN,
                "commit hook did not fire; failure was {:?}",
                evidence.2
            );
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the P0 commit-window failure did not become durably retryable"
        );
    }
    let rename_error = rename_error.load(Ordering::Acquire);
    assert_eq!(rename_error, 32, "the held guard must deny delete sharing");
    if scope == crate::domain::LibraryChangeScope::Path {
        assert_eq!(
            enumerated_entries.load(Ordering::Acquire),
            0,
            "an exact P0 path must not enumerate its containing directory"
        );
    } else {
        assert!(enumerated_entries.load(Ordering::Acquire) > 0);
        assert!(crate::adapters::source_spool_open_count(&root_path) > 0);
    }
    let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("evidence catalog");
    let current_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("reload guarded root")
        .expect("guarded root remains published");
    assert_eq!(current_root.catalog_revision, initial_root.catalog_revision);
    assert_eq!(current_root.active_scan_id, initial_root.active_scan_id);
    assert_eq!(current_root.has_running_scan, with_running_scan);
    assert_eq!(
        current_root.last_consistency_audit_unix_ms,
        initial_root.last_consistency_audit_unix_ms
    );
    assert_eq!(
        current_root.publication_root_identity.as_ref(),
        Some(&publication_identity)
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root_id, changed_relative_path)
            .expect("load rolled-back location")
            .is_none()
    );
    drop(catalog);
    production.stop().expect("stop production synchronization");
    std::fs::rename(&guarded_path, &moved_path)
        .expect("guarded namespace rename after production worker exit");
    std::fs::rename(&moved_path, &guarded_path).expect("restore guarded namespace");
}

#[cfg(windows)]
#[test]
fn production_p0_root_holds_guard_through_catalog_rollback_on_commit_window_replacement() {
    assert_production_p0_catalog_commit_guard(
        crate::domain::LibraryChangeScope::Root,
        "",
        false,
        false,
    );
}

#[cfg(windows)]
#[test]
fn production_p0_subtree_holds_ancestor_guard_through_catalog_rollback() {
    assert_production_p0_catalog_commit_guard(
        crate::domain::LibraryChangeScope::Subtree,
        "album",
        true,
        false,
    );
}

#[cfg(windows)]
#[test]
fn production_p0_live_worker_crosses_running_full_scan_gate() {
    assert_production_p0_catalog_commit_guard(
        crate::domain::LibraryChangeScope::Path,
        "album/created.png",
        false,
        true,
    );
}

#[cfg(windows)]
#[test]
fn production_bounded_p2_holds_guard_through_catalog_rollback_without_retiring_authority() {
    let directory = ordinary_user_test_directory();
    let namespace = directory.path().join("namespace");
    let source = namespace.join("source");
    std::fs::create_dir_all(&source).expect("source namespace");
    image::RgbImage::from_pixel(2, 2, image::Rgb([80, 40, 160]))
        .save_with_format(source.join("created.png"), image::ImageFormat::Png)
        .expect("bounded P2 source fixture");
    let replacement = namespace.join("replacement");
    std::fs::create_dir(&replacement).expect("replacement root fixture");
    let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
        .expect("bounded P2 root discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical bounded P2 root")
        .to_string_lossy()
        .into_owned();
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("bounded P2 root identity query")
        .expect("bounded P2 stable root identity");
    drop(discovery);
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let generation = LibraryRootGeneration::initial();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan_with_publication_namespace(
            &ScanRequest {
                scan_id: "bounded-p2-commit-window-scan".to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            },
            &root_id,
            &root_path,
            &publication_identity,
        )
        .expect("begin proven initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("bounded-p2-commit-window-scan")
        .expect("prove bounded P2 fixture first-import handoff");
    catalog
        .publish_scan("bounded-p2-commit-window-scan", &root_id, 0, 0)
        .expect("publish initial scan");
    let initial_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load bounded P2 root")
        .expect("published bounded P2 root");
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root_id.clone(),
            root_generation: generation,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: base_unix_ms,
        })
        .expect("seed bounded P2 baseline capability");
    let baseline = catalog
        .begin_persistent_journal_baseline(
            &PersistentJournalBaselineStartRequest {
                run_id: "bounded-p2-commit-window".to_owned(),
                root_id: root_id.clone(),
                root_generation: generation,
                authority_reason: LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: "bounded-p2-volume".to_owned(),
                    volume_serial: 7,
                },
                root_file_reference: JournalFileReference::V2([7; 8]),
                journal_id: JournalIdentifier::new(44).expect("fixture journal ID"),
                opening_next_usn: JournalUsn::new(20).expect("fixture opening USN"),
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                authorized_unix_ms: base_unix_ms,
            },
            policy,
        )
        .expect("begin bounded P2 control window");
    catalog
        .enqueue_library_change_intents(
            &[crate::domain::LibraryChangeIntent {
                root_id: root_id.clone(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                scope: crate::domain::LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::UserRefresh,
                first_observed_unix_ms: base_unix_ms + 1,
                most_recent_observed_unix_ms: base_unix_ms + 1,
                first_sequence: 2,
                most_recent_sequence: 2,
                coalesced_observation_count: 1,
            }],
            base_unix_ms + 1,
            policy,
        )
        .expect("merge explicit refresh into bounded P2 control");
    drop(catalog);
    let fixture_connection = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open bounded P2 fixture evidence");
    let fixture_control: (String, String, Option<i64>) = fixture_connection
        .query_row(
            "SELECT queue.origin, queue.status, authority.retired_unix_ms
             FROM library_change_queue AS queue
             JOIN library_recovery_authorities AS authority ON authority.change_id = queue.id
             WHERE queue.id = ?1",
            [i64::try_from(baseline.change_id.value()).expect("baseline change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load bounded P2 control");
    assert_eq!(
        fixture_control,
        ("user_refresh".to_owned(), "pending".to_owned(), None)
    );
    drop(fixture_connection);

    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
    let displaced = namespace.join("displaced-source");
    let rename_error = Arc::new(std::sync::atomic::AtomicI32::new(i32::MIN));
    let enumerated_entries = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let hook_rename_error = Arc::clone(&rename_error);
    let hook_enumerated_entries = Arc::clone(&enumerated_entries);
    let hook_root_path = root_path.clone();
    let hook_source = source.clone();
    let hook_displaced = displaced.clone();
    let _commit_hook = crate::adapters::set_before_catalog_delta_commit_hook(&root_id, move || {
        hook_enumerated_entries.store(
            crate::adapters::source_entry_read_count(&hook_root_path),
            Ordering::Release,
        );
        let error = std::fs::rename(&hook_source, &hook_displaced)
            .expect_err("the production P2 guard must prevent root displacement");
        hook_rename_error.store(error.raw_os_error().unwrap_or(-1), Ordering::Release);
        Err(ScanError::new(
            "root_publication_namespace_guard_unsupported",
            "The P2 commit-window guard capability became unavailable",
        ))
    });

    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.runtime.queue_policy = policy;
    poll_runtime_with_storage(&mut production, &storage)
        .expect("schedule production bounded P2 work");
    assert!(matches!(
        production.recovery.as_ref().map(|task| task.kind()),
        Some(RecoveryTaskKind::BoundedAuthoritative { .. })
    ));

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        std::thread::sleep(Duration::from_millis(5));
        poll_runtime_with_storage(&mut production, &storage)
            .expect("poll production bounded P2 work");
        let evidence: (String, Option<String>, Option<i64>, String, Option<i64>) =
            rusqlite::Connection::open(&storage.catalog_path)
                .expect("open bounded P2 commit-window evidence")
                .query_row(
                    "SELECT queue.status, queue.last_failure_code,
                            queue.catalog_revision_at_success, baseline.phase,
                            authority.retired_unix_ms
                     FROM library_change_queue AS queue
                     JOIN library_persistent_journal_baselines AS baseline
                       ON baseline.change_id = queue.id
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = queue.id
                     WHERE queue.id = ?1",
                    [i64::try_from(baseline.change_id.value()).expect("baseline change ID")],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .expect("load bounded P2 commit-window evidence");
        if evidence.0 == "retry_wait" {
            assert_eq!(
                evidence,
                (
                    "retry_wait".to_owned(),
                    Some("root_publication_namespace_guard_unsupported".to_owned()),
                    None,
                    "inventory".to_owned(),
                    None,
                )
            );
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the bounded P2 commit-window failure did not become durably retryable"
        );
    }
    let rename_error = rename_error.load(Ordering::Acquire);
    assert_eq!(
        rename_error, 32,
        "the held P2 guard must deny delete sharing"
    );
    assert!(enumerated_entries.load(Ordering::Acquire) > 0);
    assert!(crate::adapters::source_spool_open_count(&root_path) > 0);
    let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("evidence catalog");
    let current_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("reload bounded P2 root")
        .expect("bounded P2 root remains published");
    assert_eq!(current_root.catalog_revision, initial_root.catalog_revision);
    assert_eq!(current_root.active_scan_id, initial_root.active_scan_id);
    assert_eq!(
        current_root.publication_root_identity.as_ref(),
        Some(&publication_identity)
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path(&root_id, "created.png")
            .expect("load rolled-back bounded P2 location")
            .is_none()
    );
    drop(catalog);
    production.stop().expect("stop production synchronization");
    std::fs::rename(&source, &displaced).expect("root displacement after P2 worker exit");
    std::fs::rename(&replacement, &source).expect("install replacement after guard release");
    std::fs::rename(&source, &replacement).expect("restore replacement fixture");
    std::fs::rename(&displaced, &source).expect("restore source root");
}
