use rusqlite::{Connection, params};

use crate::domain::{LibraryChangeQueuePolicy, ScanError};

use super::database_error;

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
    connection
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM library_roots AS root
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
             )
         )",
            params![i64::from(policy.max_attempts), now_unix_ms],
            |row| row.get(0),
        )
        .map_err(database_error)
}

#[cfg(test)]
mod tests;
