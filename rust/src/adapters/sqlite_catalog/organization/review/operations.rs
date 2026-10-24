use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::organization::{OrganizationAction, OrganizationPreparedSelection};
use crate::domain::organization_review::{
    OrganizationReviewSession, OrganizationReviewWriteReceipt, stale_session,
};

use super::super::super::{database_error, unix_time_ms};
use super::super::changes;
use super::position;

pub(super) fn save(
    connection: &Connection,
    operation_id: &str,
    requested: &OrganizationReviewSession,
    prepared: &OrganizationPreparedSelection,
    action: &OrganizationAction,
) -> Result<OrganizationReviewWriteReceipt, ScanError> {
    let previous: Option<(String, i64, i64)> = connection
        .query_row(
            "SELECT session_id, position_sequence, committed_sequence
             FROM organization_review_operations WHERE operation_id = ?1",
            [operation_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(database_error)?;
    if let Some((id, sequence, committed)) = previous {
        if id != requested.session_id || sequence != requested.position_sequence as i64 {
            return Err(stale_session());
        }
        let operation = changes::save(connection, operation_id, prepared, action)?;
        return Ok(OrganizationReviewWriteReceipt {
            operation,
            session: OrganizationReviewSession {
                session_id: id,
                position_sequence: committed as u64,
            },
        });
    }
    let current = position::session(connection)?;
    if current.id != requested.session_id
        || current.sequence != requested.position_sequence as i64
        || current.sequence == 0
        || prepared.groups.len() != 1
    {
        return Err(stale_session());
    }
    let bookmark = position::bookmark(connection)?.ok_or_else(stale_session)?;
    if bookmark.after_current {
        return Err(stale_session());
    }
    let selected = &prepared.selections[0];
    let matches: bool = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM organization_bindings AS binding
               JOIN organization_record_groups AS member ON member.record_id = binding.record_id
               JOIN organization_record_groups AS anchor ON anchor.group_id = member.group_id
               WHERE binding.asset_id = ?1 AND binding.source_generation = ?2
                 AND anchor.record_id = ?3
             ) AND NOT EXISTS(SELECT 1 FROM organization_operations WHERE operation_id = ?4)",
            params![
                selected.asset_id,
                selected.source_generation as i64,
                bookmark.record_id,
                operation_id
            ],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !matches {
        return Err(stale_session());
    }
    let committed = current.sequence.checked_add(1).ok_or_else(stale_session)?;
    let operation = changes::save(connection, operation_id, prepared, action)?;
    connection
        .execute(
            "UPDATE organization_review_bookmark
             SET after_current = 1, updated_unix_ms = ?1 WHERE singleton = 1",
            [unix_time_ms()],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "UPDATE organization_review_session
             SET position_sequence = ?1, checkpoint_digest = NULL WHERE singleton = 1",
            [committed],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO organization_review_operations VALUES (?1, ?2, ?3, ?4)",
            params![operation_id, current.id, current.sequence, committed],
        )
        .map_err(database_error)?;
    Ok(OrganizationReviewWriteReceipt {
        operation,
        session: OrganizationReviewSession {
            session_id: current.id,
            position_sequence: committed as u64,
        },
    })
}
