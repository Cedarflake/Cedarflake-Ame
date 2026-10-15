use crate::domain::{
    LibraryChangeLane, PersistentJournalEnrollmentBatch, PersistentJournalVolumeBatch,
};

pub(super) fn write_lane(batch: &PersistentJournalVolumeBatch) -> LibraryChangeLane {
    for page in &batch.pages {
        let PersistentJournalEnrollmentBatch {
            range: _,
            intents,
            cross_root_lineage,
            carried_cross_root_lineage,
            pending_renames,
            consumed_pending_rename_ids,
            directory_changes,
        } = &page.enrollment;
        if !intents.is_empty()
            || !cross_root_lineage.is_empty()
            || !carried_cross_root_lineage.is_empty()
            || !pending_renames.is_empty()
            || !consumed_pending_rename_ids.is_empty()
            || !directory_changes.is_empty()
        {
            return LibraryChangeLane::Journal;
        }
    }
    // Unrelated volume activity advances the cursor without changing a source. Its bookkeeping
    // must not repeatedly roll back an in-flight scan publication. The complete transaction still
    // validates the root, range and checkpoint after admission, and persists them atomically.
    LibraryChangeLane::Recovery
}
