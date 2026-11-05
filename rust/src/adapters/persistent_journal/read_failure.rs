use std::io::Write;

use crate::domain::{
    LibraryRecoveryOpeningBoundary, PersistentJournalCheckpoint, PersistentJournalFailure,
    PersistentJournalReadFailure, PersistentJournalRootFailureKind,
};

pub(super) fn trimmed_failure(
    code: &str,
    checkpoint: &PersistentJournalCheckpoint,
    first_usn: i64,
    opening: LibraryRecoveryOpeningBoundary,
) -> PersistentJournalReadFailure {
    let message = format!(
        "The retained journal begins after the saved unread position: \
         checkpoint_usn={}, first_usn={}, next_usn={}, checkpoint_updated_unix_ms={}",
        checkpoint.next_unread_usn.value(),
        first_usn,
        opening.next_usn.value(),
        checkpoint.updated_unix_ms,
    );
    // A recovered checkpoint later clears its failure. Retain the observed numeric
    // bounds in the diagnostic log without exposing source paths or volume identity.
    let _ = writeln!(
        std::io::stderr().lock(),
        "[Ame journal] continuity lost code={code}: {message}"
    );
    PersistentJournalReadFailure {
        kind: PersistentJournalRootFailureKind::JournalTrim,
        failure: PersistentJournalFailure {
            code: code.to_owned(),
            message,
        },
        opening_boundary: Some(Box::new(opening)),
    }
}
