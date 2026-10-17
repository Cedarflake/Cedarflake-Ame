use super::*;

#[cfg(windows)]
#[test]
fn production_coordinator_completes_4096_real_files_across_the_default_page() {
    const TOTAL_FILES: usize = METADATA_INVENTORY_WORK_PAGE_ENTRIES as usize + 1;
    const TEST_DEADLINE: Duration = Duration::from_secs(300);

    let started = std::time::Instant::now();
    let directory = tempfile::tempdir().expect("test directory");
    let source = directory.path().join("source");
    std::fs::create_dir_all(&source).expect("source root");
    let template = directory.path().join("template.png");
    image::RgbImage::from_pixel(1, 1, image::Rgb([40, 80, 160]))
        .save(&template)
        .expect("PNG template");
    let png = std::fs::read(&template).expect("read PNG template");
    for index in 0..TOTAL_FILES {
        std::fs::write(source.join(format!("image-{index:05}.png")), &png)
            .expect("write controlled PNG");
    }

    let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
        .expect("controlled root discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical controlled root")
        .to_string_lossy()
        .into_owned();
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("controlled root identity query")
        .expect("controlled stable root identity");
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        lease_duration_millis: crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let generation = LibraryRootGeneration::initial();
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan_with_publication_namespace(
            &ScanRequest {
                scan_id: "production-4096-initial".to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 64,
            },
            &root_id,
            &root_path,
            &publication_identity,
        )
        .expect("begin empty initial catalog");
    catalog
        .prove_live_only_first_import_handoff_for_test("production-4096-initial")
        .expect("prove 4096-file fixture first-import handoff");
    catalog
        .publish_scan("production-4096-initial", &root_id, 0, 0)
        .expect("publish empty initial catalog");

    let registration =
        describe_production_persistent_journal_root(&root_id, generation.value(), &source)
            .expect("describe controlled root");
    let root_reference =
        JournalFileReference::from_bytes(&registration.authorization.root_identity)
            .expect("controlled root reference");
    let volume = PersistentJournalVolumeIdentity {
        volume_guid: registration.authorization.volume_id.clone(),
        volume_serial: registration.volume_serial,
    };
    let journal_id = JournalIdentifier::new(44).expect("journal ID");
    let opening_next_usn = JournalUsn::new(20).expect("opening USN");
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root_id.clone(),
            root_generation: generation,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: base_unix_ms,
        })
        .expect("persist supported journal capability");
    let current_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load controlled root")
        .expect("published controlled root");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
            root_id: root_id.clone(),
            root_generation: generation,
            volume: volume.clone(),
            root_file_reference: root_reference.clone(),
            journal_id,
            next_unread_usn: opening_next_usn,
            captured_exclusive_end: opening_next_usn,
            covered_catalog_revision: current_root.catalog_revision,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: base_unix_ms,
        })
        .expect("persist opening checkpoint");
    let recovery_control = catalog
        .persist_persistent_journal_root_failure(
            &crate::domain::PersistentJournalRootFailure {
                root_id: root_id.clone(),
                root_generation: generation,
                kind: crate::domain::PersistentJournalRootFailureKind::ContainmentFailure,
                failure: PersistentJournalFailure {
                    code: "production-4096-containment".to_owned(),
                    message: "Controlled recovery fixture".to_owned(),
                },
                opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                    volume: volume.clone(),
                    root_file_reference: root_reference,
                    journal_id,
                    next_usn: opening_next_usn,
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                }),
            },
            base_unix_ms + 1,
            policy,
        )
        .expect("persist typed containment failure")
        .expect("containment failure admits recovery");
    drop(catalog);

    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
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
        client_instance: [6; 16],
    });
    production.runtime.queue_policy = policy;
    assert_eq!(
        production.metadata_inventory_page_entries,
        METADATA_INVENTORY_WORK_PAGE_ENTRIES
    );

    let evidence_connection = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open retained evidence connection");
    let mut saw_bounded_raw_capture = false;
    let mut saw_default_source_page = false;
    let mut saw_backpressured_drain = false;
    let mut saw_refill_beyond_first_admission = false;
    let mut completed = false;
    let mut last_evidence = None;
    while started.elapsed() < TEST_DEADLINE {
        poll_runtime_with_storage(&mut production, &storage)
            .expect("drive production recovery coordinator");
        let source_reads = crate::adapters::source_entry_read_count(&root_path);
        saw_bounded_raw_capture |= source_reads > 0 && source_reads < TOTAL_FILES as u64;

        let evidence: (
            i64,
            i64,
            String,
            String,
            i64,
            String,
            String,
            String,
            i64,
            i64,
        ) = evidence_connection
            .query_row(
                "SELECT
                       COALESCE(run.staged_entry_count, 0),
                       COALESCE(run.candidate_count, 0),
                       COALESCE(run.status, 'missing'),
                       control.status,
                       authority.retired_unix_ms IS NOT NULL,
                       baseline.phase,
                       COALESCE(checkpoint.continuity_state, 'missing'),
                       root_state.continuity_state,
                       (SELECT COUNT(*)
                        FROM library_metadata_inventory_candidate_owners AS owner
                        WHERE owner.run_id = authority.run_id),
                       (SELECT COUNT(*)
                        FROM library_metadata_inventory_candidate_owners AS owner
                        JOIN library_change_queue AS owned ON owned.id = owner.change_id
                        WHERE owner.run_id = authority.run_id
                          AND owned.status = 'completed')
                     FROM library_recovery_authorities AS authority
                     JOIN library_change_queue AS control ON control.id = authority.change_id
                     JOIN library_persistent_journal_baselines AS baseline
                       ON baseline.change_id = authority.change_id
                     JOIN library_persistent_journal_root_state AS root_state
                       ON root_state.root_id = authority.root_id
                      AND root_state.root_generation = authority.root_generation
                     LEFT JOIN library_metadata_inventory_runs AS run
                       ON run.id = authority.run_id
                     LEFT JOIN library_persistent_journal_checkpoints AS checkpoint
                       ON checkpoint.root_id = authority.root_id
                      AND checkpoint.root_generation = authority.root_generation
                     WHERE authority.change_id = ?1",
                [i64::try_from(recovery_control.value()).expect("control ID")],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                    ))
                },
            )
            .expect("load production-chain evidence");
        saw_default_source_page |= evidence.0 == i64::from(METADATA_INVENTORY_WORK_PAGE_ENTRIES);
        saw_backpressured_drain |= evidence.1
            == i64::from(policy.lane_capacity(LibraryChangeLane::Recovery))
            && evidence.9 > 0;
        saw_refill_beyond_first_admission |=
            evidence.1 > i64::from(policy.lane_capacity(LibraryChangeLane::Recovery));
        if evidence
            == (
                i64::try_from(TOTAL_FILES).expect("staged total"),
                i64::try_from(TOTAL_FILES).expect("candidate total"),
                "completed".to_owned(),
                "completed".to_owned(),
                1,
                "completed".to_owned(),
                "current".to_owned(),
                "current".to_owned(),
                i64::try_from(TOTAL_FILES).expect("owner total"),
                i64::try_from(TOTAL_FILES).expect("terminal total"),
            )
        {
            completed = true;
            break;
        }
        last_evidence = Some(evidence);
        std::thread::sleep(Duration::from_millis(2));
    }

    assert!(
        completed,
        "the production chain must finish every owner and retire authority; last={last_evidence:?}"
    );
    assert!(
        saw_bounded_raw_capture,
        "the real directory must yield before consuming all 4096 source entries"
    );
    assert!(
        saw_default_source_page,
        "the production source must durably publish its 4095-entry default page"
    );
    assert!(
        saw_backpressured_drain,
        "the first P2 admission must drain before candidate refill"
    );
    assert!(
        saw_refill_beyond_first_admission,
        "production refill must cross the first P2 admission window"
    );
    assert_eq!(
        crate::adapters::source_entry_read_count(&root_path),
        u64::try_from(TOTAL_FILES).expect("source reads")
    );
    assert_eq!(crate::adapters::source_spool_open_count(&root_path), 1);
    assert!(crate::adapters::source_peak_staged_window(&root_path) <= 128);
    let spool_state: (i64, i64) = evidence_connection
        .query_row(
            "SELECT COUNT(*), COUNT(*) FILTER (WHERE state = 'retired')
             FROM library_metadata_inventory_spools",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load atomic retirement and background cleanup evidence");
    assert!(
        matches!(spool_state, (0, 0) | (1, 1)),
        "completed recovery permits only an absent or retired spool, never executable storage: {spool_state:?}"
    );
    assert!(
        query_count.load(Ordering::Acquire) >= 1,
        "production must capture the closing journal boundary"
    );
    assert_eq!(
        shared_read_count.load(Ordering::Acquire),
        1,
        "closing must perform exactly one bounded replay read even when the window is empty"
    );
    production.stop().expect("stop production synchronization");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone())
        .unwrap_or_else(|error| {
            let evidence: String = evidence_connection.query_row(
                "SELECT json_object('baseline_completed', baseline.completed_unix_ms,
                    'closing', baseline.closing_next_usn, 'next', checkpoint.next_unread_usn,
                    'end', checkpoint.captured_exclusive_end,
                    'root_state', root.continuity_state, 'checkpoint_state', checkpoint.continuity_state,
                    'range_start', ranges.requested_start_usn, 'range_end', ranges.requested_end_usn,
                    'range_covered', ranges.covered_until_usn, 'range_state', ranges.status,
                    'range_enrolled', ranges.enrolled_unix_ms, 'range_lifecycle', lifecycle.lifecycle_state)
                 FROM library_persistent_journal_baselines AS baseline
                 JOIN library_persistent_journal_root_state AS root USING(root_id, root_generation)
                 JOIN library_persistent_journal_checkpoints AS checkpoint USING(root_id, root_generation)
                 LEFT JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.root_id = baseline.root_id AND ranges.covered_until_usn = checkpoint.next_unread_usn
                 LEFT JOIN library_persistent_journal_range_lifecycle AS lifecycle ON lifecycle.source_range_id = ranges.id
                 ORDER BY ranges.enrolled_unix_ms DESC LIMIT 1", [], |row| row.get(0),
            ).expect("bounded baseline failure evidence");
            panic!("FULL reopen after production retirement: {error:?}; {evidence}")
        });
    let checkpoint = catalog
        .load_persistent_journal_checkpoint(&root_id, generation)
        .expect("load completed recovery checkpoint")
        .expect("completed recovery retains its checkpoint");
    assert_eq!(checkpoint.next_unread_usn, opening_next_usn);
    assert_eq!(checkpoint.captured_exclusive_end, opening_next_usn);
    assert_eq!(
        checkpoint.continuity,
        PersistentJournalContinuityState::Current
    );
    let run_id: String = evidence_connection
        .query_row(
            "SELECT run_id FROM library_recovery_authorities WHERE change_id = ?1",
            [i64::try_from(recovery_control.value()).expect("control ID")],
            |row| row.get(0),
        )
        .expect("completed recovery run identity");
    let run = catalog
        .load_metadata_inventory_run(&run_id)
        .expect("load completed run")
        .expect("retention preserves the completed summary");
    assert_eq!(
        catalog
            .stage_metadata_inventory_page(
                &run_id,
                &crate::domain::MetadataInventoryPage {
                    page_index: run.next_page_index,
                    entries: Vec::new(),
                    cursor: None,
                    is_complete: true,
                    frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                        "",
                        Some(publication_identity),
                    )],
                },
                now_unix_ms().expect("terminal page attempt clock"),
            )
            .expect_err("completed recovery cannot accept another source page")
            .code,
        "metadata_inventory_run_not_running",
    );
    let remaining_entries: i64 = evidence_connection.query_row(
        "SELECT (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries WHERE run_id = ?1)
              + (SELECT COUNT(*) FROM library_metadata_inventory_entries WHERE run_id = ?1)",
        [&run_id], |row| row.get(0),
    ).expect("persisted raw and logical debt after all workers stop");
    let mut reclaimed_entries = 0_i64;
    let mut cleanup_complete = false;
    for _ in 0..128 {
        assert!(
            started.elapsed() < TEST_DEADLINE,
            "cleanup shares the original test deadline"
        );
        let report = catalog
            .cleanup_terminal_metadata_inventories(0, 128, 1, Default::default())
            .expect("one real bounded cleanup batch");
        assert!(report.removed_entry_count <= 128);
        assert_eq!(
            report.removed_run_count, 0,
            "existing terminal retention stays intact"
        );
        reclaimed_entries += i64::from(report.removed_entry_count);
        if !report.has_more {
            cleanup_complete = true;
            break;
        }
    }
    assert!(
        cleanup_complete,
        "the 4096-file debt must finish within 128 bounded batches"
    );
    assert_eq!(reclaimed_entries, remaining_entries);
    let retained: (i64, i64, i64, i64, i64) = evidence_connection.query_row(
        "SELECT (SELECT COUNT(*) FROM library_metadata_inventory_spools WHERE run_id = ?1),
                (SELECT COUNT(*) FROM library_metadata_inventory_spool_directories WHERE run_id = ?1),
                (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries WHERE run_id = ?1),
                (SELECT COUNT(*) FROM library_metadata_inventory_entries WHERE run_id = ?1),
                (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners WHERE run_id = ?1)",
        [&run_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).expect("physical retirement and retained candidate lineage");
    assert_eq!(
        retained,
        (0, 0, 0, 0, i64::try_from(TOTAL_FILES).expect("owner total"))
    );
    assert_eq!(
        crate::adapters::source_entry_read_count(&root_path),
        u64::try_from(TOTAL_FILES).expect("unchanged source reads"),
    );
    SqliteCatalog::open(storage.catalog_path.clone())
        .expect("FULL reopen after all bounded raw and logical cleanup batches");
}

