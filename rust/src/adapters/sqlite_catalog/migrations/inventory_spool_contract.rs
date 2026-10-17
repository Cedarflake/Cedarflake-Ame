use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::schema_shape::{schema_object_sql_matches, table_columns_match};
use super::{
    METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL,
    METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_V27_DDL,
    METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_DDL, METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V27_DDL,
    METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V31_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL, METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
    METADATA_INVENTORY_SPOOL_ENTRY_TABLE_DDL, METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL,
    METADATA_INVENTORY_SPOOL_TABLE_DDL, METADATA_INVENTORY_SPOOL_TABLE_V27_DDL,
    SQLITE_APPLICATION_ID, database_error, inventory_spool_retirement, inventory_spool_rows,
};

pub(super) fn schema_matches(
    connection: &Connection,
    expected_contract_version: i64,
) -> Result<bool, ScanError> {
    if expected_contract_version == 4 {
        return inventory_spool_retirement::schema_matches(connection);
    }
    let contract_ddl = match expected_contract_version {
        1 => METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V27_DDL,
        2 => METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_DDL,
        3 => METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V31_DDL,
        _ => return Ok(false),
    };
    let spool_ddl = if expected_contract_version == 1 {
        METADATA_INVENTORY_SPOOL_TABLE_V27_DDL
    } else {
        METADATA_INVENTORY_SPOOL_TABLE_DDL
    };
    let binding_guard_ddl = if expected_contract_version == 1 {
        METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_V27_DDL
    } else {
        METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL
    };
    Ok(schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_spool_contract",
        contract_ddl,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_spools",
        spool_ddl,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_spool_directories",
        METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_spool_directories_state",
        METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_spool_entries",
        if expected_contract_version == 3 {
            METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL
        } else {
            METADATA_INVENTORY_SPOOL_ENTRY_TABLE_DDL
        },
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_spool_entries_order",
        METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_metadata_inventory_spool_binding_update_guard",
        binding_guard_ddl,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_metadata_inventory_spool_directory_complete_guard",
        METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    )?)
}

pub(super) fn validate_version(
    connection: &Connection,
    expected_schema_version: i64,
    expected_contract_version: i64,
) -> Result<(), ScanError> {
    validate_version_with_depth(
        connection,
        expected_schema_version,
        expected_contract_version,
        ContractValidationDepth::Full,
    )
}

pub(super) fn validate_version_with_depth(
    connection: &Connection,
    expected_schema_version: i64,
    expected_contract_version: i64,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let application_id: i64 = connection
        .query_row("PRAGMA application_id", [], |row| row.get(0))
        .map_err(database_error)?;
    let user_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(database_error)?;
    let spool_columns = if expected_contract_version == 1 {
        vec![
            ("run_id", "TEXT", false, 1),
            ("authority_change_id", "INTEGER", true, 0),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("scope_kind", "TEXT", true, 0),
            ("scope_relative_path", "TEXT", true, 0),
            ("state", "TEXT", true, 0),
            ("created_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ]
    } else {
        vec![
            ("run_id", "TEXT", expected_contract_version >= 4, 1),
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
        ]
    };
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
    if expected_contract_version >= 3 {
        spool_entry_columns.push(("source_revision_token", "TEXT", false, 0));
    }
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
            [expected_contract_version],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches = schema_matches(connection, expected_contract_version)?;
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
    inventory_spool_rows::validate(connection, expected_contract_version)
}
