use super::LibraryChangeLeaseUpdateOutcome;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalScopeRecoveryOutcome {
    Deferred,
    Completed,
    Rejected(LibraryChangeLeaseUpdateOutcome),
}
