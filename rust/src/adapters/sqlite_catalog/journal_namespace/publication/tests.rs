use super::super::super::SqliteCatalog;
use crate::domain::{
    FileIdentityEvidence, JournalIdentifier, JournalUsn, LibraryRootGeneration,
    PersistentJournalDirectoryChange, PersistentJournalEnrollmentBatch,
    PersistentJournalRangeState, PersistentJournalSourceRange, PersistentJournalVolumeIdentity,
    ScanCheckpoint, ScanRequest, persistent_journal_batch_id,
};
use crate::domain::{
    JournalFileReference, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeScope, PersistentJournalCapability, PersistentJournalCapabilityState,
    PersistentJournalCheckpoint, PersistentJournalContinuityState, PersistentJournalVolumeBatch,
    PersistentJournalVolumePage,
};
use crate::ports::{CatalogRepository, PersistentJournalRepository};

fn catalog() -> (tempfile::TempDir, SqliteCatalog) {
    let storage = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(storage.path().join("catalog.sqlite3")).unwrap();
    let request = ScanRequest {
        scan_id: "scan".to_owned(),
        root_path: storage.path().join("source").to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    catalog
        .begin_scan(&request, "root", &request.root_path)
        .unwrap();
    catalog.claim_next_directory("scan").unwrap();
    catalog
        .complete_directory("scan", &ScanCheckpoint::default())
        .unwrap();
    catalog
        .prove_live_only_first_import_handoff_for_test("scan")
        .unwrap();
    catalog.publish_scan("scan", "root", 0, 0).unwrap();
    (storage, catalog)
}

fn identity(id: u64) -> FileIdentityEvidence {
    FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!("0000000000000001:{id:032x}"),
    }
}

fn insert(catalog: &SqliteCatalog, path: &str, id: u64) {
    let identity = identity(id);
    catalog
        .connection
        .execute(
            "INSERT INTO scan_directory_identities VALUES ('scan', ?1, ?2, ?3)",
            rusqlite::params![path, identity.scheme, identity.value],
        )
        .unwrap();
}

