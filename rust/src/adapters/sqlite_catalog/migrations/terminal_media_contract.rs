use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::database_error;
use super::schema_shape::{cascade_foreign_key_matches, schema_version, table_columns_match};

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    validate_with_depth(connection, ContractValidationDepth::Full)
}

pub(super) fn validate_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let mut evidence_columns = vec![
        ("root_id", "TEXT", true, 1),
        ("relative_path", "TEXT", true, 2),
        ("file_size", "INTEGER", true, 0),
        ("modified_unix_ms", "INTEGER", true, 0),
        ("file_identity_scheme", "TEXT", false, 0),
        ("file_identity_value", "TEXT", false, 0),
        ("inspection_engine_id", "TEXT", true, 0),
        ("inspection_engine_version", "INTEGER", true, 0),
        ("issue_code", "TEXT", true, 0),
        ("issue_message", "TEXT", true, 0),
        ("updated_unix_ms", "INTEGER", true, 0),
    ];
    if schema_version(connection)? >= 31 {
        evidence_columns.push(("source_revision_token", "TEXT", false, 0));
        evidence_columns.push(("source_generation", "INTEGER", false, 0));
    }
    let structure_matches = table_columns_match(
        connection,
        "library_terminal_media_evidence_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_terminal_media_evidence",
        &evidence_columns,
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND complete = 1
             FROM library_terminal_media_evidence_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let foreign_key_matches = cascade_foreign_key_matches(
        connection,
        "library_terminal_media_evidence",
        "root_id",
        "library_roots",
        "id",
    )?;
    if !structure_matches || !marker_complete || !foreign_key_matches {
        return Err(ScanError::new(
            "catalog_terminal_media_evidence_contract_unverifiable",
            "The catalog cannot prove its terminal media evidence authority",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_terminal_media_evidence')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_terminal_media_evidence_contract_unverifiable",
            "The catalog cannot prove its terminal media evidence authority",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
