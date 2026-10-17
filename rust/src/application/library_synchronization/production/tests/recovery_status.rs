use super::*;

#[cfg(windows)]
#[test]
fn production_snapshot_projects_an_explicit_v30_claim_as_manual_recovery() {
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
        scan_id: "explicit-projection-initial".to_owned(),
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
        .expect("prove projection fixture first-import handoff");
    catalog
        .publish_scan(&request.scan_id, &root_id, 0, 0)
        .expect("publish initial scan");
    drop(catalog);
    let connection = rusqlite::Connection::open(&storage.catalog_path)
        .expect("open explicit projection fixture");
    connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               41, 41, '1', '1', 1, 'retry_wait', 41, 0, NULL,
               'live_gap_v30_explicit_recovery_required',
               'The ambiguous historical gap requires an explicit library update',
               1, 41, 41
             )",
            [&root_id],
        )
        .expect("insert explicit projection gap");
    let gap_change_id = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               created_unix_ms
             ) VALUES (?1, ?2, 1, 'explicit_recovery_required', 41)",
            rusqlite::params![gap_change_id, root_id],
        )
        .expect("insert explicit projection claim");
    drop(connection);

    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::LiveOnly(
            PersistentChangeJournalLiveOnlyReason::PortableDistribution,
        ),
    );
    let snapshot =
        poll_runtime_with_storage(&mut production, &storage).expect("poll explicit projection");
    let status = snapshot
        .roots
        .iter()
        .find(|status| status.root_id == root_id)
        .expect("explicit projection root");

    assert_eq!(
        status.freshness,
        crate::domain::CatalogFreshnessState::NeedsReconciliation
    );
    assert_eq!(status.phase, LibrarySynchronizationPhase::Blocked);
    assert_eq!(status.queue_health, LibraryChangeQueueHealth::Degraded);
    assert!(status.recovery_blocked);
    assert_eq!(
        status.last_issue_code.as_deref(),
        Some("live_gap_v30_explicit_recovery_required")
    );
    assert!(production.recovery.is_none());
    production.stop().expect("stop explicit projection runtime");
}

#[cfg(windows)]
#[test]
fn active_recovery_projects_its_real_phase_during_transient_catalog_contention() {
    let (_sender, receiver) = mpsc::sync_channel(1);
    let recovery = RecoveryTask::from_test_parts(
        "root-a".to_owned(),
        RecoveryTaskKind::BoundedAuthoritative {
            continuity_revision: 0,
        },
        Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
        Arc::new(AtomicBool::new(false)),
        receiver,
        None,
    );
    let mut snapshot = LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 1,
        applied_mutation_count: 0,
        roots: vec![crate::domain::LibraryRootSynchronizationStatus {
            root_id: "root-a".to_owned(),
            root_generation: 1,
            availability: crate::domain::LibraryRootAvailability::Available,
            freshness: crate::domain::CatalogFreshnessState::Updating,
            freshness_cause: crate::domain::CatalogFreshnessCause::PendingChanges,
            continuity: PersistentJournalContinuityState::CatchingUp,
            phase: LibrarySynchronizationPhase::QueuePublication,
            source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
            queue_health: crate::domain::LibraryChangeQueueHealth::Healthy,
            pending_change_count: 1,
            retry_wait_count: 0,
            freshness_unknown_count: 0,
            recovery_blocked: false,
            last_issue_code: Some("catalog_database_busy".to_owned()),
        }],
    };

    project_active_recovery_as_updating(Some(&recovery), &mut snapshot);

    assert_eq!(
        snapshot.roots[0].freshness,
        crate::domain::CatalogFreshnessState::Updating
    );
    assert_eq!(
        snapshot.roots[0].freshness_cause,
        crate::domain::CatalogFreshnessCause::PendingChanges
    );
    assert_eq!(
        snapshot.roots[0].phase,
        LibrarySynchronizationPhase::Reconciliation
    );
}

#[cfg(windows)]
#[test]
fn active_bounded_recovery_projects_updating_after_nominal_lease_expiry() {
    let (_sender, receiver) = mpsc::sync_channel(1);
    let recovery = RecoveryTask::from_test_parts(
        "root-a".to_owned(),
        RecoveryTaskKind::BoundedAuthoritative {
            continuity_revision: 0,
        },
        Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
        Arc::new(AtomicBool::new(false)),
        receiver,
        None,
    );
    let mut snapshot = LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 1,
        applied_mutation_count: 0,
        roots: vec![crate::domain::LibraryRootSynchronizationStatus {
            root_id: "root-a".to_owned(),
            root_generation: 1,
            availability: crate::domain::LibraryRootAvailability::Available,
            freshness: crate::domain::CatalogFreshnessState::NeedsReconciliation,
            freshness_cause: crate::domain::CatalogFreshnessCause::EvidenceGap,
            continuity: PersistentJournalContinuityState::RecoveryRequired,
            phase: LibrarySynchronizationPhase::Blocked,
            source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
            queue_health: crate::domain::LibraryChangeQueueHealth::Degraded,
            pending_change_count: 1,
            retry_wait_count: 0,
            freshness_unknown_count: 1,
            recovery_blocked: false,
            last_issue_code: None,
        }],
    };

    project_active_recovery_as_updating(Some(&recovery), &mut snapshot);

    assert_eq!(
        snapshot.roots[0].freshness,
        crate::domain::CatalogFreshnessState::Updating
    );
    assert_eq!(
        snapshot.roots[0].phase,
        LibrarySynchronizationPhase::Reconciliation
    );
}

#[cfg(windows)]
#[test]
fn active_recovery_does_not_hide_durable_degraded_queue_state() {
    let (_sender, receiver) = mpsc::sync_channel(1);
    let recovery = RecoveryTask::from_test_parts(
        "root-a".to_owned(),
        RecoveryTaskKind::BoundedAuthoritative {
            continuity_revision: 0,
        },
        Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation)),
        Arc::new(AtomicBool::new(false)),
        receiver,
        None,
    );
    let mut snapshot = LibrarySynchronizationSnapshot {
        is_running: true,
        catalog_revision: 1,
        applied_mutation_count: 0,
        roots: vec![crate::domain::LibraryRootSynchronizationStatus {
            root_id: "root-a".to_owned(),
            root_generation: 1,
            availability: crate::domain::LibraryRootAvailability::Available,
            freshness: crate::domain::CatalogFreshnessState::NeedsReconciliation,
            freshness_cause: crate::domain::CatalogFreshnessCause::EvidenceGap,
            continuity: PersistentJournalContinuityState::RecoveryRequired,
            phase: LibrarySynchronizationPhase::Blocked,
            source_health: crate::domain::LibraryChangeSourceHealth::Healthy,
            queue_health: crate::domain::LibraryChangeQueueHealth::Degraded,
            pending_change_count: 1,
            retry_wait_count: 1,
            freshness_unknown_count: 0,
            recovery_blocked: true,
            last_issue_code: Some("metadata_inventory_enumeration_failed".to_owned()),
        }],
    };

    project_active_recovery_as_updating(Some(&recovery), &mut snapshot);

    assert_eq!(
        snapshot.roots[0].freshness,
        crate::domain::CatalogFreshnessState::NeedsReconciliation
    );
    assert_eq!(
        snapshot.roots[0].last_issue_code.as_deref(),
        Some("metadata_inventory_enumeration_failed")
    );
}
