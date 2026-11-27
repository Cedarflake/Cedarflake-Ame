use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::database_error;
use super::schema_shape::schema_object_sql_matches;

mod rows;

const SOURCE_REVISION_METADATA_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_source_revision_metadata_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";

#[cfg(test)]
std::thread_local! {
    static SOURCE_REVISION_ROW_AUDIT_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_row_audit_count() {
    SOURCE_REVISION_ROW_AUDIT_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(super) fn row_audit_count() -> usize {
    SOURCE_REVISION_ROW_AUDIT_COUNT.with(std::cell::Cell::get)
}

pub(super) fn metadata_contract_is_complete(connection: &Connection) -> Result<bool, ScanError> {
    if !schema_object_sql_matches(
        connection,
        "table",
        "library_source_revision_metadata_contract",
        SOURCE_REVISION_METADATA_CONTRACT_TABLE_DDL,
    )? {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT contract_version = 1 AND complete = 1
             FROM library_source_revision_metadata_contract
             WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map(|value| value.unwrap_or(false))
        .map_err(database_error)
}

pub(super) fn validate_structure(connection: &Connection) -> Result<(), ScanError> {
    let required_columns = [
        ("catalog_state", "next_source_generation"),
        ("asset_locations", "source_revision_token"),
        ("asset_locations", "source_generation"),
        ("preview_artifacts", "source_revision_token"),
        ("preview_artifacts", "source_generation"),
        ("library_terminal_media_evidence", "source_revision_token"),
        ("library_terminal_media_evidence", "source_generation"),
        ("library_change_catch_up_handoffs", "source_revision_token"),
        ("library_change_catch_up_handoffs", "source_generation"),
        ("library_change_scan_handoff_items", "source_revision_token"),
        ("library_change_scan_handoff_items", "source_generation"),
        (
            "library_metadata_inventory_entries",
            "source_revision_token",
        ),
        (
            "library_metadata_inventory_spool_entries",
            "source_revision_token",
        ),
    ];
    let mut structure_matches = true;
    for (table, column) in required_columns {
        structure_matches &= connection
            .query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1)"
                ),
                [column],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
    }
    structure_matches &= schema_object_sql_matches(
        connection,
        "trigger",
        "asset_locations_source_generation_insert_guard",
        "CREATE TRIGGER asset_locations_source_generation_insert_guard
           BEFORE INSERT ON asset_locations
           WHEN NEW.source_generation IS NULL OR NEW.source_generation <= 0
           BEGIN
             SELECT RAISE(ABORT, 'asset location source generation is required');
           END",
    )?;
    structure_matches &= schema_object_sql_matches(
        connection,
        "trigger",
        "asset_locations_source_generation_update_guard",
        "CREATE TRIGGER asset_locations_source_generation_update_guard
           BEFORE UPDATE OF source_generation ON asset_locations
           WHEN NEW.source_generation IS NULL OR NEW.source_generation <= 0
           BEGIN
             SELECT RAISE(ABORT, 'asset location source generation is required');
           END",
    )?;
    if !structure_matches {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove its source revision and generation contract",
        ));
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    #[cfg(test)]
    SOURCE_REVISION_ROW_AUDIT_COUNT.with(|count| count.set(count.get() + 1));
    rows::validate(connection)
}

#[cfg(test)]
mod tests;
