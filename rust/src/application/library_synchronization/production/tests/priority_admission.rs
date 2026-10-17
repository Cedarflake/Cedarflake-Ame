use super::*;

#[cfg(windows)]
#[test]
fn p0_root_and_subtree_use_the_reserved_worker_while_p2_is_full_active_and_retrying() {
    let current_directory = std::env::current_dir().expect("current directory");
    let directory = tempfile::tempdir_in(current_directory).expect("test directory");
    let source = directory.path().join("source");
    std::fs::create_dir_all(source.join("album")).expect("source subtree");
    let discovery = crate::adapters::FileDiscovery::new(&source.to_string_lossy())
        .expect("priority fixture root discovery");
    let root_path = discovery
        .canonical_root()
        .expect("canonical priority fixture root")
        .to_string_lossy()
        .into_owned();
    let publication_identity = discovery
        .metadata_inventory_root_identity()
        .expect("priority fixture root identity query")
        .expect("priority fixture stable root identity");
    let root_id = crate::application::scan_library::stable_id("library-root-v1", &root_path);
    let storage = crate::application::storage::StoragePaths {
        catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
        preview_root: directory.path().join("previews"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: directory.path().join("settings").join("storage.sqlite3"),
    };
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_unresolved_changes: 16,
        max_lease_batch: 1,
        lease_duration_millis: crate::domain::LibraryChangeQueuePolicy::MAX_LEASE_DURATION_MILLIS,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let p2_capacity =
        usize::try_from(policy.lane_capacity(crate::domain::LibraryChangeLane::Recovery))
            .expect("P2 capacity");
    assert_eq!(p2_capacity, 12);
    let p2_candidate_capacity = p2_capacity
        .checked_sub(1)
        .expect("the P2 control lease occupies one recovery slot");
    let generation = crate::domain::LibraryRootGeneration::initial();
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let run_id = "p0-reserved-worker-recovery";
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan_with_publication_namespace(
            &ScanRequest {
                scan_id: "p0-reserved-worker-scan".to_owned(),
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
        .prove_live_only_first_import_handoff_for_test("p0-reserved-worker-scan")
        .expect("prove reserved-worker fixture first-import handoff");
    catalog
        .publish_scan("p0-reserved-worker-scan", &root_id, 0, 0)
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
    catalog
        .begin_metadata_inventory(&crate::domain::MetadataInventoryRunRequest {
            run_id: run_id.to_owned(),
            root_id: root_id.clone(),
            root_generation: generation,
            epoch: 1,
            scope: crate::domain::MetadataInventoryScope::Root,
            started_unix_ms: base_unix_ms,
        })
        .expect("begin recovery inventory");
    let p2_paths = (0..p2_candidate_capacity)
        .map(|index| format!("p2-{index:02}.jpg"))
        .collect::<Vec<_>>();
    catalog
        .stage_metadata_inventory_page(
            run_id,
            &crate::domain::MetadataInventoryPage {
                page_index: 1,
                entries: p2_paths
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
                cursor: p2_paths.last().cloned(),
                is_complete: true,
                frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                    "", None,
                )],
            },
            base_unix_ms + 1,
        )
        .expect("stage full P2 slice");
    let p2_intents = p2_paths
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
                first_sequence: u64::try_from(index + 1).expect("candidate sequence"),
                most_recent_sequence: u64::try_from(index + 1).expect("candidate sequence"),
                coalesced_observation_count: 1,
            },
        )
        .collect::<Vec<_>>();
    let p2_updates = p2_paths
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
            &p2_intents,
            &p2_updates,
            base_unix_ms + 2,
            policy,
        )
        .expect("fill the P2 lane")
        .expect("authority remains current");
    let retry_candidate = catalog
        .lease_metadata_inventory_recovery_candidates(
            &root_id,
            generation,
            base_unix_ms + 3,
            policy,
        )
        .expect("lease retry fixture")
        .pop()
        .expect("owned retry candidate");
    catalog
        .retry_library_change(
            retry_candidate.change.id,
            retry_candidate.lease_generation,
            &crate::domain::LibraryChangeFailure {
                code: "p2_fixture_retry".to_owned(),
                message: "The candidate remains retryable while P0 enters".to_owned(),
            },
            base_unix_ms + 4,
            policy,
        )
        .expect("persist P2 retry state");
    drop(catalog);

    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.runtime.queue_policy = policy;
    poll_runtime_with_storage(&mut production, &storage)
        .expect("start production P2 candidate drain");
    assert!(matches!(
        production.recovery.as_ref().map(|task| task.kind()),
        Some(RecoveryTaskKind::CandidateDrain { .. })
    ));

    for (index, (scope, relative_path)) in [
        (crate::domain::LibraryChangeScope::Subtree, "album"),
        (crate::domain::LibraryChangeScope::Root, ""),
    ]
    .into_iter()
    .enumerate()
    {
        let sequence = u64::try_from(index + 1).expect("P0 sequence");
        let live_unix_ms = base_unix_ms + 10 + i64::from(index as u32);
        let mut catalog =
            SqliteCatalog::open(storage.catalog_path.clone()).expect("P0 enqueue catalog");
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
                    first_observed_unix_ms: live_unix_ms,
                    most_recent_observed_unix_ms: live_unix_ms,
                    first_sequence: sequence,
                    most_recent_sequence: sequence,
                    coalesced_observation_count: 1,
                }],
                live_unix_ms,
                policy,
            )
            .expect("P0 must enter through the reserved capacity");
        let snapshot = production
            .runtime
            .poll_without_authoritative_recovery(&mut catalog, live_unix_ms, |path| {
                inspect_root_availability(path).availability
            })
            .expect("project P0 scheduling state");
        production
            .schedule_next_live_work(&catalog, &snapshot, live_unix_ms, &storage)
            .expect("schedule reserved P0 worker");
        assert!(
            production.live.is_some(),
            "P0 must own its independent worker"
        );
        if index == 0 {
            assert!(
                production.recovery.is_some(),
                "P0 scheduling must not consume the retained P2 worker slot"
            );
        }
        drop(catalog);

        let mut p0_completed = false;
        for _ in 0..200 {
            std::thread::sleep(Duration::from_millis(5));
            poll_runtime_with_storage(&mut production, &storage)
                .expect("publish P0 authoritative work");
            let connection = rusqlite::Connection::open(&storage.catalog_path)
                .expect("open P0 completion catalog");
            let completed_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM library_change_queue AS queue
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                     WHERE queue.root_id = ?1 AND lane.lane = 'p0_live'
                       AND queue.status = 'completed'",
                    [&root_id],
                    |row| row.get(0),
                )
                .expect("count completed P0 work");
            if completed_count >= i64::try_from(index + 1).expect("P0 completion count") {
                p0_completed = true;
                break;
            }
        }
        assert!(
            p0_completed,
            "P0 root/subtree must publish while P2 remains active"
        );
    }

    let connection =
        rusqlite::Connection::open(&storage.catalog_path).expect("open priority evidence catalog");
    let retry_count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM library_metadata_inventory_candidate_owners AS owner
             JOIN library_change_queue AS queue ON queue.id = owner.change_id
             WHERE owner.run_id = ?1 AND queue.status = 'retry_wait'",
            [run_id],
            |row| row.get(0),
        )
        .expect("count retained P2 retries");
    assert_eq!(retry_count, 1);
    production.stop().expect("stop production synchronization");
}

