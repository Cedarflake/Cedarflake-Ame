use super::*;

#[cfg(windows)]
#[test]
fn legacy_p2_capacity_debt_allows_only_one_bounded_drain_before_p1_admission() {
    const OWNED_CANDIDATES: usize = 64;
    const LEGACY_UNOWNED_P2: usize = 3_520;

    let directory = tempfile::tempdir().expect("test directory");
    let source = directory.path().join("source");
    std::fs::create_dir_all(&source).expect("source root");
    let root_path = source.to_string_lossy().into_owned();
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
        max_lease_batch: u32::try_from(OWNED_CANDIDATES).expect("lease batch"),
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    assert_eq!(policy.lane_capacity(LibraryChangeLane::Journal), 3_584);
    assert_eq!(policy.lane_capacity(LibraryChangeLane::Recovery), 3_072);
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let run_id = "legacy-capacity-debt";
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: "legacy-capacity-scan".to_owned(),
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 512,
            },
            &root_id,
            &root_path,
        )
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test("legacy-capacity-scan")
        .expect("prove legacy-capacity fixture first-import handoff");
    catalog
        .publish_scan("legacy-capacity-scan", &root_id, 0, 0)
        .expect("publish initial scan");
    let registration =
        describe_production_persistent_journal_root(&root_id, generation.value(), &source)
            .expect("describe journal root");
    let root_reference =
        JournalFileReference::from_bytes(&registration.authorization.root_identity)
            .expect("root reference");
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
        .expect("seed current capability");
    let current_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load root")
        .expect("published root");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
            root_id: root_id.clone(),
            root_generation: generation,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: registration.authorization.volume_id.clone(),
                volume_serial: registration.volume_serial,
            },
            root_file_reference: root_reference,
            journal_id: JournalIdentifier::new(44).expect("journal ID"),
            next_unread_usn: JournalUsn::new(20).expect("next USN"),
            captured_exclusive_end: JournalUsn::new(20).expect("captured end"),
            covered_catalog_revision: current_root.catalog_revision,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: base_unix_ms,
        })
        .expect("seed current checkpoint");
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
                first_observed_unix_ms: base_unix_ms,
                most_recent_observed_unix_ms: base_unix_ms,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            base_unix_ms,
            policy,
        )
        .expect("enqueue recovery control");
    let authority = catalog
        .lease_authoritative_library_change(&root_id, generation, base_unix_ms, policy)
        .expect("lease recovery authority")
        .expect("recovery control");
    catalog
        .authorize_metadata_inventory_recovery(&crate::domain::LibraryRecoveryAuthority {
            change_id: authority.change.id,
            run_id: run_id.to_owned(),
            root_id: root_id.clone(),
            root_generation: generation,
            reason: crate::domain::LibraryRecoveryAuthorityReason::ContainmentFailure,
            opening_boundary: None,
            authorized_unix_ms: base_unix_ms,
            retired_unix_ms: None,
        })
        .expect("persist recovery authority");
    let run = catalog
        .begin_metadata_inventory(&crate::domain::MetadataInventoryRunRequest {
            run_id: run_id.to_owned(),
            root_id: root_id.clone(),
            root_generation: generation,
            epoch: 1,
            scope: crate::domain::MetadataInventoryScope::Root,
            started_unix_ms: base_unix_ms,
        })
        .expect("begin recovery inventory");
    let root_identity = crate::adapters::FileDiscovery::new(&root_path)
        .expect("pin recovery root")
        .metadata_inventory_root_identity()
        .expect("read recovery root identity")
        .expect("stable recovery root identity");
    catalog
        .initialize_metadata_inventory_spool(
            &run,
            &authority,
            &root_identity,
            None,
            None,
            base_unix_ms + 1,
        )
        .expect("persist v28 candidate root proof");
    let paths = (0..OWNED_CANDIDATES)
        .map(|index| format!("owned-{index:04}.jpg"))
        .collect::<Vec<_>>();
    catalog
        .stage_metadata_inventory_page(
            run_id,
            &crate::domain::MetadataInventoryPage {
                page_index: 1,
                entries: paths
                    .iter()
                    .map(|relative_path| crate::domain::MetadataInventoryEntry {
                        relative_path: relative_path.clone(),
                        kind: crate::domain::MetadataInventoryEntryKind::File,
                        file_size: Some(1),
                        modified_unix_ms: base_unix_ms,
                        file_identity: None,
                        source_revision: None,
                        placeholder_state:
                            crate::domain::MetadataInventoryPlaceholderState::Available,
                        is_reparse_point: false,
                    })
                    .collect(),
                cursor: paths.last().cloned(),
                is_complete: true,
                frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                    "", None,
                )],
            },
            base_unix_ms + 1,
        )
        .expect("stage owned candidates");
    let intents = paths
        .iter()
        .enumerate()
        .map(
            |(index, relative_path)| crate::domain::LibraryChangeIntent {
                root_id: root_id.clone(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                scope: crate::domain::LibraryChangeScope::Path,
                relative_path: relative_path.clone(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                first_observed_unix_ms: base_unix_ms,
                most_recent_observed_unix_ms: base_unix_ms,
                first_sequence: u64::try_from(index + 2).expect("candidate sequence"),
                most_recent_sequence: u64::try_from(index + 2).expect("candidate sequence"),
                coalesced_observation_count: 1,
            },
        )
        .collect::<Vec<_>>();
    let updates = paths
        .iter()
        .map(
            |relative_path| crate::domain::MetadataInventoryComparisonUpdate {
                relative_path: relative_path.clone(),
                status: crate::domain::MetadataInventoryComparisonStatus::Enqueued,
                candidate_previous_relative_path: None,
            },
        )
        .collect::<Vec<_>>();
    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            run_id,
            &intents,
            &updates,
            base_unix_ms + 2,
            policy,
        )
        .expect("publish owned candidates")
        .expect("authority remains current");
    drop(catalog);
    let legacy_connection =
        rusqlite::Connection::open(&storage.catalog_path).expect("open migrated debt fixture");
    legacy_connection
        .execute(
            "WITH RECURSIVE sequence(value) AS (
               SELECT 0
               UNION ALL
               SELECT value + 1 FROM sequence WHERE value + 1 < ?1
             )
             INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               previous_relative_path, origin, first_observed_unix_ms,
               most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
               coalesced_observation_count, status, ready_unix_ms,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             )
             SELECT ?2, 1, 'reconcile', 'path', printf('legacy-%04d.jpg', value),
                    NULL, 'metadata_inventory', ?3, ?3,
                    printf('%d', value + 10000), printf('%d', value + 10000),
                    1, 'pending', ?3, 0, ?3, ?3
             FROM sequence",
            rusqlite::params![
                i64::try_from(LEGACY_UNOWNED_P2).expect("legacy count"),
                root_id,
                base_unix_ms,
            ],
        )
        .expect("seed migrated v26 P2 capacity debt");
    let (raw_lower_count, quota_lower_count): (i64, i64) = legacy_connection
        .query_row(
            "SELECT COUNT(*),
                    SUM(CASE WHEN queue.id <> ?1 THEN 1 ELSE 0 END)
             FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             WHERE queue.status IN ('pending', 'leased', 'retry_wait')
               AND lane.lane <> 'p0_live'",
            [i64::try_from(authority.change.id.value()).expect("authority ID")],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("count legacy debt");
    assert_eq!((raw_lower_count, quota_lower_count), (3_585, 3_584));
    drop(legacy_connection);

    let read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(PriorityJournalSession {
        p1_root_id: root_id.clone(),
        first_usn: 20,
        published_next_usn: Arc::new(std::sync::atomic::AtomicI64::new(21)),
        candidate_count: 1,
        shared_read_count: Arc::clone(&read_count),
        close_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
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
    production.runtime.queue_policy = policy;

    poll_runtime_with_storage(&mut production, &storage)
        .expect("schedule debt-repayment candidate drain");
    assert_eq!(read_count.load(Ordering::Acquire), 0);
    assert!(
        production.journal.is_none(),
        "P1 page must wait for headroom"
    );
    assert!(matches!(
        production.recovery.as_ref().map(|task| task.kind()),
        Some(RecoveryTaskKind::CandidateDrain { .. })
    ));

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut p1_started = false;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        poll_runtime_with_storage(&mut production, &storage).expect("drain debt then admit P1");
        if production.journal.is_some() {
            p1_started = true;
            break;
        }
    }
    assert!(p1_started, "one bounded P2 drain must release P1 admission");
    assert!(
        production
            .recovery
            .as_ref()
            .is_none_or(|task| { matches!(task.kind(), RecoveryTaskKind::CandidateDrain { .. }) }),
        "P1 may overlap only the already-started bounded debt drain"
    );
    let read_deadline = std::time::Instant::now() + Duration::from_secs(5);
    while read_count.load(Ordering::Acquire) == 0 && std::time::Instant::now() < read_deadline {
        std::thread::yield_now();
    }
    let evidence: (i64, i64, i64, i64) =
        rusqlite::Connection::open(&storage.catalog_path)
            .expect("open debt evidence")
            .query_row(
                "SELECT
                   SUM(CASE WHEN owner.run_id = ?1 AND queue.status = 'completed' THEN 1 ELSE 0 END),
                   SUM(CASE WHEN owner.run_id = ?1 AND queue.status <> 'completed' THEN 1 ELSE 0 END),
                   (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
                    WHERE run_id = ?1),
                   (SELECT COUNT(*) FROM library_change_queue
                    WHERE relative_path LIKE 'legacy-%' AND status IN ('pending', 'leased', 'retry_wait'))
                 FROM library_metadata_inventory_candidate_owners AS owner
                 JOIN library_change_queue AS queue ON queue.id = owner.change_id",
                [run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load debt repayment evidence");
    assert_eq!(evidence, (64, 0, 64, 3_520));
    assert_eq!(read_count.load(Ordering::Acquire), 1);
    production.stop().expect("stop production synchronization");
}

#[cfg(windows)]
#[test]
fn exactly_3584_legacy_unowned_p2_rows_release_p1_then_continue_without_loss() {
    const LEGACY_UNOWNED_P2: usize = 3_584;
    const LEASE_BATCH: usize = 64;

    let directory = tempfile::tempdir().expect("test directory");
    let source = directory.path().join("source");
    std::fs::create_dir_all(&source).expect("source root");
    let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
        .expect("capacity root discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical capacity root")
        .to_string_lossy()
        .into_owned();
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("capacity root identity query")
        .expect("capacity root stable identity");
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
        max_lease_batch: u32::try_from(LEASE_BATCH).expect("lease batch"),
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan_with_publication_namespace(
            &ScanRequest {
                scan_id: "zero-owner-capacity-scan".to_owned(),
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
        .prove_live_only_first_import_handoff_for_test("zero-owner-capacity-scan")
        .expect("prove zero-owner fixture first-import handoff");
    catalog
        .publish_scan("zero-owner-capacity-scan", &root_id, 0, 0)
        .expect("publish initial scan");
    let registration =
        describe_production_persistent_journal_root(&root_id, generation.value(), &source)
            .expect("describe journal root");
    let root_reference =
        JournalFileReference::from_bytes(&registration.authorization.root_identity)
            .expect("root reference");
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
        .expect("seed current capability");
    let current_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load root")
        .expect("published root");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
            root_id: root_id.clone(),
            root_generation: generation,
            volume: PersistentJournalVolumeIdentity {
                volume_guid: registration.authorization.volume_id,
                volume_serial: registration.volume_serial,
            },
            root_file_reference: root_reference,
            journal_id: JournalIdentifier::new(44).expect("journal ID"),
            next_unread_usn: JournalUsn::new(20).expect("next USN"),
            captured_exclusive_end: JournalUsn::new(20).expect("captured end"),
            covered_catalog_revision: current_root.catalog_revision,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: base_unix_ms,
        })
        .expect("seed current checkpoint");
    catalog
        .enqueue_library_change_intents(
            &[crate::domain::LibraryChangeIntent {
                root_id: root_id.clone(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                scope: crate::domain::LibraryChangeScope::Path,
                relative_path: "p0-preempts-debt.jpg".to_owned(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: base_unix_ms,
                most_recent_observed_unix_ms: base_unix_ms,
                first_sequence: 50_000,
                most_recent_sequence: 50_000,
                coalesced_observation_count: 1,
            }],
            base_unix_ms,
            policy,
        )
        .expect("P0 reserve remains available");
    drop(catalog);
    let legacy_connection =
        rusqlite::Connection::open(&storage.catalog_path).expect("open capacity debt fixture");
    legacy_connection
        .execute(
            "WITH RECURSIVE sequence(value) AS (
               SELECT 0
               UNION ALL
               SELECT value + 1 FROM sequence WHERE value + 1 < ?1
             )
             INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               previous_relative_path, origin, first_observed_unix_ms,
               most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
               coalesced_observation_count, status, ready_unix_ms,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             )
             SELECT ?2, 1, 'reconcile', 'path', printf('legacy-%04d.jpg', value),
                    NULL, 'metadata_inventory', ?3, ?3,
                    printf('%d', value + 10000), printf('%d', value + 10000),
                    1, 'pending', ?3, 0, ?3, ?3
             FROM sequence",
            rusqlite::params![
                i64::try_from(LEGACY_UNOWNED_P2).expect("legacy count"),
                root_id,
                base_unix_ms,
            ],
        )
        .expect("seed exact zero-owner capacity debt");
    let debt_shape: (i64, i64, i64) = legacy_connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_queue
                WHERE relative_path LIKE 'legacy-%'),
               (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
               (SELECT COUNT(*) FROM library_recovery_authorities)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("zero-owner debt shape");
    assert_eq!(
        debt_shape,
        (
            i64::try_from(LEGACY_UNOWNED_P2).expect("legacy count"),
            0,
            0
        )
    );
    drop(legacy_connection);

    let read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let session: Arc<dyn PersistentChangeJournalSession> = Arc::new(PriorityJournalSession {
        p1_root_id: root_id.clone(),
        first_usn: 20,
        published_next_usn: Arc::new(std::sync::atomic::AtomicI64::new(21)),
        candidate_count: 1,
        shared_read_count: Arc::clone(&read_count),
        close_count: Arc::clone(&close_count),
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

    poll_runtime_with_storage(&mut production, &storage).expect("P0 preempts capacity debt");
    assert!(
        production.live.is_some(),
        "P0 must use its reserved admission"
    );
    assert!(production.journal.is_none());
    assert!(production.recovery.is_none());

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut saw_legacy_drain = false;
    let mut saw_p1 = false;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        poll_runtime_with_storage(&mut production, &storage)
            .expect("drain debt and admit journal work");
        if matches!(
            production.recovery.as_ref().map(|task| task.kind()),
            Some(RecoveryTaskKind::LegacyUnownedDrain { .. })
        ) && !saw_legacy_drain
        {
            saw_legacy_drain = true;
            assert_eq!(
                read_count.load(Ordering::Acquire),
                0,
                "P1 cannot read until one bounded P2 page releases headroom"
            );
        }
        if read_count.load(Ordering::Acquire) > 0 {
            saw_p1 = true;
            break;
        }
    }
    assert!(
        saw_legacy_drain,
        "legacy debt must use the bounded escape path"
    );
    assert!(saw_p1, "one bounded legacy page must release P1 admission");
    assert_eq!(read_count.load(Ordering::Acquire), 1);

    let continue_deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut completed_legacy = 0_i64;
    while std::time::Instant::now() < continue_deadline {
        std::thread::sleep(Duration::from_millis(5));
        poll_runtime_with_storage(&mut production, &storage)
            .expect("resume legacy P2 after journal work");
        completed_legacy = rusqlite::Connection::open(&storage.catalog_path)
            .expect("open continuation evidence")
            .query_row(
                "SELECT COUNT(*) FROM library_change_queue
                 WHERE relative_path LIKE 'legacy-%' AND status = 'completed'",
                [],
                |row| row.get(0),
            )
            .expect("count completed legacy debt");
        if completed_legacy >= i64::try_from(LEASE_BATCH * 2).expect("continued batch count") {
            break;
        }
    }
    assert!(
        completed_legacy >= i64::try_from(LEASE_BATCH * 2).expect("continued batch count"),
        "P2 must continue after the admitted P1 page"
    );
    let evidence: (i64, i64, i64, i64, i64) = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open final debt evidence")
        .query_row(
            "SELECT
                   SUM(status = 'completed'),
                   SUM(status IN ('pending', 'leased', 'retry_wait')),
                   COUNT(*),
                   (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners),
                   (SELECT COUNT(*) FROM library_recovery_authorities)
                 FROM library_change_queue
                 WHERE relative_path LIKE 'legacy-%'",
            [],
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
        .expect("final debt evidence");
    assert_eq!(evidence.0 + evidence.1, evidence.2);
    assert_eq!(
        evidence.2,
        i64::try_from(LEGACY_UNOWNED_P2).expect("legacy count")
    );
    assert_eq!((evidence.3, evidence.4), (0, 0));
    production.stop().expect("stop production synchronization");
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[cfg(windows)]
#[test]
fn exactly_one_legacy_unowned_nonpath_row_runs_once_blocks_durably_and_refresh_supersedes() {
    use crate::ports::PersistentJournalRepository;

    let directory = tempfile::tempdir().expect("test directory");
    let source = directory.path().join("source");
    std::fs::create_dir_all(&source).expect("source root");
    let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
        .expect("legacy root discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical legacy root")
        .to_string_lossy()
        .into_owned();
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("legacy root identity query")
        .expect("legacy root stable identity");
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
                scan_id: "single-legacy-scan".to_owned(),
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
        .prove_live_only_first_import_handoff_for_test("single-legacy-scan")
        .expect("prove single-legacy fixture first-import handoff");
    catalog
        .publish_scan("single-legacy-scan", &root_id, 0, 0)
        .expect("publish initial scan");
    let registration =
        describe_production_persistent_journal_root(&root_id, generation.value(), &source)
            .expect("describe single legacy journal root");
    let checkpoint_volume = PersistentJournalVolumeIdentity {
        volume_guid: registration.authorization.volume_id.clone(),
        volume_serial: registration.volume_serial,
    };
    let checkpoint_reference =
        JournalFileReference::from_bytes(&registration.authorization.root_identity)
            .expect("single legacy root reference");
    let checkpoint_journal_id = JournalIdentifier::new(44).expect("checkpoint journal ID");
    let checkpoint_next_usn = JournalUsn::new(20).expect("checkpoint next USN");
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root_id.clone(),
            root_generation: generation,
            protocol_version: 5,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::RecoveryRequired,
            failure: None,
            updated_unix_ms: base_unix_ms,
        })
        .expect("seed recovery-required journal capability");
    let current_root = catalog
        .load_incremental_catalog_root(&root_id)
        .expect("load single legacy root")
        .expect("published single legacy root");
    catalog
        .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
            root_id: root_id.clone(),
            root_generation: generation,
            volume: checkpoint_volume.clone(),
            root_file_reference: checkpoint_reference.clone(),
            journal_id: checkpoint_journal_id,
            next_unread_usn: checkpoint_next_usn,
            captured_exclusive_end: checkpoint_next_usn,
            covered_catalog_revision: current_root.catalog_revision,
            protocol_version: 5,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            continuity: PersistentJournalContinuityState::RecoveryRequired,
            failure: Some(PersistentJournalFailure {
                code: "legacy-recovery-required".to_owned(),
                message: "Legacy recovery fixture requires an explicit refresh".to_owned(),
            }),
            updated_unix_ms: base_unix_ms,
        })
        .expect("seed recovery-required checkpoint");
    drop(registration);
    drop(catalog);
    let legacy_connection = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open single legacy seed connection");
    legacy_connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms
             ) VALUES (?1, 1, 'reconcile', 'subtree', 'legacy-subtree',
                       'metadata_inventory', ?2, ?2, '1', '1', 1,
                       'pending', ?2, 0, ?2, ?2)",
            rusqlite::params![root_id, base_unix_ms],
        )
        .expect("seed exactly one legacy nonpath row");
    let legacy_change_id = legacy_connection.last_insert_rowid();
    drop(legacy_connection);
    let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("reopen seeded catalog");
    assert!(
        catalog
            .has_ready_legacy_unowned_recovery_debt(&root_id, generation, base_unix_ms, policy,)
            .expect("single legacy debt readiness")
    );
    drop(catalog);

    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
            close_count: Arc::clone(&close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })),
    );
    production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [10; 16],
    });
    production.runtime.queue_policy = policy;
    poll_runtime_with_storage(&mut production, &storage)
        .expect("production schedules the single legacy survivor");
    assert!(matches!(
        production.recovery.as_ref().map(|task| task.kind()),
        Some(RecoveryTaskKind::LegacyUnownedDrain { .. })
    ));

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let blocked_snapshot = loop {
        std::thread::sleep(Duration::from_millis(5));
        let snapshot = poll_runtime_with_storage(&mut production, &storage)
            .expect("finish the single legacy worker");
        let evidence: (String, i64, Option<i64>, Option<String>, i64) =
            rusqlite::Connection::open(&storage.catalog_path)
                .expect("open single legacy evidence")
                .query_row(
                    "SELECT status, attempt_count, next_retry_unix_ms,
                            last_failure_code, updated_unix_ms
                     FROM library_change_queue WHERE id = ?1",
                    [legacy_change_id],
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
                .expect("load single legacy evidence");
        if evidence.0 == "retry_wait"
            && evidence.3.as_deref() == Some("legacy_recovery_authority_missing")
        {
            break (snapshot, evidence);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "single legacy production worker did not block durably"
        );
    };
    assert_eq!(blocked_snapshot.1.1, i64::from(policy.max_attempts));
    assert_eq!(blocked_snapshot.1.2, None);
    let blocked_updated_unix_ms = blocked_snapshot.1.4;
    let stable_snapshot = poll_runtime_with_storage(&mut production, &storage)
        .expect("project blocked single legacy survivor");
    let root_status = stable_snapshot
        .roots
        .iter()
        .find(|root| root.root_id == root_id)
        .expect("single legacy root status");
    assert_eq!(
        root_status.continuity,
        PersistentJournalContinuityState::RecoveryRequired
    );
    assert!(root_status.recovery_blocked);
    assert_eq!(
        root_status.last_issue_code.as_deref(),
        Some("legacy_recovery_authority_missing")
    );
    assert!(production.recovery.is_none());
    production.stop().expect("stop first production epoch");

    let mut reopened = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
            close_count: Arc::clone(&close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })),
    );
    reopened.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [11; 16],
    });
    reopened.runtime.queue_policy = policy;
    for _ in 0..3 {
        let snapshot = poll_runtime_with_storage(&mut reopened, &storage)
            .expect("reopen keeps the legacy survivor blocked");
        assert!(reopened.recovery.is_none());
        assert_eq!(
            snapshot
                .roots
                .iter()
                .find(|root| root.root_id == root_id)
                .expect("reopened root status")
                .continuity,
            PersistentJournalContinuityState::RecoveryRequired
        );
    }
    let reopened_updated_unix_ms: i64 = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open reopened legacy evidence")
        .query_row(
            "SELECT updated_unix_ms FROM library_change_queue WHERE id = ?1",
            [legacy_change_id],
            |row| row.get(0),
        )
        .expect("load reopened legacy evidence");
    assert_eq!(reopened_updated_unix_ms, blocked_updated_unix_ms);
    reopened.stop().expect("stop reopened production epoch");

    let mut catalog =
        SqliteCatalog::open(storage.catalog_path.clone()).expect("open explicit refresh catalog");
    let refresh = catalog
        .persist_persistent_journal_root_failure(
            &PersistentJournalRootFailure {
                root_id: root_id.clone(),
                root_generation: generation,
                kind: PersistentJournalRootFailureKind::ContainmentFailure,
                failure: PersistentJournalFailure {
                    code: "explicit-refresh".to_owned(),
                    message: "Explicit refresh replaces blocked legacy recovery".to_owned(),
                },
                opening_boundary: Some(crate::domain::LibraryRecoveryOpeningBoundary {
                    volume: checkpoint_volume,
                    root_file_reference: checkpoint_reference,
                    journal_id: checkpoint_journal_id,
                    next_usn: checkpoint_next_usn,
                    protocol_version: 5,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                }),
            },
            base_unix_ms.saturating_add(100),
            policy,
        )
        .expect("persist explicit refresh")
        .expect("explicit refresh supersedes blocked legacy survivor");
    drop(catalog);
    let replacement: (String, Option<i64>, String) =
        rusqlite::Connection::open(&storage.catalog_path)
            .expect("open explicit refresh evidence")
            .query_row(
                "SELECT legacy.status, legacy.superseded_by_change_id, replacement.status
             FROM library_change_queue AS legacy
             JOIN library_change_queue AS replacement ON replacement.id = ?2
             WHERE legacy.id = ?1",
                rusqlite::params![
                    legacy_change_id,
                    i64::try_from(refresh.value()).expect("refresh change ID"),
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load explicit refresh replacement");
    assert_eq!(
        replacement,
        (
            "superseded".to_owned(),
            Some(i64::try_from(refresh.value()).expect("refresh change ID")),
            "pending".to_owned(),
        )
    );
}
