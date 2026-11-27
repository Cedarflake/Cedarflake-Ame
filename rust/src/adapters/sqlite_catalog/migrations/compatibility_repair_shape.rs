use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::inventory_spool_contract;
use super::schema_shape::schema_object_sql_matches;
use super::{
    LIVE_GAP_RECOVERY_CLAIM_TABLE_DDL, LIVE_GAP_RECOVERY_CONTRACT_TABLE_DDL,
    LIVE_GAP_RECOVERY_IDENTITY_UPDATE_GUARD_DDL, LIVE_GAP_RECOVERY_INSERT_GUARD_DDL,
    LIVE_GAP_RECOVERY_ROOT_INDEX_DDL, METADATA_INVENTORY_RUN_TABLE_DDL,
    RECOVERY_AUTHORITY_TABLE_DDL, SCHEMA_VERSION, database_error,
};

pub(super) fn inventory_shape_matches(
    connection: &Connection,
    schema_version: i64,
) -> Result<bool, ScanError> {
    if schema_version != SCHEMA_VERSION {
        return Ok(false);
    }
    let core_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_runs",
        METADATA_INVENTORY_RUN_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_recovery_authorities",
        RECOVERY_AUTHORITY_TABLE_DDL,
    )?;
    if !core_matches {
        return Ok(false);
    }
    if !inventory_spool_contract::schema_matches(connection)? {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT singleton = 1 AND contract_version = 4 AND complete = 1
             FROM library_metadata_inventory_spool_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)
        .map(|marker| marker.unwrap_or(false))
}

pub(super) fn live_gap_shape_matches(
    connection: &Connection,
    schema_version: i64,
) -> Result<bool, ScanError> {
    if schema_version != SCHEMA_VERSION {
        return Ok(false);
    }
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_live_gap_recovery_contract",
        LIVE_GAP_RECOVERY_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_live_gap_recovery_claims",
        LIVE_GAP_RECOVERY_CLAIM_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_live_gap_recovery_claims_root",
        LIVE_GAP_RECOVERY_ROOT_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_live_gap_recovery_claim_insert_guard",
        LIVE_GAP_RECOVERY_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_live_gap_recovery_claim_identity_update_guard",
        LIVE_GAP_RECOVERY_IDENTITY_UPDATE_GUARD_DDL,
    )?;
    if !schema_matches {
        return Ok(false);
    }
    let marker_complete = connection
        .query_row(
            "SELECT contract_version = 1 AND complete = 1
             FROM library_live_gap_recovery_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    Ok(marker_complete)
}

#[cfg(test)]
mod tests;
