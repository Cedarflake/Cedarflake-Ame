use super::*;
use crate::journal_broker::service::JournalReadProof;

fn metadata() -> ExistingJournalMetadata {
    ExistingJournalMetadata {
        journal_id: 7,
        first_usn: 0,
        next_usn: 20,
    }
}

fn empty_page() -> SharedBackendJournalPage {
    SharedBackendJournalPage {
        candidates_by_root: vec![Ok(Vec::new())],
        handoffs: Vec::new(),
        pending_renames: Vec::new(),
        rename_barriers_by_root: vec![None],
        proof: Some(JournalReadProof::after_read(20, true, metadata()).expect("read receipt")),
    }
}

#[test]
fn empty_replay_requires_a_complete_backend_read_and_unchanged_journal() {
    let range = JournalRange {
        start_usn: 20,
        end_usn: 20,
    };
    assert!(validate_shared_backend_page(metadata(), range, &empty_page(), 1).is_ok());
    let mut incomplete = empty_page();
    incomplete.proof =
        Some(JournalReadProof::after_read(20, false, metadata()).expect("partial proof"));
    assert!(validate_shared_backend_page(metadata(), range, &incomplete, 1).is_err());
    let mut reset = empty_page();
    let after = ExistingJournalMetadata {
        journal_id: 8,
        ..metadata()
    };
    reset.proof = Some(JournalReadProof::after_read(20, true, after).expect("reset receipt"));
    assert!(validate_shared_backend_page(metadata(), range, &reset, 1).is_err());
    let mut trimmed = empty_page();
    let after = ExistingJournalMetadata {
        first_usn: 21,
        next_usn: 22,
        ..metadata()
    };
    trimmed.proof = Some(JournalReadProof::after_read(20, true, after).expect("trimmed receipt"));
    assert!(validate_shared_backend_page(metadata(), range, &trimmed, 1).is_err());
}

#[test]
fn absent_or_nonadvancing_backend_proof_cannot_complete_replay() {
    let mut absent = empty_page();
    absent.proof = None;
    let empty = JournalRange {
        start_usn: 20,
        end_usn: 20,
    };
    assert!(validate_shared_backend_page(metadata(), empty, &absent, 1).is_err());
    let mut stalled = empty_page();
    stalled.proof =
        Some(JournalReadProof::after_read(10, false, metadata()).expect("stalled receipt"));
    let nonempty = JournalRange::new(10, 20).expect("nonempty interval");
    assert!(validate_shared_backend_page(metadata(), nonempty, &stalled, 1).is_err());
}