fn paths(catalog: &SqliteCatalog) -> Vec<(String, String)> {
    let mut statement = catalog.connection.prepare("SELECT relative_path, identity_value FROM scan_directory_identities ORDER BY relative_path").unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn batch(changes: Vec<PersistentJournalDirectoryChange>) -> PersistentJournalEnrollmentBatch {
    let intents = changes
        .iter()
        .flat_map(|change| {
            change
                .previous_relative_paths
                .iter()
                .chain(change.current_relative_path.iter())
        })
        .map(|path| LibraryChangeIntent {
            root_id: "root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            kind: LibraryChangeIntentKind::Reconcile,
            scope: LibraryChangeScope::Subtree,
            relative_path: path.clone(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::StartupCatchUp,
            first_observed_unix_ms: 2,
            most_recent_observed_unix_ms: 2,
            first_sequence: 11,
            most_recent_sequence: 11,
            coalesced_observation_count: 1,
        })
        .collect();
    let mut batch = PersistentJournalEnrollmentBatch {
        range: PersistentJournalSourceRange {
            batch_id: "pending".to_owned(),
            root_id: "root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            volume: PersistentJournalVolumeIdentity {
                volume_guid: "volume".to_owned(),
                volume_serial: 1,
            },
            journal_id: JournalIdentifier::new(9).unwrap(),
            requested_start_usn: JournalUsn::new(10).unwrap(),
            requested_end_usn: JournalUsn::new(20).unwrap(),
            covered_until_usn: JournalUsn::new(20).unwrap(),
            is_complete: true,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalRangeState::Enrolled,
            enrolled_unix_ms: 2,
            checkpointed_unix_ms: None,
        },
        intents,
        cross_root_lineage: Vec::new(),
        carried_cross_root_lineage: Vec::new(),
        pending_renames: Vec::new(),
        consumed_pending_rename_ids: Vec::new(),
        directory_changes: changes,
    };
    batch.range.batch_id = persistent_journal_batch_id(&batch);
    batch.validate().unwrap();
    batch
}

fn change(id: u64, old: &str, new: &str) -> PersistentJournalDirectoryChange {
    PersistentJournalDirectoryChange {
        identity: identity(id),
        previous_relative_paths: vec![old.to_owned()],
        current_relative_path: Some(new.to_owned()),
    }
}

#[test]
fn parent_and_child_renames_use_one_immutable_projection() {
    let (_storage, mut catalog) = catalog();
    insert(&catalog, "old", 2);
    insert(&catalog, "old/child", 3);
    insert(&catalog, "old/child/deep", 4);
    let transaction = catalog.begin_write().unwrap();
    super::publish(
        &transaction,
        &batch(vec![
            change(3, "old/child", "new/renamed"),
            change(2, "old", "new"),
        ]),
    )
    .unwrap();
    transaction.commit().unwrap();
    assert_eq!(
        paths(&catalog),
        [
            ("new".to_owned(), identity(2).value),
            ("new/renamed".to_owned(), identity(3).value),
            ("new/renamed/deep".to_owned(), identity(4).value)
        ]
    );
}

#[test]
fn moving_into_a_reused_parent_path_preserves_descendant_identities() {
    let (_storage, mut catalog) = catalog();
    insert(&catalog, "old", 2);
    insert(&catalog, "old/child", 3);
    let created = PersistentJournalDirectoryChange {
        identity: identity(4),
        previous_relative_paths: Vec::new(),
        current_relative_path: Some("old".to_owned()),
    };
    let transaction = catalog.begin_write().unwrap();
    super::publish(
        &transaction,
        &batch(vec![created, change(2, "old", "old/nested")]),
    )
    .unwrap();
    transaction.commit().unwrap();
    assert_eq!(
        paths(&catalog),
        [
            ("old".to_owned(), identity(4).value),
            ("old/nested".to_owned(), identity(2).value),
            ("old/nested/child".to_owned(), identity(3).value)
        ]
    );
}

#[test]
fn rejected_namespace_authority_rolls_back_all_prefix_mutations() {
    let (_storage, mut catalog) = catalog();
    insert(&catalog, "old", 2);
    insert(&catalog, "old/child", 3);
    let before = paths(&catalog);
    catalog.connection.execute_batch("CREATE TRIGGER reject_namespace BEFORE INSERT ON scan_directory_journal_authority BEGIN SELECT RAISE(ABORT, 'controlled failure'); END;").unwrap();
    let transaction = catalog.begin_write().unwrap();
    assert!(super::publish(&transaction, &batch(vec![change(2, "old", "new")])).is_err());
    drop(transaction);
    assert_eq!(paths(&catalog), before);
    let count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM scan_directory_journal_authority",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn queue_namespace_and_checkpoint_are_one_atomic_publication() {
    let (_storage, mut catalog) = catalog();
    insert(&catalog, "old", 2);
    insert(&catalog, "old/child", 3);
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: "root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::Current,
            failure: None,
            updated_unix_ms: 1,
        })
        .unwrap();
    let initial = PersistentJournalCheckpoint {
        root_id: "root".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        volume: PersistentJournalVolumeIdentity {
            volume_guid: "volume".to_owned(),
            volume_serial: 1,
        },
        root_file_reference: JournalFileReference::from_bytes(&[1; 16]).unwrap(),
        journal_id: JournalIdentifier::new(9).unwrap(),
        next_unread_usn: JournalUsn::new(10).unwrap(),
        captured_exclusive_end: JournalUsn::new(10).unwrap(),
        covered_catalog_revision: 1,
        protocol_version: 5,
        contract_version: 1,
        continuity: PersistentJournalContinuityState::Current,
        failure: None,
        updated_unix_ms: 1,
    };
    catalog
        .seed_persistent_journal_checkpoint_for_test(&initial)
        .unwrap();
    let mut next = initial.clone();
    next.next_unread_usn = JournalUsn::new(20).unwrap();
    next.captured_exclusive_end = next.next_unread_usn;
    next.continuity = PersistentJournalContinuityState::CatchingUp;
    next.updated_unix_ms = 2;
    let volume = PersistentJournalVolumeBatch {
        pages: vec![PersistentJournalVolumePage {
            enrollment: batch(vec![change(2, "old", "new")]),
            checkpoint: next,
        }],
    };
    catalog.connection.execute_batch("CREATE TRIGGER reject_atomic_namespace BEFORE INSERT ON scan_directory_journal_authority BEGIN SELECT RAISE(ABORT, 'controlled failure'); END;").unwrap();
    assert!(
        catalog
            .publish_persistent_journal_volume_batch(&volume, 2, Default::default())
            .is_err()
    );
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint("root", LibraryRootGeneration::initial())
            .unwrap(),
        Some(initial.clone())
    );
    assert_eq!(paths(&catalog)[0].0, "old");
    let pending: i64 = catalog.connection.query_row("SELECT COUNT(*) FROM library_change_queue WHERE root_id = 'root' AND status = 'pending'", [], |row| row.get(0)).unwrap();
    assert_eq!(pending, 0);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_atomic_namespace")
        .unwrap();
    catalog
        .publish_persistent_journal_volume_batch(&volume, 2, Default::default())
        .unwrap();
    assert_eq!(
        catalog
            .load_persistent_journal_checkpoint("root", LibraryRootGeneration::initial())
            .unwrap()
            .unwrap()
            .next_unread_usn
            .value(),
        20
    );
    assert_eq!(paths(&catalog)[0].0, "new");
    let pending: i64 = catalog.connection.query_row("SELECT COUNT(*) FROM library_change_queue WHERE root_id = 'root' AND status = 'pending'", [], |row| row.get(0)).unwrap();
    assert_eq!(pending, 2);
}
