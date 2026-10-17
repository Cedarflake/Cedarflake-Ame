use rusqlite::{Connection, OptionalExtension};

use crate::domain::{JournalIdentifier, JournalUsn, ScanError};

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::schema_shape::{
    cascade_foreign_key_matches, schema_object_sql_matches, table_columns_match,
};
use super::{
    RECOVERY_AUTHORITY_CONTRACT_TABLE_DDL, RECOVERY_AUTHORITY_INSERT_GUARD_DDL,
    RECOVERY_AUTHORITY_ROOT_INDEX_DDL, RECOVERY_AUTHORITY_TABLE_DDL,
    RECOVERY_AUTHORITY_UPDATE_GUARD_DDL, database_error,
};

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    validate_with_depth(connection, ContractValidationDepth::Full)
}

pub(super) fn validate_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_recovery_authority_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_recovery_authorities",
        &[
            ("change_id", "INTEGER", false, 1),
            ("run_id", "TEXT", true, 0),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("reason", "TEXT", true, 0),
            ("opening_journal_id", "TEXT", false, 0),
            ("opening_next_usn", "TEXT", false, 0),
            ("authorized_unix_ms", "INTEGER", true, 0),
            ("retired_unix_ms", "INTEGER", false, 0),
        ],
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND complete = 1
             FROM library_recovery_authority_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_recovery_authority_contract",
        RECOVERY_AUTHORITY_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_recovery_authorities",
        RECOVERY_AUTHORITY_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_recovery_authorities_root",
        RECOVERY_AUTHORITY_ROOT_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_recovery_authority_insert_guard",
        RECOVERY_AUTHORITY_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_recovery_authority_update_guard",
        RECOVERY_AUTHORITY_UPDATE_GUARD_DDL,
    )?;
    let foreign_key_matches = cascade_foreign_key_matches(
        connection,
        "library_recovery_authorities",
        "change_id",
        "library_change_queue",
        "id",
    )?;
    if !columns_match || !marker_complete || !schema_matches || !foreign_key_matches {
        return Err(unverifiable_contract());
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_rows = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_recovery_authorities AS authority
               LEFT JOIN library_change_queue AS queue ON queue.id = authority.change_id
               LEFT JOIN library_change_queue_lanes AS lanes
                 ON lanes.change_id = authority.change_id
               WHERE queue.id IS NULL
                  OR queue.root_id <> authority.root_id
                  OR queue.root_generation <> authority.root_generation
                  OR lanes.lane <> 'p2_recovery'
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_recovery_authorities')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let mut boundaries = connection
        .prepare(
            "SELECT opening_journal_id, opening_next_usn
             FROM library_recovery_authorities
             WHERE opening_journal_id IS NOT NULL OR opening_next_usn IS NOT NULL",
        )
        .map_err(database_error)?;
    let canonical_boundaries = boundaries
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?
        .all(|row| {
            row.is_ok_and(|(journal_id, next_usn)| {
                JournalIdentifier::parse_canonical(&journal_id).is_ok()
                    && JournalUsn::parse_canonical(&next_usn).is_ok()
            })
        });
    if invalid_rows || !canonical_boundaries {
        return Err(unverifiable_contract());
    }
    Ok(())
}

fn unverifiable_contract() -> ScanError {
    ScanError::new(
        "catalog_recovery_authority_contract_unverifiable",
        "The catalog cannot prove its bounded recovery authority",
    )
}

#[cfg(test)]
mod tests;
