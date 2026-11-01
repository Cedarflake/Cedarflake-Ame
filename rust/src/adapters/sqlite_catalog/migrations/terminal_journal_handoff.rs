use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::super::handoff_retirement;
use super::{SCHEMA_VERSION, current_schema, database_error, journal_scope_recovery, schema_shape};

const COMPLETED_RECOVERY_EVIDENCE: &str = "
    SELECT retained.catch_up_source, retained.catch_up_watermark
    FROM (
      SELECT catch_up_source, catch_up_watermark FROM library_change_catch_up_handoffs
      UNION
      SELECT catch_up_source, catch_up_watermark FROM library_change_scan_handoff_lineage
    ) AS retained
    WHERE EXISTS (
      SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
      JOIN library_change_queue AS parent ON parent.id = lineage.change_id
      JOIN library_journal_scope_recoveries AS recovery ON recovery.parent_change_id = parent.id
      JOIN library_change_queue AS current ON current.id = recovery.current_change_id
      LEFT JOIN library_change_queue AS previous ON previous.id = recovery.previous_change_id
      WHERE lineage.catch_up_source = retained.catch_up_source
        AND lineage.catch_up_watermark = retained.catch_up_watermark
        AND parent.status = 'completed' AND current.status = 'completed'
        AND parent.catalog_revision_at_success IS NOT NULL
        AND (parent.previous_relative_path IS NULL OR previous.status = 'completed')
    )";

fn releasable_evidence(
    connection: &Connection,
    limit: usize,
) -> Result<Vec<(String, String)>, ScanError> {
    let mut statement = connection
        .prepare(COMPLETED_RECOVERY_EVIDENCE)
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?;
    let mut evidence = Vec::new();
    for row in rows {
        let (source, watermark) = row.map_err(database_error)?;
        if !handoff_retirement::has_active_owner(connection, &source, &watermark)? {
            evidence.push((source, watermark));
            if evidence.len() == limit {
                break;
            }
        }
    }
    Ok(evidence)
}

pub(super) fn needed(connection: &Connection, version: i64) -> Result<bool, ScanError> {
    current_schema::validate_schema_structure(connection, version)?;
    Ok(!releasable_evidence(connection, 1)?.is_empty())
}

pub(super) fn apply(transaction: &Transaction<'_>, version: i64) -> Result<(), ScanError> {
    current_schema::validate_schema_structure(transaction, version)?;
    // The ordered recovery children prove completion; a terminal queue row alone does not.
    journal_scope_recovery::validate_rows(transaction)?;
    loop {
        let evidence = releasable_evidence(transaction, 128)?;
        if evidence.is_empty() {
            return Ok(());
        }
        handoff_retirement::cleanup_terminal_catch_up_handoffs_batch(transaction, &evidence)?;
    }
}

pub(super) fn repair_before_upgrade(connection: &mut Connection) -> Result<(), ScanError> {
    let version = schema_shape::schema_version(connection)?;
    if !(37..SCHEMA_VERSION).contains(&version) || !needed(connection, version)? {
        return Ok(());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    apply(&transaction, version)?;
    current_schema::validate_schema_version(&transaction, version)?;
    transaction.commit().map_err(database_error)
}

#[cfg(test)]
mod tests;
