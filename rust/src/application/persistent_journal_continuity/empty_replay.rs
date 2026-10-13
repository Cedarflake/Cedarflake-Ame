use crate::domain::{
    PersistentJournalCheckpoint, PersistentJournalFailure, PersistentJournalReadFailure,
    PersistentJournalRootFailureKind,
};
use crate::ports::PersistentJournalRepository;

pub(super) fn record<Repository: PersistentJournalRepository>(
    repository: &mut Repository,
    previous: &PersistentJournalCheckpoint,
    proof: &PersistentJournalCheckpoint,
    observed_unix_ms: i64,
) -> Result<(), PersistentJournalReadFailure> {
    if proof.root_id != previous.root_id
        || proof.root_generation != previous.root_generation
        || proof.volume != previous.volume
        || proof.root_file_reference != previous.root_file_reference
        || proof.journal_id != previous.journal_id
        || proof.protocol_version != previous.protocol_version
        || proof.contract_version != previous.contract_version
        || proof.next_unread_usn != previous.next_unread_usn
        || proof.captured_exclusive_end != proof.next_unread_usn
    {
        return Err(super::reconstruction_failure(
            "persistent_journal_empty_replay_invalid",
            "The empty replay does not prove the requested root and exact cursor",
        ));
    }
    repository
        .record_persistent_journal_empty_replay(proof, observed_unix_ms)
        .map(|_| ())
        .map_err(|error| PersistentJournalReadFailure {
            kind: PersistentJournalRootFailureKind::Transient,
            failure: PersistentJournalFailure {
                code: error.code,
                message: error.message,
            },
            opening_boundary: None,
        })
}
