use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::schema_shape::table_columns_match;
use super::{
    SQLITE_APPLICATION_ID, database_error, inventory_spool_retirement, inventory_spool_rows,
};

pub(super) fn schema_matches(connection: &Connection) -> Result<bool, ScanError> {
    inventory_spool_retirement::schema_matches(connection)
}

pub(super) fn validate_version_with_depth(
    connection: &Connection,
    expected_schema_version: i64,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let application_id: i64 = connection
        .query_row("PRAGMA application_id", [], |row| row.get(0))
        .map_err(database_error)?;
    let user_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(database_error)?;
    let spool_columns = vec![
        ("run_id", "TEXT", true, 1),
        ("authority_change_id", "INTEGER", true, 0),
        ("root_id", "TEXT", true, 0),
        ("root_generation", "INTEGER", true, 0),
        ("root_identity_scheme", "TEXT", true, 0),
        ("root_identity_value", "TEXT", true, 0),
        ("scope_kind", "TEXT", true, 0),
        ("scope_relative_path", "TEXT", true, 0),
        ("state", "TEXT", true, 0),
        ("created_unix_ms", "INTEGER", true, 0),
        ("updated_unix_ms", "INTEGER", true, 0),
    ];
    let mut spool_entry_columns = vec![
        ("run_id", "TEXT", true, 1),
        ("directory_relative_path", "TEXT", false, 0),
        ("relative_path", "TEXT", true, 2),
        ("entry_kind", "TEXT", true, 0),
        ("file_size", "INTEGER", false, 0),
        ("modified_unix_ms", "INTEGER", true, 0),
        ("file_identity_scheme", "TEXT", false, 0),
        ("file_identity_value", "TEXT", false, 0),
        ("placeholder_state", "TEXT", true, 0),
        ("is_reparse_point", "INTEGER", true, 0),
        ("staged_unix_ms", "INTEGER", true, 0),
    ];
    spool_entry_columns.push(("source_revision_token", "TEXT", false, 0));
    let columns_match = table_columns_match(
        connection,
        "library_metadata_inventory_spool_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_spools",
        &spool_columns,
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_spool_directories",
        &[
            ("run_id", "TEXT", true, 1),
            ("ordinal", "INTEGER", true, 0),
            ("relative_directory", "TEXT", true, 2),
            ("state", "TEXT", true, 0),
            ("directory_identity_scheme", "TEXT", false, 0),
            ("directory_identity_value", "TEXT", false, 0),
            ("source_entry_count", "INTEGER", true, 0),
            ("created_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_spool_entries",
        &spool_entry_columns,
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND contract_version = ?1 AND complete = 1
             FROM library_metadata_inventory_spool_contract",
            [4],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches = schema_matches(connection)?;
    if application_id != SQLITE_APPLICATION_ID
        || user_version != expected_schema_version
        || !columns_match
        || !marker_complete
        || !schema_matches
    {
        return Err(ScanError::new(
            "catalog_metadata_inventory_spool_contract_unverifiable",
            "The catalog cannot prove its bounded metadata inventory source spool",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    inventory_spool_rows::validate(connection, 4)
}
