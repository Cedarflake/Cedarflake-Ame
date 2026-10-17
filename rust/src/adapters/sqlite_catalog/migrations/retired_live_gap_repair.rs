use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::compatibility_repair_shape::live_gap_shape_matches;
use super::database_error;

fn repairable_claim_ids(
    connection: &Connection,
    schema_version: i64,
) -> Result<Vec<i64>, ScanError> {
    if !live_gap_shape_matches(connection, schema_version)? {
        return Ok(Vec::new());
    }
    let mut statement = connection
        .prepare(
            "SELECT claim.gap_change_id
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
             JOIN library_change_root_state AS root_state
               ON root_state.root_id = claim.root_id
             WHERE claim.consumed_unix_ms IS NULL
               AND claim.source_range_id IS NULL
               AND claim.recovery_change_id IS NULL
               AND claim.foreground_scan_id IS NULL
               AND gap.root_id = claim.root_id
               AND gap.root_generation = claim.root_generation
               AND gap.intent_kind = 'freshness_unknown'
               AND gap.scope = 'root' AND gap.relative_path = ''
               AND gap.previous_relative_path IS NULL
               AND gap.status = 'superseded'
               AND gap.next_retry_unix_ms IS NULL
               AND gap.lease_expires_unix_ms IS NULL
               AND gap.authoritative_scan_id IS NULL
               AND gap.catalog_revision_at_success IS NULL
               AND gap.catch_up_source IS NULL
               AND gap.catch_up_watermark IS NULL
               AND gap.superseded_by_change_id IS NULL
               AND gap.last_failure_message IS NOT NULL
               AND (
                 (claim.consumer_kind = 'explicit_recovery_required'
                   AND claim.opening_volume_guid IS NULL
                   AND claim.opening_volume_serial IS NULL
                   AND claim.opening_root_reference_version IS NULL
                   AND claim.opening_root_file_reference IS NULL
                   AND claim.opening_journal_id IS NULL
                   AND claim.opening_next_usn IS NULL
                   AND claim.protocol_version IS NULL
                   AND claim.contract_version IS NULL
                   AND gap.origin = 'startup_catch_up'
                   AND lane.lane = 'p1_journal'
                   AND gap.last_failure_code =
                         'live_gap_v30_explicit_recovery_required')
                 OR
                 (claim.consumer_kind = 'pending_journal'
                   AND claim.opening_volume_guid IS NOT NULL
                   AND length(claim.opening_volume_guid) BETWEEN 1 AND 512
                   AND claim.opening_volume_serial IS NOT NULL
                   AND length(claim.opening_volume_serial) BETWEEN 1 AND 20
                   AND claim.opening_volume_serial NOT GLOB '*[^0-9]*'
                   AND claim.opening_root_reference_version IN (2, 3)
                   AND claim.opening_root_file_reference IS NOT NULL
                   AND length(claim.opening_root_file_reference) = CASE
                     claim.opening_root_reference_version WHEN 2 THEN 8 ELSE 16 END
                   AND claim.opening_journal_id IS NOT NULL
                   AND length(claim.opening_journal_id) BETWEEN 1 AND 20
                   AND claim.opening_journal_id <> '0'
                   AND claim.opening_journal_id NOT GLOB '*[^0-9]*'
                   AND claim.opening_next_usn IS NOT NULL
                   AND length(claim.opening_next_usn) BETWEEN 1 AND 19
                   AND claim.opening_next_usn NOT GLOB '*[^0-9]*'
                   AND claim.protocol_version BETWEEN 1 AND 65535
                   AND claim.contract_version = 1
                   AND gap.origin = 'live_notification'
                   AND lane.lane = 'p0_live'
                   AND gap.last_failure_code = 'live_gap_waiting_for_journal_range')
               )
               AND (
                 (root_state.is_active = 0
                   AND root_state.generation >= claim.root_generation
                   AND NOT EXISTS(
                     SELECT 1 FROM library_roots AS root
                     WHERE root.id = claim.root_id
                   ))
                 OR
                 (root_state.is_active = 1
                   AND root_state.generation > claim.root_generation
                   AND EXISTS(
                     SELECT 1 FROM library_roots AS root
                     WHERE root.id = claim.root_id
                   ))
               )
             ORDER BY claim.gap_change_id",
        )
        .map_err(database_error)?;
    statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)
}

pub(super) fn needed(connection: &Connection, schema_version: i64) -> Result<bool, ScanError> {
    Ok(!repairable_claim_ids(connection, schema_version)?.is_empty())
}

pub(super) fn apply(transaction: &Transaction<'_>, schema_version: i64) -> Result<(), ScanError> {
    for gap_change_id in repairable_claim_ids(transaction, schema_version)? {
        let deleted = transaction
            .execute(
                "DELETE FROM library_live_gap_recovery_claims
                 WHERE gap_change_id = ?1
                   AND consumer_kind IN (
                     'explicit_recovery_required', 'pending_journal'
                   )
                   AND consumed_unix_ms IS NULL",
                [gap_change_id],
            )
            .map_err(database_error)?;
        if deleted != 1 {
            return Err(ScanError::new(
                "catalog_live_gap_recovery_repair_conflict",
                "The retired live-gap claim changed during catalog repair",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) mod test_catalog;
