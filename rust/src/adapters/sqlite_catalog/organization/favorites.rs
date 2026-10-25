use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationGroupKey, OrganizationPreparedSelection, OrganizationWriteReceipt, stale_selection,
};

use super::super::{database_error, unix_time_ms};
use super::operations::{self, Admission, GroupWrite};
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
    let mut reconciled = false;
    for group in &prepared.groups {
        let outcome = apply_group(
            connection,
            operation_id,
            next_revision,
            &group.key,
            favorite,
        )?;
        reconciled |= outcome.reconciled;
        changed_groups += u64::from(outcome.changed);
    }
    operations::finish(
        connection,
        operation_id,
        next_revision,
        changed_groups,
        reconciled,
    )
}

pub(super) fn apply_group(
    connection: &Connection,
    operation_id: &str,
    next_revision: i64,
    key: &OrganizationGroupKey,
    favorite: bool,
) -> Result<GroupWrite, ScanError> {
    let reconciled = records::materialize(connection, key)?;
    let changes_sql = format!(
            "INSERT INTO organization_operation_changes(operation_id, record_id, field, previous_favorite, applied_revision)
             SELECT ?1, member.record_id, 'favorite', canonical.favorite, ?2
             FROM organization_record_groups AS member
             JOIN organization_records AS canonical ON canonical.record_id = member.group_id
             WHERE member.record_id IN ({}) AND canonical.favorite <> ?3", records::CURRENT_RECORDS
    );
    let changed = connection
        .prepare_cached(&changes_sql)
        .map_err(database_error)?
        .execute(params![operation_id, next_revision, favorite])
        .map_err(database_error)?;
    if changed == 0 {
        return Ok(GroupWrite {
            changed: false,
            reconciled,
        });
    }
    let update_sql = format!(
            "UPDATE organization_records SET favorite = ?1, favorite_revision = ?2, updated_unix_ms = ?3
             WHERE record_id IN ({})", records::CURRENT_RECORDS
    );
    connection
        .prepare_cached(&update_sql)
        .map_err(database_error)?
        .execute(params![favorite, next_revision, unix_time_ms()])
        .map_err(database_error)?;
    Ok(GroupWrite {
        changed: true,
        reconciled,
    })
}
