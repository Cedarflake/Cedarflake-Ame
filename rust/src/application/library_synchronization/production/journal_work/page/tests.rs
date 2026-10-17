use super::*;
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, PERSISTENT_JOURNAL_CONTRACT_VERSION,
    PersistentJournalCapability, PersistentJournalCapabilityState, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, PersistentJournalVolumeIdentity, ScanRequest,
};
use crate::ports::{CatalogRepository, LibraryChangeQueue};

#[test]
fn local_p1_registration_failure_is_durable_and_root_isolated() {
    let directory = tempfile::tempdir().expect("test directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let generation = LibraryRootGeneration::initial();
    let policy = crate::domain::LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..crate::domain::LibraryChangeQueuePolicy::default()
    };
    let mut catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
    for (root_id, marker) in [("bad-root", 1_u8), ("healthy-root", 2_u8)] {
        let root_path = directory.path().join(root_id);
        std::fs::create_dir_all(&root_path).expect("create root");
        let root_path = root_path.to_string_lossy().into_owned();
        let request = ScanRequest {
            scan_id: format!("scan-{root_id}"),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        catalog
            .begin_scan(&request, root_id, &root_path)
            .expect("begin root scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove P1 fixture first-import handoff");
        catalog
            .publish_scan(&request.scan_id, root_id, 0, 0)
            .expect("publish root scan");
        catalog
            .save_persistent_journal_capability(&PersistentJournalCapability {
                root_id: root_id.to_owned(),
                root_generation: generation,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: PersistentJournalCapabilityState::Supported,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1,
            })
            .expect("seed current capability");
        catalog
            .seed_persistent_journal_checkpoint_for_test(&PersistentJournalCheckpoint {
                root_id: root_id.to_owned(),
                root_generation: generation,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: "registration-volume".to_owned(),
                    volume_serial: 7,
                },
                root_file_reference: JournalFileReference::V2([marker; 8]),
                journal_id: JournalIdentifier::new(44).expect("journal ID"),
                next_unread_usn: JournalUsn::new(20).expect("next USN"),
                captured_exclusive_end: JournalUsn::new(20).expect("captured end"),
                covered_catalog_revision: 1,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 1,
            })
            .expect("seed current checkpoint");
    }

    let failure = persist_journal_registration_failure(
        &mut catalog,
        "bad-root",
        generation,
        PersistentChangeJournalOperationError::InvalidRequest,
        1_000,
        policy,
    )
    .expect("persist typed registration failure");

    assert_eq!(
        failure.kind,
        PersistentJournalRootFailureKind::NonRecoverable
    );
    let bad = catalog
        .load_persistent_journal_checkpoint("bad-root", generation)
        .expect("load bad checkpoint")
        .expect("bad checkpoint");
    let healthy = catalog
        .load_persistent_journal_checkpoint("healthy-root", generation)
        .expect("load healthy checkpoint")
        .expect("healthy checkpoint");
    assert_eq!(
        bad.continuity,
        PersistentJournalContinuityState::RecoveryRequired
    );
    assert_eq!(
        healthy.continuity,
        PersistentJournalContinuityState::Current
    );
    assert_eq!(bad.next_unread_usn.value(), 20);
    let metrics = catalog
        .load_library_change_root_queue_metrics("bad-root", generation, 1_000, policy)
        .expect("load bad-root queue metrics");
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.retry_wait_count, 0);
}
