use rusqlite::{Connection, params};

use crate::domain::{LibraryChangeQueuePolicy, ScanError};

use super::super::change_queue::first_import_replay::blocks_unlineaged_live_changes;
use super::{database_error, parse_root_generation};

pub(super) fn should_yield(
    connection: &Connection,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<bool, ScanError> {
    if !policy.is_valid() || now_unix_ms < 0 {
        return Err(ScanError::new(
            "journal_capture_priority_input_invalid",
            "Journal capture priority requires a valid queue policy and observation time",
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT root.id, active.generation,
                    EXISTS(
                      SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                      WHERE lineage.change_id = queue.id
                        AND lineage.catch_up_source = 'persistent_journal_v1'
                    )
           FROM library_roots AS root
           JOIN library_change_root_state AS active ON active.root_id = root.id
           JOIN library_change_queue AS queue
             ON queue.root_id = root.id AND queue.root_generation = active.generation
           JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
           WHERE root.active_scan_id IS NOT NULL AND active.is_active = 1
             AND lane.lane = 'p0_live' AND queue.origin = 'live_notification'
             AND queue.status IN ('pending', 'leased', 'retry_wait')
             AND queue.intent_kind <> 'freshness_unknown'
             AND (
               queue.status = 'leased'
               OR (queue.attempt_count < ?1 AND queue.status = 'pending'
                 AND queue.ready_unix_ms <= ?2)
               OR (queue.attempt_count < ?1 AND queue.status = 'retry_wait'
                 AND queue.next_retry_unix_ms IS NOT NULL AND queue.next_retry_unix_ms <= ?2)
             )",
        )
        .map_err(database_error)?;
    let candidates = statement
        .query_map(
            params![i64::from(policy.max_attempts), now_unix_ms],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .map_err(database_error)?;
    for candidate in candidates {
        let (root_id, generation, has_journal_lineage) = candidate.map_err(database_error)?;
        let generation = parse_root_generation(generation)?;
        if has_journal_lineage || !blocks_unlineaged_live_changes(connection, &root_id, generation)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests;
