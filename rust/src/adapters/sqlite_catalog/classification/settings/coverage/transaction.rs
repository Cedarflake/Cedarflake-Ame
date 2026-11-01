use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rusqlite::Connection;

use crate::domain::ScanError;

use super::database_error;

struct Progress<'a>(&'a Connection);

impl Drop for Progress<'_> {
    fn drop(&mut self) {
        let _ = self.0.progress_handler(0, None::<fn() -> bool>);
    }
}

pub(super) fn read<T>(
    connection: &Connection,
    cancelled: &Arc<AtomicBool>,
    query: impl FnOnce(&Connection) -> Result<T, ScanError>,
) -> Result<T, ScanError> {
    check(cancelled)?;
    let transaction = connection.unchecked_transaction().map_err(database_error)?;
    let value = interruptible(&transaction, cancelled, query)?;
    transaction.commit().map_err(database_error)?;
    Ok(value)
}

pub(in crate::adapters::sqlite_catalog::classification::settings) fn interruptible<T>(
    connection: &Connection,
    cancelled: &Arc<AtomicBool>,
    query: impl FnOnce(&Connection) -> Result<T, ScanError>,
) -> Result<T, ScanError> {
    check(cancelled)?;
    let progress = Progress(connection);
    let signal = Arc::clone(cancelled);
    connection
        .progress_handler(1000, Some(move || signal.load(Ordering::Acquire)))
        .map_err(database_error)?;
    let result = query(connection);
    drop(progress);
    check(cancelled)?;
    result
}

fn check(cancelled: &AtomicBool) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "classification_cancelled",
            "Classification was cancelled",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
