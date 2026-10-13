use std::collections::HashMap;

use crate::domain::{
    LibraryRootGeneration, PersistentJournalBaseline, PersistentJournalBaselinePhase,
    PersistentJournalCheckpoint, ScanError,
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
        self.baselines.get(&key).is_none_or(|baseline| {
            baseline
                .closing_next_usn
                .is_some_and(|closing| closing.value() == end)
        })
    }

    pub(super) fn blocks_next_window(&self, checkpoint: &PersistentJournalCheckpoint) -> bool {
        let key = (checkpoint.root_id.clone(), checkpoint.root_generation);
        self.baselines
            .get(&key)
            .is_some_and(|baseline| baseline.closing_next_usn == Some(checkpoint.next_unread_usn))
    }
}

fn invalid() -> ScanError {
    ScanError::new(
        "persistent_journal_baseline_read_window_invalid",
        "The journal read cannot extend an unfinished baseline window",
    )
}
