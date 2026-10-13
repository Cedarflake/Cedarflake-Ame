use rusqlite::{Transaction, params};

use crate::domain::ScanError;

use super::super::super::database_error;

pub(super) fn retire(
    transaction: &Transaction<'_>,
    root_id: &str,
    generation: i64,
) -> Result<(), ScanError> {
    let count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM (
           SELECT 1 FROM library_live_gap_recovery_claims
           WHERE root_id = ?1 AND root_generation = ?2
             AND consumer_kind = 'journal_source_range' LIMIT 257
         )",
            params![root_id, generation],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if count > 256 {
        return Err(ScanError::new(
            "root_location_inventory_cleanup_required",
            "Directory recovery requires bounded retirement of previous journal claims",
        ));
    }
    // The old range cannot remain a coverage consumer after generation retirement supersedes it.
    // Preserve the gap/range history; only their now-obsolete execution relationship is released.
    let removed = transaction.execute(
        "DELETE FROM library_live_gap_recovery_claims
         WHERE root_id = ?1 AND root_generation = ?2 AND consumer_kind = 'journal_source_range'
           AND gap_change_id IN (
             SELECT claim.gap_change_id FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN library_persistent_journal_source_ranges AS ranges ON ranges.id = claim.source_range_id
             JOIN library_persistent_journal_range_lifecycle AS lifecycle ON lifecycle.source_range_id = ranges.id
             WHERE claim.root_id = ?1 AND claim.root_generation = ?2
               AND claim.consumer_kind = 'journal_source_range'
               AND gap.root_id = ?1 AND gap.root_generation = ?2 AND gap.status = 'superseded'
               AND ranges.root_id = ?1 AND ranges.root_generation = ?2 AND ranges.status = 'checkpointed'
               AND lifecycle.lifecycle_state IN ('pending', 'completed')
           )",
        params![root_id, generation],
    ).map_err(database_error)?;
    if i64::try_from(removed).ok() != Some(count) {
        return Err(ScanError::new(
            "root_location_journal_claim_conflict",
            "The previous journal claim no longer has its expected generation and consumer",
        ));
    }
    Ok(())
}
