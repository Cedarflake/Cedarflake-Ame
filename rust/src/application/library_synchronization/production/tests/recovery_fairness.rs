use super::*;

#[cfg(windows)]
#[test]
fn production_candidate_drain_rotates_between_roots_before_revisiting_busy_work() {
    let directory = tempfile::tempdir().expect("test directory");
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
    let generation = crate::domain::LibraryRootGeneration::initial();
    let base_unix_ms = now_unix_ms().expect("fixture clock");
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("fixture catalog");
    for (root_index, (root_id, candidate_count)) in [("root-a", 2_usize), ("root-b", 1_usize)]
        .into_iter()
        .enumerate()
    {
        let source = directory.path().join(root_id);
        std::fs::create_dir_all(&source).expect("source root");
        let root_path = source.to_string_lossy().into_owned();
        let scan_id = format!("candidate-round-robin-scan-{root_id}");
        let run_id = format!("candidate-round-robin-run-{root_id}");
        catalog
            .begin_scan(
                &ScanRequest {
                    scan_id: scan_id.clone(),
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
            .prove_live_only_first_import_handoff_for_test(&scan_id)
            .expect("prove candidate-drain fixture first-import handoff");
        catalog
            .publish_scan(&scan_id, root_id, 0, 0)
            .expect("publish root scan");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.to_owned(),
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
            .lease_authoritative_library_change(root_id, generation, base_unix_ms, policy)
            .expect("lease recovery authority")
            .expect("recovery control");
        catalog
            .authorize_metadata_inventory_recovery(&crate::domain::LibraryRecoveryAuthority {
                change_id: authority.change.id,
                run_id: run_id.clone(),
                root_id: root_id.to_owned(),
                root_generation: generation,
                reason: crate::domain::LibraryRecoveryAuthorityReason::ContainmentFailure,
                opening_boundary: None,
                authorized_unix_ms: base_unix_ms,
                retired_unix_ms: None,
            })
            .expect("persist recovery authority");
        let run = catalog
            .begin_metadata_inventory(&crate::domain::MetadataInventoryRunRequest {
                run_id: run_id.clone(),
                root_id: root_id.to_owned(),
                root_generation: generation,
                epoch: 1,
                scope: crate::domain::MetadataInventoryScope::Root,
                started_unix_ms: base_unix_ms,
            })
            .expect("begin recovery inventory");
        let root_identity = crate::adapters::FileDiscovery::new(&root_path)
            .expect("pin candidate round-robin root")
            .metadata_inventory_root_identity()
            .expect("read candidate round-robin root identity")
            .expect("stable candidate round-robin root identity");
        catalog
            .initialize_metadata_inventory_spool(
                &run,
                &authority,
                &root_identity,
                None,
                None,
                base_unix_ms + 1,
            )
            .expect("persist candidate round-robin root proof");
        let paths = (0..candidate_count)
            .map(|index| format!("missing-{root_index}-{index}.jpg"))
            .collect::<Vec<_>>();
        catalog
            .stage_metadata_inventory_page(
                &run_id,
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
            .expect("stage recovery candidates");
        let intents = paths
            .iter()
            .enumerate()
            .map(
                |(index, relative_path)| crate::domain::LibraryChangeIntent {
                    root_id: root_id.to_owned(),
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
                &run_id,
                &intents,
                &updates,
                base_unix_ms + 2,
                policy,
            )
            .expect("publish owned recovery candidates")
            .expect("authority remains current");
    }
    drop(catalog);

    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.runtime.queue_policy = policy;
    poll_runtime_with_storage(&mut production, &storage).expect("schedule first candidate root");
    assert_eq!(
        production.recovery.as_ref().map(|task| task.root_id()),
        Some("root-a")
    );
    assert!(matches!(
        production.recovery.as_ref().map(|task| task.kind()),
        Some(RecoveryTaskKind::CandidateDrain { .. })
    ));

    let mut rotated = false;
    for _ in 0..200 {
        std::thread::sleep(Duration::from_millis(5));
        poll_runtime_with_storage(&mut production, &storage)
            .expect("rotate production candidate root");
        if production
            .recovery
            .as_ref()
            .is_some_and(|task| task.root_id() == "root-b")
        {
            rotated = true;
            break;
        }
    }
    assert!(
        rotated,
        "root-b must run before root-a's second owned candidate"
    );
    let connection = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open round-robin evidence catalog");
    let root_a_evidence: (i64, i64) = connection
        .query_row(
            "SELECT
               SUM(queue.status = 'completed'),
               SUM(queue.status IN ('pending', 'leased', 'retry_wait'))
             FROM library_metadata_inventory_candidate_owners AS owner
             JOIN library_change_queue AS queue ON queue.id = owner.change_id
             WHERE owner.run_id = 'candidate-round-robin-run-root-a'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load busy root evidence");
    assert_eq!(root_a_evidence, (1, 1));
    production.stop().expect("stop production synchronization");
}

#[cfg(windows)]
#[test]
fn legacy_unowned_drain_pages_rotate_fairly_across_roots() {
    let directory = tempfile::tempdir().expect("test directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
    let generation = LibraryRootGeneration::initial();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 1,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let mut roots = Vec::new();
    for (root_index, root_id) in ["legacy-root-a", "legacy-root-b"].into_iter().enumerate() {
        let root_path = directory.path().join(root_id);
        std::fs::create_dir_all(&root_path).expect("source root");
        let root_path = root_path.to_string_lossy().into_owned();
        let scan_id = format!("scan-{root_id}");
        catalog
            .begin_scan(
                &ScanRequest {
                    scan_id: scan_id.clone(),
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
            .prove_live_only_first_import_handoff_for_test(&scan_id)
            .expect("prove legacy-drain fixture first-import handoff");
        catalog
            .publish_scan(&scan_id, root_id, 0, 0)
            .expect("publish root scan");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                    scope: crate::domain::LibraryChangeScope::Path,
                    relative_path: format!("legacy-{root_index}.jpg"),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                    first_observed_unix_ms: 100,
                    most_recent_observed_unix_ms: 100,
                    first_sequence: u64::try_from(root_index + 1).expect("sequence"),
                    most_recent_sequence: u64::try_from(root_index + 1).expect("sequence"),
                    coalesced_observation_count: 1,
                }],
                100,
                policy,
            )
            .expect("seed zero-owner recovery debt");
        roots.push(crate::domain::LibraryRootSynchronizationStatus {
            root_id: root_id.to_owned(),
            root_generation: generation.value(),
            availability: crate::domain::LibraryRootAvailability::Available,
            freshness: crate::domain::CatalogFreshnessState::Updating,
            freshness_cause: crate::domain::CatalogFreshnessCause::PendingChanges,
            continuity: PersistentJournalContinuityState::RecoveryRequired,
            phase: LibrarySynchronizationPhase::Reconciliation,
            source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
            queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
            pending_change_count: 1,
            retry_wait_count: 0,
            freshness_unknown_count: 0,
            recovery_blocked: false,
            last_issue_code: None,
        });
    }
    let snapshot = LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 0,
        applied_mutation_count: 0,
        roots,
    };
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.runtime.queue_policy = policy;

    let first = ready_recovery_work(&mut production, &catalog, &snapshot, 100)
        .expect("select first legacy page")
        .expect("first root has legacy debt");
    let second = ready_recovery_work(&mut production, &catalog, &snapshot, 100)
        .expect("select second legacy page")
        .expect("second root has legacy debt");

    assert!(matches!(
        first,
        ReadyRecoveryWork::LegacyUnownedDrain { .. }
    ));
    assert!(matches!(
        second,
        ReadyRecoveryWork::LegacyUnownedDrain { .. }
    ));
    assert_ne!(first.root_id(), second.root_id());
}

#[cfg(windows)]
#[test]
fn recovery_blocked_root_is_excluded_from_p2_while_other_roots_keep_rotating() {
    let directory = tempfile::tempdir().expect("test directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
    let generation = LibraryRootGeneration::initial();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 1,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let mut roots = Vec::new();
    for (index, (root_id, recovery_blocked)) in [("blocked-root", true), ("ready-root", false)]
        .into_iter()
        .enumerate()
    {
        let root_path = directory.path().join(root_id);
        std::fs::create_dir_all(&root_path).expect("source root");
        let root_path = root_path.to_string_lossy().into_owned();
        let scan_id = format!("scan-{root_id}");
        catalog
            .begin_scan(
                &ScanRequest {
                    scan_id: scan_id.clone(),
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
            .prove_live_only_first_import_handoff_for_test(&scan_id)
            .expect("prove recovery-blocked fixture first-import handoff");
        catalog
            .publish_scan(&scan_id, root_id, 0, 0)
            .expect("publish root scan");
        catalog
            .enqueue_library_change_intents(
                &[crate::domain::LibraryChangeIntent {
                    root_id: root_id.to_owned(),
                    root_generation: generation,
                    kind: crate::domain::LibraryChangeIntentKind::Reconcile,
                    scope: crate::domain::LibraryChangeScope::Path,
                    relative_path: format!("legacy-{index}.jpg"),
                    previous_relative_path: None,
                    origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                    first_observed_unix_ms: 100,
                    most_recent_observed_unix_ms: 100,
                    first_sequence: u64::try_from(index + 1).expect("sequence"),
                    most_recent_sequence: u64::try_from(index + 1).expect("sequence"),
                    coalesced_observation_count: 1,
                }],
                100,
                policy,
            )
            .expect("seed legacy unowned P2 debt");
        assert!(
            catalog
                .has_ready_legacy_unowned_recovery_debt(root_id, generation, 100, policy,)
                .expect("legacy debt readiness")
        );
        roots.push(crate::domain::LibraryRootSynchronizationStatus {
            root_id: root_id.to_owned(),
            root_generation: generation.value(),
            availability: crate::domain::LibraryRootAvailability::Available,
            freshness: if recovery_blocked {
                crate::domain::CatalogFreshnessState::NeedsReconciliation
            } else {
                crate::domain::CatalogFreshnessState::Updating
            },
            freshness_cause: if recovery_blocked {
                crate::domain::CatalogFreshnessCause::EvidenceGap
            } else {
                crate::domain::CatalogFreshnessCause::PendingChanges
            },
            continuity: PersistentJournalContinuityState::RecoveryRequired,
            phase: if recovery_blocked {
                LibrarySynchronizationPhase::Blocked
            } else {
                LibrarySynchronizationPhase::Reconciliation
            },
            source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
            queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
            pending_change_count: 1,
            retry_wait_count: 0,
            freshness_unknown_count: if recovery_blocked { 1 } else { 0 },
            recovery_blocked,
            last_issue_code: recovery_blocked
                .then(|| "live_gap_v30_explicit_recovery_required".to_owned()),
        });
    }
    let snapshot = LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 0,
        applied_mutation_count: 0,
        roots,
    };
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.runtime.queue_policy = policy;

    let blocked_only = LibrarySynchronizationSnapshot {
        roots: vec![snapshot.roots[0].clone()],
        ..snapshot.clone()
    };
    assert!(
        ready_recovery_work(&mut production, &catalog, &blocked_only, 100)
            .expect("inspect blocked root")
            .is_none(),
        "a recovery-blocked root must not receive candidate, legacy, or control P2 work"
    );

    let first = ready_recovery_work(&mut production, &catalog, &snapshot, 100)
        .expect("rotate past blocked root")
        .expect("ready root remains eligible");
    assert!(matches!(
        &first,
        ReadyRecoveryWork::LegacyUnownedDrain { .. }
    ));
    assert_eq!(first.root_id(), "ready-root");
    let second = ready_recovery_work(&mut production, &catalog, &snapshot, 100)
        .expect("keep rotating past blocked root")
        .expect("ready root remains eligible on the next rotation");
    assert_eq!(second.root_id(), "ready-root");

    production
        .start_legacy_unowned_recovery_drain(
            first.root_id().to_owned(),
            generation,
            0,
            100,
            catalog.catalog_path().to_path_buf(),
        )
        .expect("start selected ready-root P2 work");
    let deadline = Instant::now() + Duration::from_secs(5);
    while production.recovery.is_some() {
        std::thread::sleep(Duration::from_millis(5));
        production
            .poll_recovery(100)
            .expect("finish selected ready-root P2 work");
        assert!(
            Instant::now() < deadline,
            "selected ready-root P2 work did not finish"
        );
    }

    let attempt_counts: (i64, i64) = rusqlite::Connection::open(catalog.catalog_path())
        .expect("open blocked attempt evidence")
        .query_row(
            "SELECT
               MAX(CASE WHEN root_id = 'blocked-root' THEN attempt_count END),
               MAX(CASE WHEN root_id = 'ready-root' THEN attempt_count END)
             FROM library_change_queue",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load mixed-root attempt counts");
    assert_eq!(
        attempt_counts,
        (0, 1),
        "automatic P2 dispatch must lease the ready root without retrying blocked legacy debt"
    );

    let mut unblocked_snapshot = snapshot;
    unblocked_snapshot.roots[0].recovery_blocked = false;
    unblocked_snapshot.roots[0].freshness = crate::domain::CatalogFreshnessState::Updating;
    unblocked_snapshot.roots[0].freshness_cause =
        crate::domain::CatalogFreshnessCause::PendingChanges;
    unblocked_snapshot.roots[0].phase = LibrarySynchronizationPhase::Reconciliation;
    unblocked_snapshot.roots[0].freshness_unknown_count = 0;
    unblocked_snapshot.roots[0].last_issue_code = None;
    let released = ready_recovery_work(&mut production, &catalog, &unblocked_snapshot, 100)
        .expect("inspect released root")
        .expect("released root regains P2 eligibility");
    assert!(matches!(
        &released,
        ReadyRecoveryWork::LegacyUnownedDrain { .. }
    ));
    assert_eq!(released.root_id(), "blocked-root");
}
