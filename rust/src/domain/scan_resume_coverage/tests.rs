use super::*;
use crate::domain::{LibraryChangeId, LibraryRootGeneration, PERSISTENT_JOURNAL_CONTRACT_VERSION};

fn baseline() -> PersistentJournalBaseline {
    PersistentJournalBaseline {
        change_id: LibraryChangeId::new(1).expect("change"),
        run_id: "paused-import".to_owned(),
        root_id: "root".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        volume: PersistentJournalVolumeIdentity {
            volume_guid: "volume-a".to_owned(),
            volume_serial: 7,
        },
        root_file_reference: JournalFileReference::V3([3; 16]),
        journal_id: JournalIdentifier::new(44).expect("journal"),
        opening_next_usn: JournalUsn::new(20).expect("opening"),
        closing_next_usn: None,
        protocol_version: 1,
        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
        phase: PersistentJournalBaselinePhase::Inventory,
        authorized_unix_ms: 1,
        updated_unix_ms: 1,
        completed_unix_ms: None,
    }
}

#[test]
fn retained_interval_accepts_unchanged_and_advanced_journals() {
    for (first, next) in [(0, 20), (20, 20), (20, 50)] {
        let original = baseline();
        let proof = ScanResumeCoverage::verify(
            original.clone(),
            &original.volume,
            &original.root_file_reference,
            original.journal_id,
            JournalUsn::new(first).expect("first"),
            JournalUsn::new(next).expect("next"),
        )
        .expect("coverage")
        .expect("retained interval");
        assert_eq!(proof.baseline(), &original);
    }
}

#[test]
fn changed_identity_expired_interval_and_rewound_journal_require_reconciliation() {
    for case in ["volume", "root", "journal", "expired", "rewound"] {
        let original = baseline();
        let mut observed = original.clone();
        let mut first = 0;
        let mut next = 50;
        match case {
            "volume" => observed.volume.volume_serial += 1,
            "root" => observed.root_file_reference = JournalFileReference::V3([4; 16]),
            "journal" => observed.journal_id = JournalIdentifier::new(45).expect("new journal"),
            "expired" => first = 21,
            "rewound" => next = 19,
            _ => unreachable!(),
        }
        assert!(
            ScanResumeCoverage::verify(
                original,
                &observed.volume,
                &observed.root_file_reference,
                observed.journal_id,
                JournalUsn::new(first).expect("first"),
                JournalUsn::new(next).expect("next"),
            )
            .expect("valid boundary")
            .is_none(),
            "{case} cannot retain the old traversal"
        );
    }
}
