use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;
use crate::domain::organization::{OrganizationHistoryReceipt, OrganizationWriteReceipt};

use super::super::database_error;

pub(super) fn load(
    connection: &Connection,
    operation: OrganizationWriteReceipt,
) -> Result<OrganizationHistoryReceipt, ScanError> {
    let retired_album_id = connection
        .query_row(
            "SELECT change.album_id FROM organization_album_operation_changes AS change
         JOIN organization_album_retirements AS retirement ON retirement.album_id = change.album_id
         WHERE change.operation_id = ?1",
            [&operation.operation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    Ok(OrganizationHistoryReceipt {
        operation,
        retired_album_id,
    })
}
