use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{FileIdentityEvidence, MetadataInventoryRun, ScanError};

use super::{database_error, validate_active_root};

pub(super) fn publish(
    transaction: &Transaction<'_>,
    run: &MetadataInventoryRun,
) -> Result<(), ScanError> {
    validate_active_root(transaction, &run.request)?;
    let scope_identity = transaction
        .query_row(
            "SELECT directory_identity_scheme, directory_identity_value
             FROM library_metadata_inventory_frontier
             WHERE run_id = ?1 AND relative_directory = ?2 AND state = 'completed'
               AND directory_identity_scheme IS NOT NULL
               AND directory_identity_value IS NOT NULL",
            params![run.request.run_id, run.request.scope.relative_path()],
            |row| {
                Ok(FileIdentityEvidence {
                    scheme: row.get(0)?,
                    value: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(scope_identity) = scope_identity else {
        return Ok(());
    };
    if scope_identity.scheme != "windows-file-id-128-v1" {
        return Ok(());
    }
    super::super::validate_root_publication_identity(&scope_identity)?;
    let scan_id = transaction
        .query_row(
            "SELECT active_scan_id FROM library_roots WHERE id = ?1",
            [&run.request.root_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(database_error)?;
    let relative_path = run.request.scope.relative_path();

    // These are observations from the completed inventory, not current-path or USN coverage
    // authority. Their publication shares the inventory's commit and never rereads the source.
    if relative_path.is_empty() {
        transaction
            .execute(
                "DELETE FROM scan_directory_identities WHERE scan_id = ?1",
                [&scan_id],
            )
            .map_err(database_error)?;
    } else {
        transaction
            .execute(
                "DELETE FROM scan_directory_identities
                 WHERE scan_id = ?1 AND (relative_path = ?2
                   OR (relative_path >= ?3 AND relative_path < ?4))",
                params![
                    scan_id,
                    relative_path,
                    format!("{relative_path}/"),
                    format!("{relative_path}0"),
                ],
            )
            .map_err(database_error)?;
    }
    transaction
        .execute(
            "INSERT INTO scan_directory_identities(
               scan_id, relative_path, identity_scheme, identity_value
             )
             SELECT ?1, relative_path, file_identity_scheme, file_identity_value
             FROM library_metadata_inventory_entries
             WHERE run_id = ?2 AND entry_kind = 'directory'
             ON CONFLICT(scan_id, relative_path) DO UPDATE SET
               identity_scheme = excluded.identity_scheme,
               identity_value = excluded.identity_value",
            params![scan_id, run.request.run_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO scan_directory_identities(
               scan_id, relative_path, identity_scheme, identity_value
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(scan_id, relative_path) DO UPDATE SET
               identity_scheme = excluded.identity_scheme,
               identity_value = excluded.identity_value",
            params![
                scan_id,
                relative_path,
                scope_identity.scheme,
                scope_identity.value,
            ],
        )
        .map_err(database_error)?;
    // Durable traversal revalidates each directory after enumeration. Prefer that pinned
    // identity over the earlier parent-entry observation when the source uses a spool.
    transaction
        .execute(
            "INSERT INTO scan_directory_identities(
               scan_id, relative_path, identity_scheme, identity_value
             )
             SELECT ?1, relative_directory, directory_identity_scheme, directory_identity_value
             FROM library_metadata_inventory_spool_directories
             WHERE run_id = ?2 AND state = 'completed'
             ON CONFLICT(scan_id, relative_path) DO UPDATE SET
               identity_scheme = excluded.identity_scheme,
               identity_value = excluded.identity_value",
            params![scan_id, run.request.run_id],
        )
        .map_err(database_error)?;
    Ok(())
}
