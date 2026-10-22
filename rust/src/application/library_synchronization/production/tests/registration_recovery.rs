use super::*;
use crate::domain::{CatalogFreshnessState, LibraryChangeQueuePolicy};

fn runtime() -> ProductionSynchronization {
    let count = || Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut production = fast_gap_runtime(
        QueuedSourceFactory::default(),
        PersistentChangeJournalConnection::Connected(Arc::new(NoChangeJournalSession {
            register_count: count(),
            query_count: count(),
            shared_read_count: count(),
            close_count: count(),
            client_instances: Arc::new(Mutex::new(Vec::new())),
        })),
    );
    production.persistent_change_journal_caller = Some(crate::journal_broker::CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [12; 16],
    });
    production
}

fn recovery_history(fixture: &ProductionGapFixture) -> Vec<(i64, String)> {
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).unwrap();
    connection
        .prepare(
            "SELECT change_id, phase
             FROM library_persistent_journal_baselines
             WHERE root_id = ?1 AND root_generation = 1
             ORDER BY change_id",
        )
        .unwrap()
        .query_map([&fixture.root_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn retained_registration_failure_recovers_and_stays_current_after_restart() {
    let fixture = ProductionGapFixture::new("registration-recovery", 0);
    seed_current_gap_journal(&fixture);
    let path = fixture.source_root.join("added.png");
    image::RgbImage::from_pixel(2, 3, image::Rgb([20, 40, 80]))
        .save(&path)
        .expect("controlled offline addition");
    let source_bytes = std::fs::read(&path).expect("source preservation baseline");
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    assert!(
        catalog
            .persist_persistent_journal_root_failure(
                &PersistentJournalRootFailure {
                    root_id: fixture.root_id.clone(),
                    root_generation: LibraryRootGeneration::initial(),
                    kind: PersistentJournalRootFailureKind::NonRecoverable,
                    failure: PersistentJournalFailure {
                        code: "persistent_journal_request_invalid".into(),
                        message: "Retained local registration failure".into(),
                    },
                    opening_boundary: None,
                },
                2_000,
                LibraryChangeQueuePolicy::default(),
            )
            .unwrap()
            .is_none()
    );
    drop(catalog);

    let mut production = runtime();
    let initial = poll_runtime_with_storage(&mut production, &fixture.storage).unwrap();
    assert_eq!(initial.roots[0].freshness, CatalogFreshnessState::Updating);
    assert_eq!(
        initial.roots[0].continuity,
        PersistentJournalContinuityState::RecoveryRequired
    );
    assert!(production.journal.is_some());
    let completed = drive_gap_to_synchronized(&mut production, &fixture);
    assert_eq!(
        completed.roots[0].continuity,
        PersistentJournalContinuityState::Current
    );
    assert_eq!(
        completed.roots[0].phase,
        LibrarySynchronizationPhase::Synchronized
    );
    assert!(completed.roots[0].last_issue_code.is_none());
    production.stop().expect("retire recovery runtime");

    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let added = catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "added.png")
        .unwrap()
        .expect("offline addition published before completion");
    assert_eq!((added.width, added.height), (2, 3));
    let checkpoint = catalog
        .load_persistent_journal_checkpoint(&fixture.root_id, LibraryRootGeneration::initial())
        .unwrap()
        .unwrap();
    assert_eq!(
        checkpoint.continuity,
        PersistentJournalContinuityState::Current
    );
    assert!(checkpoint.failure.is_none());
    assert!(
        catalog
            .load_persistent_journal_baselines()
            .unwrap()
            .is_empty()
    );
    let baselines = recovery_history(&fixture);
    assert_eq!(baselines.len(), 1);
    assert_eq!(baselines[0].1, "completed");
    drop(catalog);

    let mut restarted = runtime();
    let checked = drive_gap_to_synchronized(&mut restarted, &fixture);
    assert_eq!(
        checked.roots[0].continuity,
        PersistentJournalContinuityState::Current
    );
    assert!(restarted.recovery.is_none());
    restarted.stop().expect("retire restarted runtime");
    let catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    assert!(
        catalog
            .load_persistent_journal_baselines()
            .unwrap()
            .is_empty()
    );
    assert_eq!(recovery_history(&fixture), baselines);
    assert_eq!(std::fs::read(path).unwrap(), source_bytes);
    fixture.assert_no_automatic_full_scan();
}
