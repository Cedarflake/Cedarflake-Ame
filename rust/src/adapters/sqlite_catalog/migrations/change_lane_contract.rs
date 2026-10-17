use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::schema_shape::{
    cascade_foreign_key_matches, schema_object_sql_matches, table_columns_match,
};
use super::{
    CHANGE_LANE_CONTRACT_TABLE_DDL, CHANGE_LANE_INDEX_DDL, CHANGE_LANE_INSERT_GUARD_DDL,
    CHANGE_LANE_INSERT_TRIGGER_DDL, CHANGE_LANE_ORIGIN_TRIGGER_DDL, CHANGE_LANE_QUEUE_TABLE_DDL,
    CHANGE_LANE_UPDATE_GUARD_DDL, database_error,
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
        "library_change_lane_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_change_queue_lanes",
        &[
            ("change_id", "INTEGER", false, 1),
            ("lane", "TEXT", true, 0),
        ],
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND complete = 1
             FROM library_change_lane_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_change_lane_contract",
        CHANGE_LANE_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_change_queue_lanes",
        CHANGE_LANE_QUEUE_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_change_queue_lanes_eligible",
        CHANGE_LANE_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_change_queue_lane_insert",
        CHANGE_LANE_INSERT_TRIGGER_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_change_queue_lane_origin_update",
        CHANGE_LANE_ORIGIN_TRIGGER_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_change_queue_lane_insert_guard",
        CHANGE_LANE_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_change_queue_lane_update_guard",
        CHANGE_LANE_UPDATE_GUARD_DDL,
    )?;
    let foreign_key_matches = cascade_foreign_key_matches(
        connection,
        "library_change_queue_lanes",
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
               FROM library_change_queue AS queue
               LEFT JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
               WHERE lanes.change_id IS NULL
                  OR lanes.lane <> CASE queue.origin
                    WHEN 'live_notification' THEN 'p0_live'
                    WHEN 'startup_catch_up' THEN 'p1_journal'
                    ELSE 'p2_recovery'
                  END
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_change_queue_lanes')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_rows {
        return Err(unverifiable_contract());
    }
    Ok(())
}

fn unverifiable_contract() -> ScanError {
    ScanError::new(
        "catalog_change_lane_contract_unverifiable",
        "The catalog cannot prove its durable change-lane authority",
    )
}

#[cfg(test)]
mod tests;
