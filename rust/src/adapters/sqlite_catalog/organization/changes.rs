use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationAction, OrganizationGroupKey, OrganizationPreparedSelection,
    OrganizationWriteReceipt, stale_selection,
};

use super::super::{database_error, unix_time_ms};
use super::operations::{self, Admission, GroupWrite};
use super::{albums, authority, inspection, records, selection};

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
    let (_, album_id) = action.decision();
    if let Some(album_id) = album_id {
        albums::require_active(connection, album_id)?;
    }
    let mut changed_groups = 0;
    let mut reconciled = false;
    for group in &prepared.groups {
        let outcome = apply_group(connection, operation_id, revision, &group.key, action)?;
        reconciled |= outcome.reconciled;
        changed_groups += u64::from(outcome.changed);
    }
    operations::finish(
        connection,
        operation_id,
        revision,
        changed_groups,
        reconciled,
    )
}

pub(super) fn apply_group(
    connection: &Connection,
    operation_id: &str,
    revision: i64,
    key: &OrganizationGroupKey,
    action: &OrganizationAction,
) -> Result<GroupWrite, ScanError> {
    if matches!(action, OrganizationAction::RemoveAlbum) {
        let current = inspection::decision(connection, key)?;
        if current.album.is_none() && !current.has_conflict {
            return Ok(GroupWrite {
                changed: false,
                reconciled: false,
            });
        }
    }
    let (disposition, album_id) = action.decision();
    let reconciled = records::materialize(connection, key)?;
    let conflict: bool = connection
        .prepare_cached(&format!(
            "SELECT EXISTS(SELECT 1 FROM organization_conflicts WHERE record_id IN ({}))",
            records::CURRENT_RECORDS
        ))
        .map_err(database_error)?
        .query_row([], |row| row.get(0))
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
    let changes_sql = format!(
            "INSERT INTO organization_operation_changes(operation_id, record_id, field, previous_album_id, previous_disposition, applied_revision)
             SELECT ?1, member.record_id, 'organization', canonical.album_id, canonical.disposition, ?2
             FROM organization_record_groups AS member
             JOIN organization_records AS canonical ON canonical.record_id = member.group_id
             WHERE member.record_id IN ({}) AND (canonical.album_id IS NOT ?3 OR canonical.disposition <> ?4 OR ?5)", records::CURRENT_RECORDS
    );
    let changed = connection
        .prepare_cached(&changes_sql)
        .map_err(database_error)?
        .execute(params![
            operation_id,
            revision,
            album_id,
            disposition.stored(),
            resolving
        ])
        .map_err(database_error)?;
    if changed == 0 {
        return Ok(GroupWrite {
            changed: false,
            reconciled,
        });
    }
    let conflicts_sql = format!(
            "INSERT INTO organization_operation_conflicts
             SELECT ?1, record_id, album_id, decision_revision FROM organization_conflicts WHERE record_id IN ({})", records::CURRENT_RECORDS
    );
    connection
        .prepare_cached(&conflicts_sql)
        .map_err(database_error)?
        .execute([operation_id])
        .map_err(database_error)?;
    if resolving {
        connection
            .prepare_cached(&format!(
                "DELETE FROM organization_conflicts WHERE record_id IN ({})",
                records::CURRENT_RECORDS
            ))
            .map_err(database_error)?
            .execute([])
            .map_err(database_error)?;
    }
    let update_sql = format!(
            "UPDATE organization_records SET album_id = ?1, disposition = ?2, organization_revision = ?3, updated_unix_ms = ?4
             WHERE record_id IN ({})", records::CURRENT_RECORDS
    );
    connection
        .prepare_cached(&update_sql)
        .map_err(database_error)?
        .execute(params![
            album_id,
            disposition.stored(),
            revision,
            unix_time_ms()
        ])
        .map_err(database_error)?;
    Ok(GroupWrite {
        changed: true,
        reconciled,
    })
}
