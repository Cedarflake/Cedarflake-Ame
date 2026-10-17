use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::current_schema::validate_current_schema_contract_with_source_revision_rows;
use super::database_error;
use super::schema_shape::schema_object_sql_matches;

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

pub(super) fn invalidate_precontract_metadata(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "DELETE FROM preview_artifact_locations;
             UPDATE preview_artifacts SET lifecycle_state = 'evictable';
             UPDATE asset_locations
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL,
                 metadata_engine_id = 'ame-invalidated-media-metadata',
                 metadata_engine_version = '0',
                 capture_local_time = NULL, capture_offset_minutes = NULL,
                 capture_time_source = NULL, capture_raw_value = NULL;",
        )
        .map_err(database_error)
}

pub(super) fn create_metadata_contract(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(SOURCE_REVISION_METADATA_CONTRACT_TABLE_DDL)
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO library_source_revision_metadata_contract(
               singleton, contract_version, complete
             ) VALUES (1, 1, 1)",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn repair_precontract_metadata(connection: &mut Connection) -> Result<(), ScanError> {
    let marker_sql = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'table'
               AND name = 'library_source_revision_metadata_contract'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    if marker_sql.is_some() {
        if metadata_contract_is_complete(connection)? {
            return Ok(());
        }
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove that legacy source metadata was invalidated",
        ));
    }

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    validate_current_schema_contract_with_source_revision_rows(&transaction)?;
    invalidate_precontract_metadata(&transaction)?;
    create_metadata_contract(&transaction)?;
    if !metadata_contract_is_complete(&transaction)? {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog could not record its source metadata invalidation",
        ));
    }
    validate_current_schema_contract_with_source_revision_rows(&transaction)?;
    transaction.commit().map_err(database_error)
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
    let invalid_state = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM asset_locations
               WHERE source_generation IS NULL OR source_generation <= 0
                  OR (source_revision_token IS NOT NULL AND (
                    length(source_revision_token) <> 50
                    OR source_revision_token NOT GLOB
                      'windows-file-change-time-100ns-v1:[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]'
                  ))
             ) OR EXISTS(
               SELECT 1 FROM preview_artifacts
               WHERE lifecycle_state = 'ready'
                 AND (source_generation IS NULL OR source_generation <= 0
                      OR source_revision_token IS NULL)
             ) OR EXISTS(
               SELECT 1
               FROM asset_locations AS locations
               JOIN library_roots AS roots
                 ON roots.id = locations.root_id
                AND roots.active_scan_id = locations.scan_id
               WHERE locations.file_identity_scheme IS NOT NULL
               GROUP BY locations.file_identity_scheme, locations.file_identity_value
               HAVING MIN(locations.source_generation) <> MAX(locations.source_generation)
                  OR COUNT(DISTINCT locations.source_revision_token) > 1
                  OR MIN(locations.file_size) <> MAX(locations.file_size)
                  OR MIN(locations.modified_unix_ms) <> MAX(locations.modified_unix_ms)
             ) OR EXISTS(
               SELECT 1 FROM asset_locations AS locations
               WHERE locations.file_identity_scheme IS NOT NULL
               GROUP BY locations.scan_id, locations.file_identity_scheme,
                        locations.file_identity_value
               HAVING MIN(locations.source_generation) <> MAX(locations.source_generation)
                  OR COUNT(DISTINCT locations.source_revision_token) > 1
                  OR MIN(locations.file_size) <> MAX(locations.file_size)
                  OR MIN(locations.modified_unix_ms) <> MAX(locations.modified_unix_ms)
             ) OR (SELECT next_source_generation FROM catalog_state) <= COALESCE(
               (SELECT MAX(source_generation) FROM asset_locations), 0
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_state {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove its source revision and generation contract",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