#[cfg(windows)]
#[test]
fn higher_lane_ready_or_active_blocks_the_next_lower_page_then_p2_resumes() {
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
        max_unresolved_changes: 16,
        max_lease_batch: 1,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let run_id = "lane-boundary-recovery";
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: "lane-boundary-scan".to_owned(),
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
        .prove_live_only_first_import_handoff_for_test("lane-boundary-scan")
        .expect("prove lane-boundary fixture first-import handoff");
    catalog
        .publish_scan("lane-boundary-scan", &root_id, 0, 0)
        .expect("publish initial scan");
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root_id.clone(),
            root_generation: generation,
            protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
            state: PersistentJournalCapabilityState::LiveOnly,
            continuity: PersistentJournalContinuityState::LiveOnly,
            failure: None,
            updated_unix_ms: base_unix_ms,
        })
        .expect("keep the fixture out of baseline acquisition");
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
    catalog
        .begin_metadata_inventory(&crate::domain::MetadataInventoryRunRequest {
            run_id: run_id.to_owned(),
            root_id: root_id.clone(),
            root_generation: generation,
            epoch: 1,
            scope: crate::domain::MetadataInventoryScope::Root,
            started_unix_ms: base_unix_ms,
        })
        .expect("begin recovery inventory");
    catalog
        .stage_metadata_inventory_page(
            run_id,
            &crate::domain::MetadataInventoryPage {
                page_index: 1,
                entries: vec![crate::domain::MetadataInventoryEntry {
                    relative_path: "p2-missing.jpg".to_owned(),
                    kind: crate::domain::MetadataInventoryEntryKind::File,
                    file_size: Some(1),
                    modified_unix_ms: base_unix_ms,
                    file_identity: None,
                    source_revision: None,
                    placeholder_state: crate::domain::MetadataInventoryPlaceholderState::Available,
                    is_reparse_point: false,
                }],
                cursor: Some("p2-missing.jpg".to_owned()),
                is_complete: true,
                frontier: vec![crate::domain::MetadataInventoryFrontierEntry::completed(
                    "", None,
                )],
            },
            base_unix_ms + 1,
        )
        .expect("stage recovery candidate");
    let p2_intent = crate::domain::LibraryChangeIntent {
        root_id: root_id.clone(),
        root_generation: generation,
        kind: crate::domain::LibraryChangeIntentKind::Reconcile,
        scope: crate::domain::LibraryChangeScope::Path,
        relative_path: "p2-missing.jpg".to_owned(),
        previous_relative_path: None,
        origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
        first_observed_unix_ms: base_unix_ms,
        most_recent_observed_unix_ms: base_unix_ms,
        first_sequence: 2,
        most_recent_sequence: 2,
        coalesced_observation_count: 1,
    };
    catalog
        .publish_metadata_inventory_comparison_candidates(
            &authority,
            run_id,
            std::slice::from_ref(&p2_intent),
            &[crate::domain::MetadataInventoryComparisonUpdate {
                relative_path: p2_intent.relative_path.clone(),
                status: crate::domain::MetadataInventoryComparisonStatus::Enqueued,
                candidate_previous_relative_path: None,
            }],
            base_unix_ms + 2,
            policy,
        )
        .expect("publish recovery candidate")
        .expect("authority remains current");
    let mut p1_intent = p2_intent.clone();
    p1_intent.relative_path = "p1-missing.jpg".to_owned();
    p1_intent.origin = crate::domain::LibraryChangeOrigin::StartupCatchUp;
    p1_intent.first_sequence = 3;
    p1_intent.most_recent_sequence = 3;
    let mut p0_intent = p1_intent.clone();
    p0_intent.relative_path = "p0-missing.jpg".to_owned();
    p0_intent.origin = crate::domain::LibraryChangeOrigin::LiveNotification;
    p0_intent.first_sequence = 4;
    p0_intent.most_recent_sequence = 4;
    catalog
        .enqueue_library_change_intents(&[p1_intent, p0_intent], base_unix_ms + 3, policy)
        .expect("enqueue higher lanes");
    let snapshot = LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 0,
        applied_mutation_count: 0,
        roots: vec![crate::domain::LibraryRootSynchronizationStatus {
            root_id: root_id.clone(),
            root_generation: generation.value(),
            availability: crate::domain::LibraryRootAvailability::Available,
            freshness: crate::domain::CatalogFreshnessState::Updating,
            freshness_cause: crate::domain::CatalogFreshnessCause::PendingChanges,
            continuity: PersistentJournalContinuityState::Current,
            phase: LibrarySynchronizationPhase::Reconciliation,
            source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
            queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
            pending_change_count: 3,
            retry_wait_count: 0,
            freshness_unknown_count: 0,
            recovery_blocked: false,
            last_issue_code: None,
        }],
    };
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.runtime.queue_policy = policy;
    production.journal_lane = JournalLane::after_page_for_test();
    assert!(
        ready_recovery_work(&mut production, &catalog, &snapshot, base_unix_ms + 3)
            .expect("inspect P2 readiness")
            .is_none(),
        "ready P0/P1 work must block the next P2 page"
    );
    drop(catalog);

    poll_runtime_with_storage(&mut production, &storage).expect("start P0 work");
    assert!(production.live.is_some(), "P0 must start first");
    assert!(
        production.journal.is_none(),
        "P1 cannot start beside ready P0"
    );
    assert!(
        production.recovery.is_none(),
        "P2 cannot start beside ready P0"
    );

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut saw_p1 = false;
    let mut saw_p2_after_p1 = false;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        poll_runtime_with_storage(&mut production, &storage).expect("drive lane ordering");
        if production.journal.is_some() {
            saw_p1 = true;
            assert!(
                production.recovery.is_none(),
                "P1 active work must block the next P2 page"
            );
        }
        if saw_p1
            && matches!(
                production.recovery.as_ref().map(|task| task.kind()),
                Some(RecoveryTaskKind::CandidateDrain { .. })
            )
        {
            saw_p2_after_p1 = true;
            break;
        }
    }
    assert!(saw_p1, "P1 must start after P0 completes");
    assert!(
        saw_p2_after_p1,
        "P2 must resume without starvation after higher lanes finish"
    );
    let owner_count: i64 = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open owner evidence")
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
             WHERE run_id = ?1",
            [run_id],
            |row| row.get(0),
        )
        .expect("count retained owner");
    assert_eq!(
        owner_count, 1,
        "priority yielding must not lose P2 ownership"
    );
    production.stop().expect("stop production synchronization");
}
