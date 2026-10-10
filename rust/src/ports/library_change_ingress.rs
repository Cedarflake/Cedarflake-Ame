use crate::domain::{
    LibraryChangeEnqueueReport, LibraryChangeIntent, LibraryChangeQueuePolicy, ScanError,
};

use super::LibraryChangeQueue;

pub(crate) trait LibraryChangeIngress: LibraryChangeQueue {
    type Reservation: Send;

    /// Reserves ordering without holding a transaction or waiting for the active writer.
    /// Dropping the reservation retires only this request's place in the queue.
    fn reserve_change_ingress(
        &self,
        intents: &[LibraryChangeIntent],
    ) -> Result<Self::Reservation, ScanError>;

    /// A busy attempt retains its reservation and commits nothing. Other failures require
    /// retiring admission before retrying, so capacity consumers can make progress.
    fn try_enqueue_reserved_changes(
        &mut self,
        reservation: &mut Self::Reservation,
        intents: &[LibraryChangeIntent],
        enqueued_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError>;
}
