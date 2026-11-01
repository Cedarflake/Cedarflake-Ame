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
    let progress = Progress(&transaction);
    let signal = Arc::clone(cancelled);
    transaction
        .progress_handler(1000, Some(move || signal.load(Ordering::Acquire)))
        .map_err(database_error)?;
    let result = query(&transaction);
    drop(progress);
    check(cancelled)?;
    let value = result?;
    transaction.commit().map_err(database_error)?;
    Ok(value)
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
