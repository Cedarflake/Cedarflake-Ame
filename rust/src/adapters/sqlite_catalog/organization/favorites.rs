use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationPreparedSelection, OrganizationWriteReceipt, stale_selection,
};

use super::super::{database_error, unix_time_ms};
use super::operations::{self, Admission};
use super::{authority, records, selection};

pub(super) fn save(
    connection: &Connection,
    operation_id: &str,
    prepared: &OrganizationPreparedSelection,
    favorite: bool,
) -> Result<OrganizationWriteReceipt, ScanError> {
    let digest = authority::favorite_request(prepared, favorite);
    let next_revision = match operations::admit(
        connection,
        operation_id,
        &digest,
        "favorite",
        prepared.organization_revision,
    )? {
        Admission::Replayed(receipt) => return Ok(receipt),
        Admission::New { revision } => revision,
    };
    if &selection::prepare(connection, &prepared.selections)? != prepared {
        return Err(stale_selection());
    }
    let mut changed_groups = 0_u64;
    for group in &prepared.groups {
        records::materialize(connection, &group.key)?;
        let changed = connection.execute(&format!(
            "INSERT INTO organization_operation_changes(operation_id, record_id, field, previous_favorite, applied_revision)
             SELECT ?1, record_id, 'favorite', favorite, ?2 FROM organization_records
             WHERE record_id IN ({}) AND favorite <> ?3", records::CURRENT_RECORDS
        ), params![operation_id, next_revision, favorite]).map_err(database_error)?;
        if changed == 0 {
            continue;
        }
        connection.execute(&format!(
            "UPDATE organization_records SET favorite = ?1, favorite_revision = ?2, updated_unix_ms = ?3
             WHERE record_id IN ({}) AND favorite <> ?1", records::CURRENT_RECORDS
        ), params![favorite, next_revision, unix_time_ms()]).map_err(database_error)?;
        changed_groups += 1;
    }
    operations::finish(
        connection,
        operation_id,
        next_revision,
        changed_groups,
        false,
    )
}
