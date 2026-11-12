use rusqlite::{Connection, params};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::file_operation_exports::{
    FileOrganizationDestination as Destination, FileOrganizationRule as Rule,
};
use crate::domain::file_operation_journal::FILE_OPERATION_PAGE_SIZE;
use crate::domain::organization::OrganizationSelection;

use super::super::super::{database_error, sqlite_unsigned};

mod destination;

pub(super) fn freeze(
    connection: &Connection,
    rule: Rule,
    cancelled: &AtomicBool,
) -> Result<(), ScanError> {
    let transaction = connection.unchecked_transaction().map_err(database_error)?;
    let mut after = 0_i64;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(ScanError::new(
                "file_operation_selection_closed",
                "The file organization selection was cancelled",
            ));
        }
        let mut statement = transaction
            .prepare(
                "SELECT ordinal, asset_id, location_id, source_generation
            FROM temp.file_operation_selection WHERE ordinal > ?1 ORDER BY ordinal LIMIT ?2",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![after, FILE_OPERATION_PAGE_SIZE as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        drop(statement);
        if rows.is_empty() {
            break;
        }
        for (ordinal, asset_id, location_id, generation) in rows {
            let selected = OrganizationSelection {
                asset_id,
                location_id,
                source_generation: sqlite_unsigned(generation, "selected generation")?,
            };
            let destination = destination::resolve(&transaction, &selected, rule)?;
            let (folder, year, reason) = match destination {
                Destination::Direct => (None, None, None),
                Destination::Folder(name) => (Some(name), None, None),
                Destination::AlbumYear { album, year } => (Some(album), Some(year), None),
                Destination::Skip(reason) => (None, None, Some(reason)),
            };
            transaction
                .execute(
                    "UPDATE temp.file_operation_selection
                 SET destination_folder = ?2, destination_year = ?3, skip_reason = ?4 WHERE ordinal = ?1",
                    params![ordinal, folder, year, reason],
                )
                .map_err(database_error)?;
            after = ordinal;
        }
    }
    transaction.commit().map_err(database_error)
}
