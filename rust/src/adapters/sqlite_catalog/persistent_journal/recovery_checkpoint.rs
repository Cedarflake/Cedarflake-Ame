use crate::domain::{
    JournalUsn, PersistentJournalBaseline, PersistentJournalCheckpoint,
    PersistentJournalContinuityState, ScanError,
};

pub(super) fn replay_start(
    checkpoint: &PersistentJournalCheckpoint,
    baseline: &PersistentJournalBaseline,
    reason: &str,
    scope: &str,
) -> Result<JournalUsn, ScanError> {
    let replaces_journal = reason == "journal_reset";
    let journal_matches = if replaces_journal {
        checkpoint.journal_id != baseline.journal_id
    } else {
        checkpoint.journal_id == baseline.journal_id
            && checkpoint.next_unread_usn <= baseline.opening_next_usn
            && checkpoint.captured_exclusive_end <= baseline.opening_next_usn
    };
    if checkpoint.continuity != PersistentJournalContinuityState::RecoveryRequired
        || checkpoint.root_id != baseline.root_id
        || checkpoint.root_generation != baseline.root_generation
        || checkpoint.volume != baseline.volume
        || checkpoint.root_file_reference != baseline.root_file_reference
        || checkpoint.protocol_version != baseline.protocol_version
        || checkpoint.contract_version != baseline.contract_version
        || !journal_matches
    {
        return Err(ScanError::new(
            "persistent_journal_baseline_closing_mismatch",
            "The recovery closing boundary no longer extends its last trustworthy checkpoint",
        ));
    }
    let unread_interval_unavailable = replaces_journal
        || reason == "journal_trim"
        || (reason == "journal_reconstruction_failure"
            && checkpoint
                .failure
                .as_ref()
                .is_some_and(|failure| failure.code == "identity_journal_hardlinks_not_supported"));
    match scope {
        // Reconstruct only an unavailable interval. Otherwise replay preserves pending
        // cross-root rename ownership that the inventory alone cannot recover.
        "root" if unread_interval_unavailable => Ok(baseline.opening_next_usn),
        "root" | "subtree" if !unread_interval_unavailable => Ok(checkpoint.next_unread_usn),
        _ => Err(ScanError::new(
            "persistent_journal_baseline_closing_mismatch",
            "The recovery scope cannot reconstruct its journal interval",
        )),
    }
}
