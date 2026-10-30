use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::{ClassificationAction, ClassificationRecordId};
use crate::domain::classification_catalog::PreparedClassificationSelection;
use crate::domain::classification_review::{
    ClassificationReviewReceipt, ClassificationReviewSession,
};

use super::super::super::{database_error, sqlite_integer};
use super::super::{codec, observations, writes};
use super::{position, stale};

pub(super) fn apply(
    connection: &Connection,
    operation_id: &ClassificationRecordId,
    requested: &ClassificationReviewSession,
    prepared: &PreparedClassificationSelection,
    action: &ClassificationAction,
) -> Result<ClassificationReviewReceipt, ScanError> {
    let sequence = sqlite_integer(
        requested.position_sequence,
        "classification review position",
    )?;
    let previous: Option<(String, i64, i64)> = connection.query_row(
        "SELECT session_id, position_sequence, committed_sequence FROM classification_review_operations WHERE operation_id = ?1",
        [operation_id.as_str()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(database_error)?;
    if let Some((id, before, committed)) = previous {
        if id != requested.id.as_str() || before != sequence {
            return Err(stale());
        }
        return Ok(ClassificationReviewReceipt {
            operation: writes::apply(connection, operation_id, prepared, action)?,
            session: ClassificationReviewSession {
                id: codec::identifier(id)?,
                position_sequence: committed as u64,
            },
        });
    }
    let current = position::session(connection)?;
    if current.id != requested.id.as_str()
        || current.sequence != sequence
        || sequence == 0
        || prepared.groups.len() != 1
    {
        return Err(stale());
    }
    let bookmark = position::load(connection)?.ok_or_else(stale)?;
    if bookmark.after_current {
        return Err(stale());
    }
    let selected = prepared.selections.first().ok_or_else(stale)?;
    let matches: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_bindings AS binding
           JOIN organization_record_groups AS member ON member.record_id = binding.record_id
           JOIN organization_record_groups AS anchor ON anchor.group_id = member.group_id
           WHERE binding.asset_id = ?1 AND binding.source_generation = ?2 AND anchor.record_id = ?3)
         AND NOT EXISTS(SELECT 1 FROM classification_operations WHERE operation_id = ?4)",
            params![
                selected.asset_id,
                selected.source_generation as i64,
                bookmark.record_id,
                operation_id.as_str()
            ],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !matches {
        return Err(stale());
    }
    let committed = sequence.checked_add(1).ok_or_else(stale)?;
    let operation = writes::apply(connection, operation_id, prepared, action)?;
    let after = observations::prepare(connection, &prepared.selections)?;
    let remaining = after.groups.first().ok_or_else(stale)?;
    let advance = matches!(action, ClassificationAction::Defer)
        || !current.scope.includes(&remaining.effective());
    connection
        .execute(
            "UPDATE classification_review_bookmark SET after_current = ?1,
           revision = (SELECT revision FROM classification_state) WHERE singleton = 1",
            [advance],
        )
        .map_err(database_error)?;
    connection.execute(
        "UPDATE classification_review_session SET position_sequence = ?1, checkpoint_digest = NULL WHERE singleton = 1", [committed],
    ).map_err(database_error)?;
    connection
        .execute(
            "INSERT INTO classification_review_operations VALUES (?1, ?2, ?3, ?4)",
            params![operation_id.as_str(), current.id, sequence, committed],
        )
        .map_err(database_error)?;
    Ok(ClassificationReviewReceipt {
        operation,
        session: ClassificationReviewSession {
            id: requested.id.clone(),
            position_sequence: committed as u64,
        },
    })
}
