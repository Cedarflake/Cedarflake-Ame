use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::schema_shape::{schema_object_sql_matches, table_columns_match};
use super::{
    ROOT_PUBLICATION_NAMESPACE_CONTRACT_TABLE_DDL, ROOT_PUBLICATION_NAMESPACE_TABLE_DDL,
    SCAN_PUBLICATION_NAMESPACE_BINDING_TABLE_DDL, SCAN_PUBLICATION_NAMESPACE_ROOT_INDEX_DDL,
    database_error,
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
        "library_root_publication_namespace_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_root_publication_namespaces",
        &[
            ("root_id", "TEXT", false, 1),
            ("root_generation", "INTEGER", true, 0),
            ("identity_scheme", "TEXT", true, 0),
            ("identity_value", "TEXT", true, 0),
            ("authority_kind", "TEXT", true, 0),
            ("established_catalog_revision", "INTEGER", true, 0),
            ("established_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_scan_publication_namespace_bindings",
        &[
            ("scan_id", "TEXT", false, 1),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("identity_scheme", "TEXT", true, 0),
            ("identity_value", "TEXT", true, 0),
            ("bound_unix_ms", "INTEGER", true, 0),
        ],
    )?;
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_root_publication_namespace_contract",
        ROOT_PUBLICATION_NAMESPACE_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_root_publication_namespaces",
        ROOT_PUBLICATION_NAMESPACE_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_scan_publication_namespace_bindings",
        SCAN_PUBLICATION_NAMESPACE_BINDING_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_scan_publication_namespace_root",
        SCAN_PUBLICATION_NAMESPACE_ROOT_INDEX_DDL,
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT contract_version = 1 AND complete = 1
             FROM library_root_publication_namespace_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !columns_match || !schema_matches || !marker_complete {
        return Err(unverifiable_contract());
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_root_publication_namespaces AS proof
               LEFT JOIN library_change_root_state AS active ON active.root_id = proof.root_id
               WHERE active.root_id IS NULL OR active.is_active <> 1
                  OR active.generation <> proof.root_generation
             ) OR EXISTS(
               SELECT 1
               FROM library_scan_publication_namespace_bindings AS binding
               LEFT JOIN scan_runs AS scan ON scan.id = binding.scan_id
               LEFT JOIN library_change_root_state AS active
                 ON active.root_id = binding.root_id
               WHERE scan.id IS NULL OR scan.root_id <> binding.root_id
                  OR scan.root_generation_at_start <> binding.root_generation
                  OR scan.status NOT IN ('running', 'paused')
                  OR active.is_active <> 1 OR active.generation <> binding.root_generation
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_root_publication_namespaces'
               )
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_scan_publication_namespace_bindings'
               )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let mut identities = connection
        .prepare(
            "SELECT identity_scheme, identity_value
             FROM library_root_publication_namespaces
             UNION ALL
             SELECT identity_scheme, identity_value
             FROM library_scan_publication_namespace_bindings",
        )
        .map_err(database_error)?;
    let identities_valid = identities
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?
        .all(|row| {
            row.is_ok_and(|(scheme, value)| validate_windows_root_identity(&scheme, &value).is_ok())
        });
    if invalid_relations || !identities_valid {
        return Err(unverifiable_contract());
    }
    Ok(())
}

pub(super) fn validate_windows_root_identity(scheme: &str, value: &str) -> Result<(), ScanError> {
    let valid = scheme == "windows-file-id-128-v1"
        && value.len() == 49
        && value.as_bytes().get(16) == Some(&b':')
        && value.bytes().enumerate().all(|(index, byte)| {
            index == 16 || byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        });
    if valid {
        Ok(())
    } else {
        Err(unverifiable_contract())
    }
}

pub(super) fn unverifiable_contract() -> ScanError {
    ScanError::new(
        "catalog_root_publication_namespace_unverifiable",
        "The catalog cannot prove the configured-root publication namespace contract",
    )
}
