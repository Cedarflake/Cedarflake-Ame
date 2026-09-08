use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior};

use crate::ports::LibraryChangeIngress;

use super::super::{SQLITE_BUSY_TIMEOUT, SqliteDatabaseIdentity, write_admission::ReservedWrite};
use super::*;

#[cfg(test)]
mod tests;

pub(crate) struct IngressReservation {
    catalog_identity: SqliteDatabaseIdentity,
    root_id: String,
    root_generation: LibraryRootGeneration,
    lane: LibraryChangeLane,
    admission: ReservedWrite,
}

impl LibraryChangeIngress for SqliteCatalog {
    type Reservation = IngressReservation;

    fn reserve_change_ingress(
        &self,
        intents: &[LibraryChangeIntent],
    ) -> Result<Self::Reservation, ScanError> {
        let (root_id, root_generation, lane) = binding(intents)?;
        Ok(IngressReservation {
            catalog_identity: self.session.database_identity.clone(),
            root_id: root_id.to_owned(),
            root_generation,
            lane,
            admission: self.write_admission.reserve(lane),
        })
    }

    fn try_enqueue_reserved_changes(
        &mut self,
        reservation: &mut Self::Reservation,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        validate_policy(policy)?;
        let (root_id, generation, lane) = binding(intents)?;
        if reservation.catalog_identity != self.session.database_identity
            || reservation.root_id != root_id
            || reservation.root_generation != generation
            || reservation.lane != lane
        {
            return Err(ScanError::new(
                "change_ingress_reservation_mismatch",
                "Change ingress admission does not belong to this catalog, root generation, and lane",
            ));
        }
        let _permit = reservation.admission.try_acquire().ok_or_else(|| {
            ScanError::new(
                "catalog_database_busy",
                "Change ingress is waiting for its reserved writer position",
            )
        })?;
        let restore = RestoreBusyTimeout {
            connection: &mut self.connection,
        };
        restore
            .connection
            .busy_timeout(Duration::ZERO)
            .map_err(database_error)?;
        let transaction = restore
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        cleanup_for_enqueue(&transaction, enqueued_unix_ms, policy)?;
        let report =
            enqueue_intents_in_transaction(&transaction, intents, None, enqueued_unix_ms, policy)?;
        transaction.commit().map_err(database_error)?;
        Ok(report)
    }
}

fn binding(
    intents: &[LibraryChangeIntent],
) -> Result<(&str, LibraryRootGeneration, LibraryChangeLane), ScanError> {
    validate_enqueue_batch(intents)?;
    let first = intents.first().ok_or_else(|| {
        ScanError::new(
            "change_queue_batch_empty",
            "Change ingress requires a nonempty plan",
        )
    })?;
    let lane = intents
        .iter()
        .map(|intent| intent.origin.lane())
        .min_by_key(|lane| super::super::sqlite_write_priority(*lane))
        .expect("nonempty validated batch");
    Ok((&first.root_id, first.root_generation, lane))
}

struct RestoreBusyTimeout<'connection> {
    connection: &'connection mut Connection,
}

impl Drop for RestoreBusyTimeout<'_> {
    fn drop(&mut self) {
        let _ = self.connection.busy_timeout(SQLITE_BUSY_TIMEOUT);
    }
}
