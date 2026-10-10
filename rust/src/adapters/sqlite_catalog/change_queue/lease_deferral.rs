use std::collections::BTreeSet;

use rusqlite::params;

use crate::domain::{
    LibraryChangeLeaseIdentity, LibraryChangeLeaseUpdateOutcome, LibraryChangeQueuePolicy,
    ScanError,
};

use super::{
    SqliteCatalog, admission_lane_for_change_ids, classify_lease_update, database_error,
    sqlite_integer,
};

pub(super) fn defer(
    catalog: &mut SqliteCatalog,
    leases: &[LibraryChangeLeaseIdentity],
    deferred_unix_ms: i64,
) -> Result<Vec<LibraryChangeLeaseUpdateOutcome>, ScanError> {
    if leases.is_empty() {
        return Ok(Vec::new());
    }
    let mut identities = BTreeSet::new();
    if leases.len() > LibraryChangeQueuePolicy::MAX_LEASE_BATCH as usize
        || leases
            .iter()
            .any(|lease| lease.lease_generation == 0 || !identities.insert(lease.change_id.value()))
    {
        return Err(ScanError::new(
            "change_queue_deferral_batch_invalid",
            "Lease deferral requires one bounded batch of unique nonzero leases",
        ));
    }
    let mut existing_ids = Vec::with_capacity(leases.len());
    for lease in leases {
        sqlite_integer(lease.lease_generation, "lease generation")?;
        let exists = catalog
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM library_change_queue WHERE id = ?1)",
                [sqlite_integer(lease.change_id.value(), "change ID")?],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if exists {
            existing_ids.push(lease.change_id);
        }
    }
    let lane = admission_lane_for_change_ids(&catalog.connection, existing_ids)?;
    let transaction = catalog.begin_write_in_lane(lane)?;
    let mut outcomes = Vec::with_capacity(leases.len());
    for lease in leases {
        let outcome =
            classify_lease_update(&transaction, lease.change_id, lease.lease_generation, None)?;
        if outcome == LibraryChangeLeaseUpdateOutcome::Applied {
            transaction
                .execute(
                    "UPDATE library_change_queue
                 SET status = 'pending', ready_unix_ms = ?1,
                     attempt_count = CASE
                       WHEN attempt_count > 0 THEN attempt_count - 1 ELSE 0 END,
                     next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                     updated_unix_ms = ?1
                 WHERE id = ?2 AND status = 'leased' AND lease_generation = ?3",
                    params![
                        deferred_unix_ms,
                        sqlite_integer(lease.change_id.value(), "change ID")?,
                        sqlite_integer(lease.lease_generation, "lease generation")?,
                    ],
                )
                .map_err(database_error)?;
        }
        outcomes.push(outcome);
    }
    transaction.commit().map_err(database_error)?;
    Ok(outcomes)
}

#[cfg(test)]
mod tests;
