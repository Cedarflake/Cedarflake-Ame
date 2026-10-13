use rusqlite::Connection;

use crate::domain::{PersistentJournalBaselinePhase, PersistentJournalVolumeBatch, ScanError};

pub(super) fn validate_publication(
    connection: &Connection,
    batch: &PersistentJournalVolumeBatch,
) -> Result<(), ScanError> {
    for baseline in super::load_active_baselines(connection)? {
        if matches!(
            baseline.phase,
            PersistentJournalBaselinePhase::Replay | PersistentJournalBaselinePhase::Absence
        ) {
            for page in &batch.pages {
                if page.checkpoint.root_id == baseline.root_id
                    && page.checkpoint.root_generation == baseline.root_generation
                {
                    baseline.validate_replay_checkpoint(&page.checkpoint)?;
                }
            }
        }
    }
    Ok(())
}
