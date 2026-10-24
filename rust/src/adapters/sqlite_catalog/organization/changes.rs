use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationAction, OrganizationPreparedSelection, OrganizationWriteReceipt, stale_selection,
};

use super::super::{database_error, unix_time_ms};
use super::operations::{self, Admission};
use super::{albums, authority, records, selection};

pub(super) fn save(
    connection: &Connection,
    operation_id: &str,
    prepared: &OrganizationPreparedSelection,
    action: &OrganizationAction,
) -> Result<OrganizationWriteReceipt, ScanError> {
    let digest = authority::change_request(prepared, action);
    let revision = match operations::admit(
        connection,
        operation_id,
        &digest,
        "organization",
        prepared.organization_revision,
    )? {
        Admission::Replayed(receipt) => return Ok(receipt),
        Admission::New { revision } => revision,
    };
    if &selection::prepare(connection, &prepared.selections)? != prepared {
        return Err(stale_selection());
    }
    let (disposition, album_id) = action.decision();
    if let Some(album_id) = album_id {
        albums::require_active(connection, album_id)?;
    }
    let mut changed_groups = 0;
    for group in &prepared.groups {
        records::materialize(connection, &group.key)?;
        let conflict: bool = connection
            .query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM organization_conflicts WHERE record_id IN ({}))",
                    records::CURRENT_RECORDS
                ),
                [],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if conflict
            && !matches!(
                action,
                OrganizationAction::AssignAlbum { .. } | OrganizationAction::Defer
            )
        {
            return Err(ScanError::new(
                "organization_conflict_unresolved",
                "Choose an album to resolve this group's conflicting assignments",
            ));
        }
        let resolving = conflict && matches!(action, OrganizationAction::AssignAlbum { .. });
        let changed = connection.execute(&format!(
            "INSERT INTO organization_operation_changes(operation_id, record_id, field, previous_album_id, previous_disposition, applied_revision)
             SELECT ?1, record_id, 'organization', album_id, disposition, ?2 FROM organization_records
             WHERE record_id IN ({}) AND (album_id IS NOT ?3 OR disposition <> ?4 OR ?5)", records::CURRENT_RECORDS
        ), params![operation_id, revision, album_id, disposition.stored(), resolving]).map_err(database_error)?;
        if changed == 0 {
            continue;
        }
        connection.execute(&format!(
            "INSERT INTO organization_operation_conflicts
             SELECT ?1, record_id, album_id, decision_revision FROM organization_conflicts WHERE record_id IN ({})", records::CURRENT_RECORDS
        ), [operation_id]).map_err(database_error)?;
        if resolving {
            connection
                .execute(
                    &format!(
                        "DELETE FROM organization_conflicts WHERE record_id IN ({})",
                        records::CURRENT_RECORDS
                    ),
                    [],
                )
                .map_err(database_error)?;
        }
        connection.execute(&format!(
            "UPDATE organization_records SET album_id = ?1, disposition = ?2, organization_revision = ?3, updated_unix_ms = ?4
             WHERE record_id IN ({})", records::CURRENT_RECORDS
        ), params![album_id, disposition.stored(), revision, unix_time_ms()]).map_err(database_error)?;
        changed_groups += 1;
    }
    operations::finish(connection, operation_id, revision, changed_groups, false)
}
