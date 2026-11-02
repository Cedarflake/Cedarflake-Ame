use std::path::Path;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::domain::ScanError;

use super::{schema::open, storage_error};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DownloadIntent {
    generation: i64,
    package: String,
}

impl DownloadIntent {
    pub fn pending(path: &Path, package: &str) -> Result<Option<Self>, ScanError> {
        if !path.try_exists().map_err(storage_error)? {
            return Ok(None);
        }
        read_pending(&open(path)?, package)
    }

    pub fn authorize(path: &Path, package: &str) -> Result<Self, ScanError> {
        let mut connection = open(path)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let pending = read_pending(&transaction, package)?;
        if let Some(pending) = pending {
            return Ok(pending);
        }
        transaction.execute(
            "INSERT INTO download_intent(id, generation, package, pending) VALUES (1, 1, ?1, 1)
             ON CONFLICT(id) DO UPDATE SET generation = generation + 1, package = excluded.package, pending = 1",
            [package],
        ).map_err(storage_error)?;
        let pending = read_pending(&transaction, package)?.ok_or_else(stale)?;
        transaction.commit().map_err(storage_error)?;
        Ok(pending)
    }

    pub fn require_current(&self, path: &Path) -> Result<(), ScanError> {
        if Self::pending(path, &self.package)?.as_ref() == Some(self) {
            Ok(())
        } else {
            Err(stale())
        }
    }

    pub fn cancel(&self, path: &Path) -> Result<(), ScanError> {
        // An old operation may finish retiring after a newer explicit request was admitted.
        let connection = open(path)?;
        retire(&connection, self)?;
        Ok(())
    }
}

pub(super) fn complete(connection: &Connection, intent: &DownloadIntent) -> Result<(), ScanError> {
    if retire(connection, intent)? != 1 {
        return Err(stale());
    }
    Ok(())
}

fn retire(connection: &Connection, intent: &DownloadIntent) -> Result<usize, ScanError> {
    connection
        .execute(
            "UPDATE download_intent SET pending = 0
         WHERE id = 1 AND generation = ?1 AND package = ?2 AND pending = 1",
            params![intent.generation, intent.package],
        )
        .map_err(storage_error)
}

fn read_pending(
    connection: &Connection,
    package: &str,
) -> Result<Option<DownloadIntent>, ScanError> {
    connection.query_row(
        "SELECT generation, package FROM download_intent WHERE id = 1 AND pending = 1 AND package = ?1",
        [package],
        |row| Ok(DownloadIntent { generation: row.get(0)?, package: row.get(1)? }),
    ).optional().map_err(storage_error)
}

fn stale() -> ScanError {
    ScanError::new(
        "classification_installation_changed",
        "The model download request is no longer current",
    )
}

#[cfg(test)]
mod tests;
