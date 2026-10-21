use super::super::indexed_startup::RecoveryPreparation;
use super::*;
use crate::domain::{
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryRecoveryOpeningBoundary,
    PersistentJournalRootFailure, PersistentJournalRootFailureKind,
};

#[test]
fn journal_failure_control_selects_indexed_startup_once_and_containment_does_not() {
    for (kind, expected) in [
        (
            PersistentJournalRootFailureKind::JournalTrim,
            RecoveryPreparation::IndexedStartup,
        ),
        (
            PersistentJournalRootFailureKind::ContainmentFailure,
            RecoveryPreparation::Verification,
        ),
    ] {
        let fixture = ProductionGapFixture::new("indexed-startup-admission", 0);
        seed_current_gap_journal(&fixture);
        let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
            .expect("open admission catalog");
        let generation = LibraryRootGeneration::initial();
        let checkpoint = catalog
            .load_persistent_journal_checkpoint(&fixture.root_id, generation)
            .expect("read checkpoint")
            .expect("current checkpoint");
        let policy = LibraryChangeQueuePolicy::default();
        let control = catalog
            .persist_persistent_journal_root_failure(
                &PersistentJournalRootFailure {
                    root_id: fixture.root_id.clone(),
                    root_generation: generation,
                    kind,
                    failure: PersistentJournalFailure {
                        code: "indexed_startup_admission_fixture".into(),
                        message: "Controlled journal recovery admission".into(),
                    },
                    opening_boundary: Some(LibraryRecoveryOpeningBoundary {
                        volume: checkpoint.volume,
                        root_file_reference: checkpoint.root_file_reference,
                        journal_id: checkpoint.journal_id,
                        next_usn: checkpoint.next_unread_usn,
                        protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    }),
                },
                2_000,
                policy,
            )
            .expect("persist production failure")
            .expect("admitted control");
        let lease = catalog
            .lease_metadata_inventory_recovery(&fixture.root_id, generation, 2_500, policy)
            .expect("lease recovery")
            .expect("ready recovery");
        assert_eq!(lease.change.id, control);
        assert_eq!(
            lease.change.intent.origin,
            LibraryChangeOrigin::MetadataInventory
        );
        let mut owner = IndexedStartupOwner::default();
        assert_eq!(owner.preparation(&catalog, &lease).unwrap(), expected);
        owner.started(&lease, expected);
        owner.finished(&fixture.root_id);
        assert_eq!(
            owner.preparation(&catalog, &lease).unwrap(),
            RecoveryPreparation::Verification,
        );
    }
}
