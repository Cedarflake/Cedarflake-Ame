use crate::domain::{
    LibraryRecoveryOpeningBoundary, PersistentJournalCheckpoint, PersistentJournalContinuityState,
    PersistentJournalFailure, PersistentJournalReadFailure, PersistentJournalRootFailureKind,
};

pub(crate) fn can_reprobe_journal_failure(checkpoint: &PersistentJournalCheckpoint) -> bool {
    checkpoint.continuity == PersistentJournalContinuityState::RecoveryRequired
        && checkpoint.failure.as_ref().is_some_and(can_reprobe_failure)
}

pub(crate) fn can_reprobe_failure(failure: &PersistentJournalFailure) -> bool {
    matches!(
        failure.code.as_str(),
        "identity_journal_hardlinks_not_supported" | "persistent_journal_request_invalid"
    )
}

pub(super) fn reprobed_failure(
    checkpoint: &PersistentJournalCheckpoint,
    opening: &LibraryRecoveryOpeningBoundary,
) -> Option<PersistentJournalReadFailure> {
    if !can_reprobe_journal_failure(checkpoint) {
        return None;
    }
    // Older local registration failures left no recovery consumer. A newly verified
    // root and journal boundary admit reconstruction, never immediate freshness.
    Some(PersistentJournalReadFailure {
        kind: PersistentJournalRootFailureKind::JournalReconstructionFailure,
        failure: checkpoint.failure.clone()?,
        opening_boundary: Some(Box::new(opening.clone())),
    })
}
