use std::collections::HashMap;

use crate::domain::{
    LibraryRecoveryOpeningBoundary, LibraryRootGeneration, PersistentJournalBaseline,
    PersistentJournalBaselinePhase, PersistentJournalCheckpoint, PersistentJournalContinuityState,
    PersistentJournalFailure, PersistentJournalReadFailure, PersistentJournalRootFailureKind,
    ScanError,
};

#[derive(Default)]
pub(super) struct JournalReadWindows {
    baselines: HashMap<(String, LibraryRootGeneration), PersistentJournalBaseline>,
}

impl JournalReadWindows {
    pub(super) fn new(baselines: Vec<PersistentJournalBaseline>) -> Result<Self, ScanError> {
        let mut windows = Self::default();
        for baseline in baselines {
            baseline.validate()?;
            if matches!(
                baseline.phase,
                PersistentJournalBaselinePhase::Replay | PersistentJournalBaselinePhase::Absence
            ) {
                let key = (baseline.root_id.clone(), baseline.root_generation);
                if windows.baselines.insert(key, baseline).is_some() {
                    return Err(invalid());
                }
            }
        }
        Ok(windows)
    }

    pub(super) fn end(
        &self,
        checkpoint: &PersistentJournalCheckpoint,
        observed_end: i64,
    ) -> Result<i64, ScanError> {
        let key = (checkpoint.root_id.clone(), checkpoint.root_generation);
        let Some(baseline) = self.baselines.get(&key) else {
            if is_partial_catchup(checkpoint) {
                let captured = checkpoint.captured_exclusive_end.value();
                if observed_end < captured {
                    return Err(ScanError::new(
                        "persistent_journal_catchup_window_regressed",
                        "The journal regressed behind the unfinished catch-up window",
                    ));
                }
                return Ok(captured);
            }
            return Ok(observed_end);
        };
        let closing = baseline.closing_next_usn.ok_or_else(invalid)?;
        baseline.validate_replay_checkpoint(checkpoint)?;
        if observed_end < closing.value() {
            return Err(invalid());
        }
        // Later volume writes belong to a new window after this baseline retires.
        Ok(closing.value())
    }

    pub(super) fn admits_shared_end(
        &self,
        checkpoint: &PersistentJournalCheckpoint,
        end: i64,
    ) -> bool {
        let key = (checkpoint.root_id.clone(), checkpoint.root_generation);
        if let Some(baseline) = self.baselines.get(&key) {
            return baseline
                .closing_next_usn
                .is_some_and(|closing| closing.value() == end);
        }
        !is_partial_catchup(checkpoint) || checkpoint.captured_exclusive_end.value() == end
    }

    pub(super) fn blocks_next_window(&self, checkpoint: &PersistentJournalCheckpoint) -> bool {
        let key = (checkpoint.root_id.clone(), checkpoint.root_generation);
        self.baselines.get(&key).is_some_and(|baseline| {
            baseline.closing_next_usn == Some(checkpoint.next_unread_usn)
                && !self.requires_empty_replay(checkpoint)
        })
    }

    pub(super) fn requires_empty_replay(&self, checkpoint: &PersistentJournalCheckpoint) -> bool {
        let key = (checkpoint.root_id.clone(), checkpoint.root_generation);
        self.baselines.get(&key).is_some_and(|baseline| {
            baseline.phase == PersistentJournalBaselinePhase::Replay
                && baseline.closing_next_usn == Some(baseline.opening_next_usn)
                && checkpoint.next_unread_usn == baseline.opening_next_usn
        })
    }
}

fn is_partial_catchup(checkpoint: &PersistentJournalCheckpoint) -> bool {
    checkpoint.continuity == PersistentJournalContinuityState::CatchingUp
        && checkpoint.next_unread_usn < checkpoint.captured_exclusive_end
}

fn invalid() -> ScanError {
    ScanError::new(
        "persistent_journal_baseline_read_window_invalid",
        "The journal read cannot extend an unfinished baseline window",
    )
}

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
    eprintln!("[Ame journal] continuity lost code={code}: {message}");
    PersistentJournalReadFailure {
        kind: PersistentJournalRootFailureKind::JournalTrim,
        failure: PersistentJournalFailure {
            code: code.to_owned(),
            message,
        },
        opening_boundary: Some(Box::new(opening)),
    }
}
