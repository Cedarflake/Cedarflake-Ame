use rusqlite::Connection;

use crate::domain::ScanError;

use super::{
    SCHEMA_VERSION, database_error, source_revision_metadata_contract_is_complete,
    validate_live_gap_recovery_contract, validate_live_gap_recovery_contract_with_depth,
    validate_pre_live_gap_schema_contract, validate_pre_live_gap_schema_contract_with_depth,
    validate_source_revision_rows, validate_source_revision_structure_contract,
};

#[derive(Clone, Copy)]
pub(super) enum ContractValidationDepth {
    StructureOnly,
    Full,
}

impl ContractValidationDepth {
    pub(super) fn includes_rows(self) -> bool {
        matches!(self, Self::Full)
    }

    pub(super) fn includes_active_rows(self) -> bool {
        self.includes_rows()
    }
}

pub(super) fn validate_current_schema_contract(connection: &Connection) -> Result<(), ScanError> {
    #[cfg(test)]
    super::CURRENT_SCHEMA_VALIDATION_COUNT.with(|count| count.set(count.get() + 1));
    validate_current_schema_contract_with_source_revision_rows(connection)
}

pub(super) fn validate_current_schema_contract_with_source_revision_rows(
    connection: &Connection,
) -> Result<(), ScanError> {
    validate_schema_version(connection, SCHEMA_VERSION)
}

pub(super) fn validate_schema_version(
    connection: &Connection,
    schema_version: i64,
) -> Result<(), ScanError> {
    if connection.is_autocommit() {
        let transaction = connection.unchecked_transaction().map_err(database_error)?;
        validate_in_snapshot(&transaction, schema_version)?;
        transaction.commit().map_err(database_error)
    } else {
        validate_in_snapshot(connection, schema_version)
    }
}

fn validate_in_snapshot(connection: &Connection, schema_version: i64) -> Result<(), ScanError> {
    validate_schema_structure(connection, schema_version)?;

    // The process session owns reuse. Only its initial validation (or explicit stale renewal)
    // reaches this full proof; ordinary connections retain bounded identity/schema checks.
    validate_pre_live_gap_schema_contract(connection, schema_version)?;
    validate_live_gap_recovery_contract(connection)?;
    validate_source_revision_rows(connection)
}

pub(super) fn validate_current_schema_structure(connection: &Connection) -> Result<(), ScanError> {
    validate_schema_structure(connection, SCHEMA_VERSION)
}

pub(super) fn validate_schema_structure(
    connection: &Connection,
    schema_version: i64,
) -> Result<(), ScanError> {
    // Reject missing or altered schema before queries interpret any authority rows. Successful
    // DDL markers cannot prove canonical journal payloads, cross-root ownership or generations.
    validate_pre_live_gap_schema_contract_with_depth(
        connection,
        schema_version,
        ContractValidationDepth::StructureOnly,
    )?;
    validate_live_gap_recovery_contract_with_depth(
        connection,
        ContractValidationDepth::StructureOnly,
    )?;
    validate_source_revision_structure_contract(connection)?;
    validate_existing_source_metadata_marker(connection)
}

fn validate_existing_source_metadata_marker(connection: &Connection) -> Result<(), ScanError> {
    let marker_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
             WHERE name = 'library_source_revision_metadata_contract')",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    // An absent marker belongs to the exact prerelease repair; a present damaged marker does not.
    if marker_exists && !source_revision_metadata_contract_is_complete(connection)? {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove that legacy source metadata was invalidated",
        ));
    }
    Ok(())
}
