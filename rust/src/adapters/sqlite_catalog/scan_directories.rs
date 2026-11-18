use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{FileIdentityEvidence, ScanError};

use super::{SqliteCatalog, database_error, validate_root_publication_identity};

impl SqliteCatalog {
    pub(crate) fn complete_directory_with_identity(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
        remaining_entries: &[String],
        identity: Option<&FileIdentityEvidence>,
    ) -> Result<(), ScanError> {
        if let Some(identity) = identity {
            validate_root_publication_identity(identity)?;
        }
        let transaction = self.begin_write()?;
        let updated = transaction
            .execute(
                "UPDATE scan_runs SET current_directory_enumerated = 1
                 WHERE id = ?1 AND status = 'running'
                   AND current_directory_relative_path = ?2
                   AND current_directory_enumerated = 0
                   AND root_generation_at_start = (
                       SELECT generation FROM library_change_root_state
                       WHERE root_id = scan_runs.root_id AND is_active = 1
                   )",
                params![scan_id, relative_directory],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_directory_not_enumerable",
                "The current directory enumeration could not be completed",
            ));
        }
        insert_directory_entries(&transaction, scan_id, relative_directory, remaining_entries)?;
        if let Some(identity) = identity {
            transaction
                .execute(
                    "INSERT INTO scan_directory_identities(
                        scan_id, relative_path, identity_scheme, identity_value
                     ) VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(scan_id, relative_path) DO UPDATE SET
                        identity_scheme = excluded.identity_scheme,
                        identity_value = excluded.identity_value",
                    params![scan_id, relative_directory, identity.scheme, identity.value],
                )
                .map_err(database_error)?;
        }
        // The active scan pointer publishes these bindings in the same transaction as its
        // asset projection. Consumed traversal frontiers are not namespace history.
        transaction.commit().map_err(database_error)
    }
}

pub(super) fn insert_directory_entries(
    transaction: &Transaction<'_>,
    scan_id: &str,
    relative_directory: &str,
    entries: &[String],
) -> Result<(), ScanError> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut insert = transaction
        .prepare_cached(
            "INSERT OR IGNORE INTO scan_directory_entries(
                scan_id, directory_relative_path, relative_path
             ) VALUES (?1, ?2, ?3)",
        )
        .map_err(database_error)?;
    for entry in entries {
        insert
            .execute(params![scan_id, relative_directory, entry])
            .map_err(database_error)?;
    }
    Ok(())
}

pub(super) fn take_next_directory(
    transaction: &Transaction<'_>,
    scan_id: &str,
) -> Result<Option<String>, ScanError> {
    transaction
        .query_row(
            "DELETE FROM scan_directory_frontier
             WHERE id = (SELECT id FROM scan_directory_frontier
                         WHERE scan_id = ?1 ORDER BY id LIMIT 1)
             RETURNING relative_path",
            [scan_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)
}

#[cfg(test)]
mod tests;