#[cfg(windows)]
#[test]
fn p2_page_completion_rotates_to_another_root_and_retains_each_frontier() {
    let directory = tempfile::tempdir().expect("test directory");
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    let generation = crate::domain::LibraryRootGeneration::initial();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let mut root_paths = BTreeMap::new();
    for root_id in ["root-a", "root-b"] {
        let source = directory.path().join(root_id);
        std::fs::create_dir_all(&source).expect("source root");
        for index in 0..257 {
            std::fs::write(source.join(format!("image-{index}.jpg")), b"metadata")
                .expect("metadata fixture");
        }
        let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
            .expect("controlled root discovery");
        let root_path = discovery
            .canonical_root()
            .expect("canonical controlled root")
            .to_string_lossy()
            .into_owned();
        let publication_identity = discovery
            .metadata_inventory_root_identity()
            .expect("controlled root identity query")
            .expect("controlled stable root identity");
        root_paths.insert(root_id, root_path.clone());
        let scan_id = format!("scan-{root_id}");
        catalog
            .begin_scan_with_publication_namespace(
                &ScanRequest {
                    scan_id: scan_id.clone(),
                    root_path: root_path.clone(),
                    max_items: None,
                    max_entries: None,
                    preview_edge: 512,
                },
                root_id,
                &root_path,
                &publication_identity,
            )
            .expect("begin root scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&scan_id)
            .expect("prove P2 page fixture first-import handoff");
        catalog
            .publish_scan(&scan_id, root_id, 0, 0)
            .expect("publish root scan");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: generation,
                protocol_version: 1,
                contract_version: 1,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1,
            })
            .expect("seed current journal capability");
        let root = catalog
            .load_incremental_catalog_root(root_id)
            .expect("load current root")
            .expect("published root");
        let root_marker = if root_id == "root-a" { 1 } else { 2 };
        let checkpoint = PersistentJournalCheckpoint {
            root_id: root_id.to_owned(),
            root_generation: generation,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: format!("volume-{root_id}"),
                volume_serial: root_marker,
            },
            root_file_reference: JournalFileReference::V2([root_marker as u8; 8]),
            journal_id: JournalIdentifier::new(9 + root_marker).expect("journal ID"),
            next_unread_usn: JournalUsn::new(20).expect("opening USN"),
            captured_exclusive_end: JournalUsn::new(20).expect("opening end USN"),
            covered_catalog_revision: root.catalog_revision,
            protocol_version: 1,
            contract_version: 1,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 1,
        };
        catalog
            .seed_persistent_journal_checkpoint_for_test(&checkpoint)
            .expect("seed current journal checkpoint");
        catalog
            .persist_persistent_journal_root_failure(
                &crate::domain::PersistentJournalRootFailure {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    kind: crate::domain::PersistentJournalRootFailureKind::ContainmentFailure,
                    failure: PersistentJournalFailure {
                        code: format!("fixture-containment-{root_id}"),
                        message: format!("Fixture containment failure for {root_id}"),
                    },
                    opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                        volume: checkpoint.volume.clone(),
                        root_file_reference: checkpoint.root_file_reference.clone(),
                        journal_id: checkpoint.journal_id,
                        next_usn: checkpoint.next_unread_usn,
                        protocol_version: checkpoint.protocol_version,
                        contract_version: checkpoint.contract_version,
                    }),
                },
                2,
                policy,
            )
            .expect("admit typed containment recovery")
            .expect("allowlisted containment recovery");
    }
    drop(catalog);
    let root_a_path = root_paths.remove("root-a").expect("canonical root-a path");
    let root_b_path = root_paths.remove("root-b").expect("canonical root-b path");
    crate::adapters::reset_source_enumeration_instrumentation(&root_a_path);
    crate::adapters::reset_source_enumeration_instrumentation(&root_b_path);
    let root_a_gate = crate::adapters::gate_source_enumeration(&root_a_path);
    let root_b_gate = crate::adapters::gate_source_enumeration(&root_b_path);
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

    poll_runtime_with_storage(&mut production, &storage).expect("schedule first P2 page");
    let first_root = production
        .recovery
        .as_ref()
        .expect("first recovery task")
        .root_id()
        .to_owned();
    let first_active_change_id = match production
        .recovery
        .as_ref()
        .expect("first recovery task")
        .kind()
    {
        RecoveryTaskKind::MetadataInventory { change_id, .. } => *change_id,
        _ => panic!("first root must own metadata inventory recovery"),
    };
    let first_gate = if first_root == "root-a" {
        &root_a_gate
    } else {
        &root_b_gate
    };
    let first_path = if first_root == "root-a" {
        &root_a_path
    } else {
        &root_b_path
    };
    assert!(
        first_gate.wait_until_blocked(Duration::from_secs(5)),
        "first root did not block at the real source-entry boundary"
    );
    first_gate.allow_entries(128);
    let mut second_root = None;
    for _ in 0..1_000 {
        poll_runtime_with_storage(&mut production, &storage).expect("rotate P2 page");
        if let Some(task) = production.recovery.as_ref()
            && task.root_id() != first_root
        {
            second_root = Some(task.root_id().to_owned());
            break;
        }
        std::thread::yield_now();
    }
    let second_root = second_root.expect("second root receives the next P2 page");
    assert_ne!(first_root, second_root);
    let second_active_change_id = match production
        .recovery
        .as_ref()
        .expect("second recovery task")
        .kind()
    {
        RecoveryTaskKind::MetadataInventory { change_id, .. } => *change_id,
        _ => panic!("second root must own metadata inventory recovery"),
    };
    assert_eq!(production.recovery_sources.retained_for_test().len(), 1);
    let first_retained_change_id = *production
        .recovery_sources
        .retained_for_test()
        .keys()
        .next()
        .expect("first root retained change owner");
    assert_eq!(first_retained_change_id, first_active_change_id);
    let first_retained_run_id = production
        .recovery_sources
        .retained_for_test()
        .get(&first_retained_change_id)
        .expect("first root retained source")
        .run_id()
        .to_owned();
    let first_retained_open_count = crate::adapters::source_spool_open_count(first_path);
    assert!(
        first_retained_open_count >= 1,
        "the first root must open a source before retaining its frontier"
    );
    let second_gate = if second_root == "root-a" {
        &root_a_gate
    } else {
        &root_b_gate
    };
    let second_path = if second_root == "root-a" {
        &root_a_path
    } else {
        &root_b_path
    };
    assert!(
        second_gate.wait_until_blocked(Duration::from_secs(5)),
        "second root did not block at the real source-entry boundary"
    );
    assert_eq!(crate::adapters::source_entry_read_count(first_path), 128);
    assert_eq!(crate::adapters::source_entry_read_count(second_path), 0);
    second_gate.allow_entries(128);

    let mut third_root = None;
    for _ in 0..1_000 {
        poll_runtime_with_storage(&mut production, &storage).expect("return to first P2 root");
        if let Some(task) = production.recovery.as_ref()
            && task.root_id() == first_root
        {
            third_root = Some(task.root_id().to_owned());
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(third_root.as_deref(), Some(first_root.as_str()));
    let third_active_change_id = match production
        .recovery
        .as_ref()
        .expect("returned first-root recovery")
        .kind()
    {
        RecoveryTaskKind::MetadataInventory { change_id, .. } => *change_id,
        _ => panic!("returned first root must own metadata inventory recovery"),
    };
    assert_eq!(third_active_change_id, first_retained_change_id);
    assert_eq!(production.recovery_sources.retained_for_test().len(), 1);
    let second_retained_change_id = *production
        .recovery_sources
        .retained_for_test()
        .keys()
        .next()
        .expect("second root retained change owner");
    assert_eq!(second_retained_change_id, second_active_change_id);
    let second_retained_open_count = crate::adapters::source_spool_open_count(second_path);
    assert!(
        second_retained_open_count >= 1,
        "the second root must open a source before retaining its frontier"
    );
    let evidence_connection =
        rusqlite::Connection::open(&storage.catalog_path).expect("open two-root spool evidence");
    let recovery_identity = |change_id: LibraryChangeId| {
        evidence_connection
            .query_row(
                "SELECT queue.id, queue.root_id, queue.status, queue.attempt_count,
                        queue.last_failure_code, authority.run_id
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = queue.id
                 WHERE queue.id = ?1 AND lane.lane = 'p2_recovery'
                   AND authority.retired_unix_ms IS NULL
                 LIMIT 1",
                [i64::try_from(change_id.value()).expect("SQLite change ID")],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .expect("load retained recovery identity")
    };
    let first_retained_identity = recovery_identity(first_retained_change_id);
    let second_retained_identity = recovery_identity(second_retained_change_id);
    let second_retained_run_id = production
        .recovery_sources
        .retained_for_test()
        .get(&second_retained_change_id)
        .expect("second root retained source")
        .run_id()
        .to_owned();
    assert_eq!(first_retained_identity.1, first_root);
    assert_eq!(first_retained_run_id, first_retained_identity.5);
    assert_eq!(second_retained_identity.1, second_root);
    assert_eq!(second_retained_run_id, second_retained_identity.5);
    assert!(
        first_gate.wait_until_blocked(Duration::from_secs(5)),
        "first root did not retain its live iterator for the third raw batch"
    );
    assert_eq!(crate::adapters::source_entry_read_count(first_path), 128);
    assert_eq!(crate::adapters::source_entry_read_count(second_path), 128);
    first_gate.allow_entries(128);
    let third_deadline = std::time::Instant::now() + Duration::from_secs(5);
    while crate::adapters::source_entry_read_count(first_path) != 256 {
        assert!(
            std::time::Instant::now() < third_deadline,
            "first root did not consume its second bounded raw batch"
        );
        std::thread::yield_now();
    }
    let commit_deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        poll_runtime_with_storage(&mut production, &storage)
            .expect("commit the first root's second raw batch");
        if production
            .recovery
            .as_ref()
            .is_some_and(|task| task.root_id() == second_root)
            && second_gate.wait_until_blocked(Duration::from_millis(10))
        {
            break;
        }
        assert!(
            std::time::Instant::now() < commit_deadline,
            "third raw batch did not commit before the next root rotation"
        );
        std::thread::yield_now();
    }

    let active_second_change_id = match production
        .recovery
        .as_ref()
        .expect("overlapping second-root recovery")
        .kind()
    {
        RecoveryTaskKind::MetadataInventory { change_id, .. } => *change_id,
        _ => panic!("second root must own metadata inventory recovery"),
    };
    assert_eq!(active_second_change_id, second_retained_change_id);
    let active_second_identity = recovery_identity(active_second_change_id);
    assert_eq!(active_second_identity.5, second_retained_run_id);
    let second_released_identity = recovery_identity(second_retained_change_id);
    assert_eq!(second_released_identity.0, second_retained_identity.0);
    assert_eq!(second_released_identity.5, second_retained_identity.5);
    assert!(
        production
            .recovery_sources
            .retained_for_test()
            .contains_key(&first_retained_change_id)
    );
    assert!(
        !production
            .recovery_sources
            .retained_for_test()
            .contains_key(&second_retained_change_id)
    );
    let watcher_overlap_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let watcher_authority_count: i64 = evidence_connection
            .query_row(
                "SELECT COUNT(*) FROM library_recovery_authorities
                 WHERE root_id = ?1 AND root_generation = 1
                   AND reason = 'watcher_uncovered_gap'
                   AND retired_unix_ms IS NULL",
                [second_root.as_str()],
                |row| row.get(0),
            )
            .expect("load second-root watcher authority count");
        if watcher_authority_count == 1 {
            break;
        }
        assert!(
            Instant::now() < watcher_overlap_deadline,
            "the second root did not persist its independent watcher-gap authority"
        );
        poll_runtime_with_storage(&mut production, &storage)
            .expect("advance the independent second-root watcher gap");
        assert!(
            production.recovery.as_ref().is_some_and(|task| {
                task.root_id() == second_root
                    && matches!(
                        task.kind(),
                        RecoveryTaskKind::MetadataInventory { change_id, .. }
                            if *change_id == second_retained_change_id
                    )
            }),
            "watcher-gap admission displaced the gated second-root P2 owner"
        );
        std::thread::yield_now();
    }
    let second_recovery_authorities = {
        let mut statement = evidence_connection
            .prepare(
                "SELECT queue.id, queue.status, queue.attempt_count,
                        queue.last_failure_code, authority.run_id, authority.reason,
                        baseline.phase
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = queue.id
                 LEFT JOIN library_persistent_journal_baselines AS baseline
                   ON baseline.change_id = queue.id
                 WHERE queue.root_id = ?1 AND lane.lane = 'p2_recovery'
                   AND authority.retired_unix_ms IS NULL
                 ORDER BY queue.id",
            )
            .expect("prepare recovery authority evidence");
        statement
            .query_map([second_root.as_str()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })
            .expect("query recovery authority evidence")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect recovery authority evidence")
    };
    assert_eq!(
        second_recovery_authorities.len(),
        2,
        "unexpected second-root recovery authorities: {second_recovery_authorities:?}"
    );
    assert!(second_recovery_authorities.iter().any(|authority| {
        authority.0 == i64::try_from(second_retained_change_id.value()).expect("retained ID")
            && authority.4 == second_retained_run_id
    }));
    assert!(
        second_recovery_authorities
            .iter()
            .any(|authority| authority.5 == "watcher_uncovered_gap")
    );
    assert_eq!(
        crate::adapters::source_spool_open_count(first_path),
        first_retained_open_count,
        "rotation reopened the retained first-authority source"
    );
    assert_eq!(
        crate::adapters::source_spool_open_count(second_path),
        second_retained_open_count,
        "rotation reopened the retained second-authority source"
    );
    assert!(crate::adapters::source_peak_staged_window(first_path) <= 128);
    assert!(crate::adapters::source_peak_staged_window(second_path) <= 128);
    for (run_id, expected_count) in [
        (&first_retained_run_id, 256_i64),
        (&second_retained_run_id, 128_i64),
    ] {
        let evidence: (String, String, i64, i64) = evidence_connection
            .query_row(
                "SELECT spool.state, directory.state, directory.source_entry_count,
                        (SELECT COUNT(*)
                         FROM library_metadata_inventory_spool_entries AS entry
                         WHERE entry.run_id = spool.run_id)
                 FROM library_metadata_inventory_spools AS spool
                 JOIN library_metadata_inventory_spool_directories AS directory
                   ON directory.run_id = spool.run_id
                  WHERE spool.run_id = ?1 AND directory.relative_directory = ''",
                [run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load two-root spool frontier");
        assert_eq!(
            evidence,
            (
                "enumerating".to_owned(),
                "enumerating".to_owned(),
                expected_count,
                expected_count,
            )
        );
    }
    second_gate.release();
    production.stop().expect("stop production synchronization");
}
