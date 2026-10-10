use rusqlite::{OptionalExtension, Transaction, params};

use crate::adapters::sqlite_catalog::change_queue::{
    retained_live_recovery_candidate, retained_live_recovery_query,
};
use crate::domain::{LibraryChangeQueuePolicy, LibraryRootGeneration, ScanError};

use super::{LiveGapRecoveryConsumer, PublicationAuthority, database_error};

#[cfg(test)]
mod tests;

pub(super) fn ensure_publishable(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    authority: &PublicationAuthority,
) -> Result<(), ScanError> {
    // Live reconciliation requires a published baseline. First import has separately proved
    // its opening capture boundary; publishing that baseline does not consume newer Live work.
    if authority.previous_active_scan.is_none() {
        return Ok(());
    }
    let policy = LibraryChangeQueuePolicy::default();
    let recoverable = retained_live_recovery_query("queue.id = changes.id", "?5");
    let generation = LibraryRootGeneration::new(super::sqlite_unsigned(
        authority.root_generation,
        "root generation",
    )?)
    .ok_or_else(|| {
        ScanError::new(
            "catalog_root_generation_invalid",
            "The scan has no valid root generation",
        )
    })?;
    // Recovery cannot drain a full P2 lane while this foreground scan is running. Only a
    // transfer admitted now can retire the P0 blocker before this publication completes.
    let recovery_can_progress =
        retained_live_recovery_candidate(transaction, root_id, generation, policy)?.is_some();
    let blocker = transaction
        .query_row(
            &format!(
                "SELECT changes.status = 'retry_wait'
                      AND changes.attempt_count >= ?5
                      AND changes.next_retry_unix_ms IS NULL
                      AND NOT (?6 AND EXISTS({recoverable})) AS exhausted,
                    changes.last_failure_code
             FROM library_change_queue AS changes
             JOIN library_change_queue_lanes AS lanes ON lanes.change_id = changes.id
             WHERE changes.root_id = ?1 AND changes.root_generation = ?2
               AND lanes.lane = 'p0_live'
               AND (
                 changes.status IN ('pending', 'leased')
                 OR (changes.status = 'retry_wait' AND NOT (
                   changes.scope = 'path' AND changes.intent_kind <> 'freshness_unknown'
                 ))
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_live_gap_recovery_claims AS claims
                 WHERE claims.gap_change_id = changes.id
                   AND claims.consumer_kind = ?3
                   AND claims.foreground_scan_id = ?4
                   AND claims.consumed_unix_ms IS NULL
               )
             ORDER BY exhausted DESC, changes.id
             LIMIT 1"
            ),
            params![
                root_id,
                authority.root_generation,
                LiveGapRecoveryConsumer::ForegroundScan.as_str(),
                scan_id,
                i64::from(policy.max_attempts),
                recovery_can_progress,
            ],
            |row| Ok((row.get::<_, bool>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(database_error)?;
    match blocker {
        None => Ok(()),
        Some((true, failure_code)) => Err(ScanError::new(
            "catalog_scan_live_changes_exhausted",
            format!(
                "The scan cannot publish because live reconciliation exhausted its retries: {}",
                failure_code
                    .as_deref()
                    .unwrap_or("change_failure_unavailable"),
            ),
        )),
        Some((false, _)) => Err(ScanError::new(
            "catalog_scan_live_changes_pending",
            "The scan cannot publish while newer live changes are still unfinished",
        )),
    }
}
