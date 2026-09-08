use rusqlite::{Connection, params};

use crate::domain::{
    LibraryChangeCapacityDeferral, LibraryChangeQueueHealth, LibraryChangeQueueMetrics,
    LibraryChangeQueuePolicy, LibraryRootGeneration, ScanError,
};

use super::super::{database_error, sqlite_integer, sqlite_unsigned};

pub(super) fn load_metrics(
    connection: &Connection,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeQueueMetrics, ScanError> {
    load_filtered_metrics(connection, None, None, now_unix_ms, policy)
}

pub(super) fn load_root_metrics(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeQueueMetrics, ScanError> {
    load_filtered_metrics(
        connection,
        Some(root_id),
        Some(root_generation),
        now_unix_ms,
        policy,
    )
}

fn load_filtered_metrics(
    connection: &Connection,
    root_id: Option<&str>,
    root_generation: Option<LibraryRootGeneration>,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
) -> Result<LibraryChangeQueueMetrics, ScanError> {
    // Casting the integer rowid preserves order without inviting a cross-root reverse scan.
    // Claim ownership is read through the referenced gap, not its redundant root fields.
    let (root_predicate, exhausted_predicate, exhausted_order, explicit_count) =
        if root_id.is_some() {
            (
                "root_id = ?3 AND root_generation = ?4",
                "exhausted_change.root_id = ?3 AND exhausted_change.root_generation = ?4",
                "CAST(exhausted_change.id AS INTEGER) DESC",
                "SELECT COUNT(*) FROM library_change_queue AS claimed_gap
                 WHERE claimed_gap.root_id = ?3 AND claimed_gap.root_generation = ?4
                   AND EXISTS (
                     SELECT 1 FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.gap_change_id = claimed_gap.id
                       AND claim.consumer_kind = 'explicit_recovery_required'
                   )",
            )
        } else {
            (
                "1",
                "1",
                "exhausted_change.id DESC",
                "SELECT COUNT(*) FROM library_live_gap_recovery_claims AS claim
                 JOIN library_change_queue AS claimed_gap ON claimed_gap.id = claim.gap_change_id
                 WHERE claim.consumer_kind = 'explicit_recovery_required'",
            )
        };
    let (
        pending,
        leased,
        retry_wait,
        completed,
        superseded,
        ready,
        expired,
        exhausted,
        freshness_unknown,
        explicit_recovery_required,
        oldest_due,
        latest_exhausted_failure_code,
    ) = connection
        .query_row(
            &format!(
                "SELECT
               COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'leased' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'retry_wait' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'superseded' THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN
                 (status = 'pending' AND ready_unix_ms <= ?1 AND attempt_count < ?2)
                 OR
                 (status = 'retry_wait' AND next_retry_unix_ms IS NOT NULL
                   AND next_retry_unix_ms <= ?1 AND attempt_count < ?2)
                 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'leased' AND lease_expires_unix_ms <= ?1
                 THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN status = 'retry_wait' AND attempt_count >= ?2
                 AND NOT (
                   COALESCE(last_failure_code = ?5, 0)
                   AND origin = 'live_notification'
                   AND intent_kind = 'freshness_unknown'
                   AND scope = 'root'
                   AND relative_path = ''
                   AND previous_relative_path IS NULL
                   AND EXISTS(
                     SELECT 1 FROM library_change_queue_lanes AS capacity_lane
                     WHERE capacity_lane.change_id = library_change_queue.id
                       AND capacity_lane.lane = 'p0_live'
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_live_gap_recovery_claims AS capacity_claim
                     WHERE capacity_claim.gap_change_id = library_change_queue.id
                   )
                 ) THEN 1 ELSE 0 END), 0),
               COALESCE(SUM(CASE WHEN intent_kind = 'freshness_unknown'
                  AND status IN ('pending', 'leased', 'retry_wait') THEN 1 ELSE 0 END), 0),
               ({explicit_count}),
               MIN(CASE
                 WHEN status = 'pending' AND ready_unix_ms <= ?1 AND attempt_count < ?2
                   THEN ready_unix_ms
                 WHEN status = 'retry_wait' AND next_retry_unix_ms <= ?1
                   AND attempt_count < ?2
                   THEN next_retry_unix_ms
                 WHEN status = 'leased' AND lease_expires_unix_ms <= ?1
                   THEN lease_expires_unix_ms
                 ELSE NULL END),
               (SELECT exhausted_change.last_failure_code
                FROM library_change_queue AS exhausted_change
                WHERE {exhausted_predicate}
                  AND exhausted_change.status = 'retry_wait'
                  AND exhausted_change.attempt_count >= ?2
                  AND NOT (
                    exhausted_change.last_failure_code = ?5
                    AND exhausted_change.origin = 'live_notification'
                    AND exhausted_change.intent_kind = 'freshness_unknown'
                    AND exhausted_change.scope = 'root'
                    AND exhausted_change.relative_path = ''
                    AND exhausted_change.previous_relative_path IS NULL
                    AND EXISTS(
                      SELECT 1 FROM library_change_queue_lanes AS capacity_lane
                      WHERE capacity_lane.change_id = exhausted_change.id
                        AND capacity_lane.lane = 'p0_live'
                    )
                    AND NOT EXISTS(
                      SELECT 1 FROM library_live_gap_recovery_claims AS capacity_claim
                      WHERE capacity_claim.gap_change_id = exhausted_change.id
                    )
                  )
                  AND exhausted_change.last_failure_code IS NOT NULL
                ORDER BY {exhausted_order}
                LIMIT 1)
             FROM library_change_queue
             WHERE {root_predicate}"
            ),
            params![
                now_unix_ms,
                i64::from(policy.max_attempts),
                root_id,
                root_generation
                    .map(|generation| sqlite_integer(generation.value(), "root generation"))
                    .transpose()?,
                LibraryChangeCapacityDeferral::MetadataInventoryLane.failure_code(),
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            },
        )
        .map_err(database_error)?;
    let pending_count = sqlite_unsigned(pending, "pending change count")?;
    let leased_count = sqlite_unsigned(leased, "leased change count")?;
    let retry_wait_count = sqlite_unsigned(retry_wait, "retry-wait change count")?;
    let ready_count = sqlite_unsigned(ready, "ready change count")?;
    let expired_lease_count = sqlite_unsigned(expired, "expired lease count")?;
    let exhausted_retry_count = sqlite_unsigned(exhausted, "exhausted retry count")?;
    let explicit_recovery_required_count =
        sqlite_unsigned(explicit_recovery_required, "explicit recovery claim count")?;
    let unresolved_count = pending_count
        .saturating_add(leased_count)
        .saturating_add(retry_wait_count);
    let oldest_ready_delay_millis = oldest_due
        .and_then(|due| now_unix_ms.checked_sub(due))
        .and_then(|delay| u64::try_from(delay).ok())
        .unwrap_or(0);
    let health = if unresolved_count == 0 {
        LibraryChangeQueueHealth::Idle
    } else if expired_lease_count > 0
        || exhausted_retry_count > 0
        || explicit_recovery_required_count > 0
    {
        LibraryChangeQueueHealth::Degraded
    } else if ready_count > 0 && oldest_ready_delay_millis > 0 {
        LibraryChangeQueueHealth::Delayed
    } else {
        LibraryChangeQueueHealth::Healthy
    };
    Ok(LibraryChangeQueueMetrics {
        health,
        pending_count,
        leased_count,
        retry_wait_count,
        completed_count: sqlite_unsigned(completed, "completed change count")?,
        superseded_count: sqlite_unsigned(superseded, "superseded change count")?,
        ready_count,
        expired_lease_count,
        exhausted_retry_count,
        latest_exhausted_failure_code,
        freshness_unknown_count: sqlite_unsigned(
            freshness_unknown,
            "freshness-unknown change count",
        )?,
        explicit_recovery_required_count,
        oldest_ready_delay_millis,
    })
}
