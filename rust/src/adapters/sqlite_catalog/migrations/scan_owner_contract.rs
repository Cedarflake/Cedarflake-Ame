use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::database_error;

pub(super) fn validate_recovery_marker(connection: &Connection) -> Result<(), ScanError> {
    validate_recovery_base_marker(connection)?;
    let has_scan_ownership_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('library_change_queue_contract')
               WHERE name = 'scan_ownership_complete'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_scan_ownership_contract {
        return Err(unverifiable_recovery_contract());
    }
    let ownership_complete = connection
        .query_row(
            "SELECT scan_ownership_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !ownership_complete {
        return Err(unverifiable_recovery_contract());
    }
    Ok(())
}

pub(super) fn validate_recovery_base_marker(connection: &Connection) -> Result<(), ScanError> {
    let has_recovery_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('library_change_queue_contract')
               WHERE name = 'authoritative_recovery_complete'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_recovery_contract {
        return Err(unverifiable_recovery_contract());
    }
    let recovery_complete = connection
        .query_row(
            "SELECT authoritative_recovery_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !recovery_complete {
        return Err(unverifiable_recovery_contract());
    }
    Ok(())
}

pub(super) fn validate_queue_authority(connection: &Connection) -> Result<(), ScanError> {
    let has_contract = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'library_change_queue_contract'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_contract {
        return Err(unverifiable_queue_authority());
    }
    let authority_complete = connection
        .query_row(
            "SELECT root_authority_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !authority_complete {
        return Err(unverifiable_queue_authority());
    }
    Ok(())
}

pub(super) fn unverifiable_queue_authority() -> ScanError {
    ScanError::new(
        "catalog_change_queue_authority_unverifiable",
        "This prerelease schema 17 catalog cannot prove its highest root generations and must not be opened",
    )
}

pub(super) fn unverifiable_recovery_contract() -> ScanError {
    ScanError::new(
        "catalog_authoritative_recovery_contract_unverifiable",
        "This prerelease schema 18 catalog cannot prove its authoritative recovery contract",
    )
}
