use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::database_error;
use super::schema_shape::{
    cascade_foreign_key_matches, named_index_matches, schema_object_sql_matches, schema_version,
    table_columns_match,
};

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    validate_with_depth(connection, ContractValidationDepth::Full)
}

pub(super) fn validate_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let mut entry_columns = vec![
        ("run_id", "TEXT", true, 1),
        ("relative_path", "TEXT", true, 2),
        ("entry_kind", "TEXT", true, 0),
        ("file_size", "INTEGER", false, 0),
        ("modified_unix_ms", "INTEGER", true, 0),
        ("file_identity_scheme", "TEXT", false, 0),
        ("file_identity_value", "TEXT", false, 0),
        ("placeholder_state", "TEXT", true, 0),
        ("is_reparse_point", "INTEGER", true, 0),
        ("staged_page_index", "INTEGER", true, 0),
        ("comparison_status", "TEXT", true, 0),
        ("candidate_previous_relative_path", "TEXT", false, 0),
        ("staged_unix_ms", "INTEGER", true, 0),
    ];
    if schema_version(connection)? >= 31 {
        entry_columns.push(("source_revision_token", "TEXT", false, 0));
    }
    let structure_matches = table_columns_match(
        connection,
        "library_metadata_inventory_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_runs",
        &[
            ("id", "TEXT", true, 1),
            ("root_id", "TEXT", true, 0),
            ("root_generation", "INTEGER", true, 0),
            ("epoch", "INTEGER", true, 0),
            ("scope_kind", "TEXT", true, 0),
            ("scope_relative_path", "TEXT", true, 0),
            ("status", "TEXT", true, 0),
            ("next_page_index", "INTEGER", true, 0),
            ("enumeration_cursor", "TEXT", false, 0),
            ("comparison_cursor", "TEXT", false, 0),
            ("absence_cursor", "TEXT", false, 0),
            ("staged_entry_count", "INTEGER", true, 0),
            ("candidate_count", "INTEGER", true, 0),
            ("enumeration_complete", "INTEGER", true, 0),
            ("absence_authority", "INTEGER", true, 0),
            ("started_unix_ms", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
            ("completed_unix_ms", "INTEGER", false, 0),
            ("last_issue_code", "TEXT", false, 0),
            ("last_issue_message", "TEXT", false, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_entries",
        &entry_columns,
    )?;
    let indexes_match = named_index_matches(
        connection,
        "library_metadata_inventory_runs",
        "library_metadata_inventory_runs_cleanup",
        &["status", "updated_unix_ms", "id"],
    )? && named_index_matches(
        connection,
        "library_metadata_inventory_entries",
        "library_metadata_inventory_entries_compare",
        &["run_id", "comparison_status", "relative_path"],
    )? && named_index_matches(
        connection,
        "library_metadata_inventory_entries",
        "library_metadata_inventory_entries_identity",
        &[
            "run_id",
            "file_identity_scheme",
            "file_identity_value",
            "relative_path",
        ],
    )? && named_index_matches(
        connection,
        "library_metadata_inventory_entries",
        "library_metadata_inventory_entries_previous",
        &[
            "run_id",
            "candidate_previous_relative_path",
            "relative_path",
        ],
    )?;
    let active_index_matches = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_index_list('library_metadata_inventory_runs')
               WHERE name = 'library_metadata_inventory_runs_one_active_root'
                 AND \"unique\" = 1 AND partial = 1
             ) AND (SELECT group_concat(name, ',') FROM (
               SELECT name FROM pragma_index_info(
                 'library_metadata_inventory_runs_one_active_root'
               ) ORDER BY seqno
             )) = 'root_id'",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let active_index_predicate_matches = schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_runs_one_active_root",
        "CREATE UNIQUE INDEX library_metadata_inventory_runs_one_active_root
           ON library_metadata_inventory_runs(root_id)
           WHERE status IN ('running', 'comparing')",
    )?;
    let queue_accepts_inventory_origin = connection
        .query_row(
            "SELECT instr(
               replace(replace(replace(replace(lower(sql), ' ', ''), char(10), ''),
                 char(13), ''), char(9), ''),
               '''metadata_inventory'''
             ) > 0
             FROM sqlite_master
             WHERE type = 'table' AND name = 'library_change_queue'",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let foreign_keys_match = cascade_foreign_key_matches(
        connection,
        "library_metadata_inventory_runs",
        "root_id",
        "library_roots",
        "id",
    )? && cascade_foreign_key_matches(
        connection,
        "library_metadata_inventory_entries",
        "run_id",
        "library_metadata_inventory_runs",
        "id",
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND complete = 1
             FROM library_metadata_inventory_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !structure_matches
        || !indexes_match
        || !active_index_matches
        || !active_index_predicate_matches
        || !queue_accepts_inventory_origin
        || !foreign_keys_match
        || !marker_complete
    {
        return Err(unverifiable_contract());
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT
               EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_runs AS runs
                 LEFT JOIN library_roots AS roots ON roots.id = runs.root_id
                 LEFT JOIN library_change_root_state AS state ON state.root_id = runs.root_id
                 WHERE roots.id IS NULL OR state.root_id IS NULL
                    OR (
                      runs.status IN ('running', 'comparing')
                      AND (
                        state.is_active <> 1 OR state.generation <> runs.root_generation
                        OR NOT EXISTS(
                          SELECT 1 FROM scan_runs AS scans
                          WHERE scans.id = roots.active_scan_id
                            AND scans.status = 'completed'
                        )
                      )
                    )
               )
               OR EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_runs AS runs
                 WHERE (
                   runs.status IN ('running', 'comparing')
                   AND runs.staged_entry_count <> (
                     SELECT COUNT(*) FROM library_metadata_inventory_entries AS entries
                     WHERE entries.run_id = runs.id
                   )
                 )
                    OR (
                      runs.status IN ('completed', 'failed', 'cancelled', 'superseded')
                      AND runs.staged_entry_count < (
                        SELECT COUNT(*) FROM library_metadata_inventory_entries AS entries
                        WHERE entries.run_id = runs.id
                      )
                    )
                    OR (runs.scope_kind = 'root' AND runs.scope_relative_path <> '')
                    OR (runs.scope_kind = 'subtree' AND runs.scope_relative_path = '')
               )
               OR EXISTS(
                 SELECT 1
                 FROM library_metadata_inventory_entries AS entries
                 JOIN library_metadata_inventory_runs AS runs ON runs.id = entries.run_id
                 WHERE entries.staged_page_index >= runs.next_page_index
                    OR (
                      runs.scope_kind = 'subtree'
                      AND entries.relative_path <> runs.scope_relative_path
                      AND substr(entries.relative_path, 1, length(runs.scope_relative_path) + 1)
                        <> runs.scope_relative_path || '/'
                    )
               )
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_runs'
               ))
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_entries'
               ))",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(unverifiable_contract());
    }
    Ok(())
}

fn unverifiable_contract() -> ScanError {
    ScanError::new(
        "catalog_metadata_inventory_contract_unverifiable",
        "The catalog cannot prove its metadata-inventory staging and completion authority",
    )
}

#[cfg(test)]
mod tests;
