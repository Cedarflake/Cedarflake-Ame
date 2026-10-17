use rusqlite::{Connection, OptionalExtension};

use crate::domain::{PERSISTENT_JOURNAL_CONTRACT_VERSION, ScanError};

use super::current_schema::ContractValidationDepth;
use super::schema_shape::{named_index_matches, schema_object_sql_matches, table_columns_match};
use super::{database_error, record_current_schema_row_audit};

mod lineage;
mod lineage_payload;
mod payload_children;
mod payload_intents;
mod rows;
pub(super) mod schema;
mod state_rows;
pub(super) mod values;

pub(in crate::adapters::sqlite_catalog) use lineage::validate_lineage_rows;
pub(in crate::adapters::sqlite_catalog) use payload_children::validate_payload_children;

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    validate_with_depth(connection, ContractValidationDepth::Full)
}

pub(super) fn validate_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let columns_match = table_columns_match(
        connection,
        "library_persistent_journal_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_root_state",
        &[
            ("root_id", "TEXT", true, 1),
            ("root_generation", "INTEGER", true, 2),
            ("protocol_version", "INTEGER", true, 0),
            ("contract_version", "INTEGER", true, 0),
            ("capability_state", "TEXT", true, 0),
            ("continuity_state", "TEXT", true, 0),
            ("last_failure_code", "TEXT", false, 0),
            ("last_failure_message", "TEXT", false, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_checkpoints",
        &[
            ("root_id", "TEXT", false, 1),
            ("root_generation", "INTEGER", true, 0),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("root_reference_version", "INTEGER", true, 0),
            ("root_file_reference", "BLOB", true, 0),
            ("journal_id", "TEXT", true, 0),
            ("next_unread_usn", "TEXT", true, 0),
            ("captured_exclusive_end", "TEXT", true, 0),
            ("covered_catalog_revision", "INTEGER", true, 0),
            ("protocol_version", "INTEGER", true, 0),
            ("contract_version", "INTEGER", true, 0),
            ("continuity_state", "TEXT", true, 0),
            ("last_failure_code", "TEXT", false, 0),
            ("last_failure_message", "TEXT", false, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_source_ranges",
        &[
            ("id", "TEXT", false, 1),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("journal_id", "TEXT", true, 0),
            ("requested_start_usn", "TEXT", true, 0),
            ("requested_end_usn", "TEXT", true, 0),
            ("covered_until_usn", "TEXT", true, 0),
            ("is_complete", "INTEGER", true, 0),
            ("protocol_version", "INTEGER", true, 0),
            ("contract_version", "INTEGER", true, 0),
            ("status", "TEXT", true, 0),
            ("enrolled_unix_ms", "INTEGER", true, 0),
            ("checkpointed_unix_ms", "INTEGER", false, 0),
            ("canonical_payload", "BLOB", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_queue_lineage",
        &[
            ("source_range_id", "TEXT", true, 1),
            ("change_id", "INTEGER", true, 2),
            ("enrolled_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_cross_root_lineage",
        &[
            ("id", "TEXT", false, 1),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("file_reference_version", "INTEGER", true, 0),
            ("file_reference", "BLOB", true, 0),
            ("previous_root_id", "TEXT", true, 0),
            ("previous_root_generation", "INTEGER", true, 0),
            ("previous_relative_path", "TEXT", true, 0),
            ("current_root_id", "TEXT", true, 0),
            ("current_root_generation", "INTEGER", true, 0),
            ("current_relative_path", "TEXT", true, 0),
            ("status", "TEXT", true, 0),
            ("created_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
            ("journal_id", "TEXT", false, 0),
            ("old_usn", "TEXT", false, 0),
            ("new_usn", "TEXT", false, 0),
            ("previous_carry_id", "TEXT", false, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_cross_root_ranges",
        &[
            ("lineage_id", "TEXT", true, 1),
            ("source_range_id", "TEXT", true, 2),
            ("participant_role", "TEXT", true, 0),
            ("enrolled_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_range_lifecycle",
        &[
            ("source_range_id", "TEXT", false, 1),
            ("lifecycle_state", "TEXT", true, 0),
            ("completed_unix_ms", "INTEGER", false, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_persistent_journal_pending_renames",
        &[
            ("carry_id", "TEXT", false, 1),
            ("source_range_id", "TEXT", true, 0),
            ("volume_guid", "TEXT", true, 0),
            ("volume_serial", "TEXT", true, 0),
            ("journal_id", "TEXT", true, 0),
            ("file_reference_version", "INTEGER", true, 0),
            ("file_reference", "BLOB", true, 0),
            ("old_usn", "TEXT", true, 0),
            ("previous_root_id", "TEXT", true, 0),
            ("previous_root_generation", "INTEGER", true, 0),
            ("previous_relative_path", "TEXT", true, 0),
            ("is_directory", "INTEGER", true, 0),
            ("enrolled_unix_ms", "INTEGER", true, 0),
        ],
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND contract_version = ?1 AND complete = 1
             FROM library_persistent_journal_contract",
            [i64::from(PERSISTENT_JOURNAL_CONTRACT_VERSION)],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let indexes_match = schema_object_sql_matches(
        connection,
        "index",
        "library_change_root_state_generation_identity",
        "CREATE UNIQUE INDEX library_change_root_state_generation_identity
           ON library_change_root_state(root_id, generation)",
    )? && named_index_matches(
        connection,
        "library_persistent_journal_root_state",
        "library_persistent_journal_root_state_continuity",
        &[
            "continuity_state",
            "capability_state",
            "updated_unix_ms",
            "root_id",
        ],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_checkpoints",
        "library_persistent_journal_checkpoints_volume",
        &["volume_guid", "journal_id", "continuity_state", "root_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_source_ranges",
        "library_persistent_journal_source_ranges_root",
        &[
            "root_id",
            "root_generation",
            "status",
            "enrolled_unix_ms",
            "id",
        ],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_source_ranges",
        "library_persistent_journal_source_ranges_volume",
        &["volume_guid", "journal_id", "requested_start_usn", "id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_queue_lineage",
        "library_persistent_journal_queue_lineage_change",
        &["change_id", "source_range_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_cross_root_lineage",
        "library_persistent_journal_cross_root_lineage_previous",
        &[
            "previous_root_id",
            "previous_root_generation",
            "status",
            "id",
        ],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_cross_root_lineage",
        "library_persistent_journal_cross_root_lineage_current",
        &["current_root_id", "current_root_generation", "status", "id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_cross_root_ranges",
        "library_persistent_journal_cross_root_ranges_source",
        &["source_range_id", "lineage_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_range_lifecycle",
        "library_persistent_journal_range_lifecycle_state",
        &["lifecycle_state", "updated_unix_ms", "source_range_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_pending_renames",
        "library_persistent_journal_pending_renames_volume",
        &["volume_guid", "journal_id", "old_usn", "carry_id"],
    )? && named_index_matches(
        connection,
        "library_persistent_journal_pending_renames",
        "library_persistent_journal_pending_renames_source",
        &["source_range_id", "carry_id"],
    )?;
    let checks_and_foreign_keys_match = schema::matches(connection)?;
    if !columns_match || !marker_complete || !indexes_match || !checks_and_foreign_keys_match {
        return Err(values::unverifiable_contract());
    }
    if depth.includes_rows() {
        record_current_schema_row_audit();
        rows::validate(connection)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
