use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::schema_shape::{schema_object_sql_matches, table_columns_match};
use super::{
    METADATA_INVENTORY_CANDIDATE_CHANGE_INDEX_DDL, METADATA_INVENTORY_CANDIDATE_INSERT_GUARD_DDL,
    METADATA_INVENTORY_CANDIDATE_OWNER_TABLE_DDL, METADATA_INVENTORY_CANDIDATE_UPDATE_GUARD_DDL,
    METADATA_INVENTORY_FRONTIER_STATE_INDEX_DDL, METADATA_INVENTORY_FRONTIER_TABLE_DDL,
    RECOVERY_EXECUTION_CONTRACT_TABLE_DDL, database_error,
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
        "library_recovery_execution_contract",
        &[
            ("singleton", "INTEGER", false, 1),
            ("contract_version", "INTEGER", true, 0),
            ("complete", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_candidate_owners",
        &[
            ("run_id", "TEXT", true, 1),
            ("candidate_key", "TEXT", true, 2),
            ("change_id", "INTEGER", true, 0),
            ("candidate_role", "TEXT", true, 0),
            ("relative_path", "TEXT", true, 0),
            ("previous_relative_path", "TEXT", false, 0),
            ("owned_unix_ms", "INTEGER", true, 0),
        ],
    )? && table_columns_match(
        connection,
        "library_metadata_inventory_frontier",
        &[
            ("run_id", "TEXT", true, 1),
            ("ordinal", "INTEGER", true, 2),
            ("relative_directory", "TEXT", true, 0),
            ("state", "TEXT", true, 0),
            ("directory_identity_scheme", "TEXT", false, 0),
            ("directory_identity_value", "TEXT", false, 0),
            ("resume_after_relative_path", "TEXT", false, 0),
            ("enumerated_entry_count", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )?;
    let marker_complete = connection
        .query_row(
            "SELECT singleton = 1 AND contract_version = 1 AND complete = 1
             FROM library_recovery_execution_contract",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let schema_matches = schema_object_sql_matches(
        connection,
        "table",
        "library_recovery_execution_contract",
        RECOVERY_EXECUTION_CONTRACT_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_candidate_owners",
        METADATA_INVENTORY_CANDIDATE_OWNER_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_candidate_owners_change",
        METADATA_INVENTORY_CANDIDATE_CHANGE_INDEX_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_metadata_inventory_candidate_owner_insert_guard",
        METADATA_INVENTORY_CANDIDATE_INSERT_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "trigger",
        "library_metadata_inventory_candidate_owner_update_guard",
        METADATA_INVENTORY_CANDIDATE_UPDATE_GUARD_DDL,
    )? && schema_object_sql_matches(
        connection,
        "table",
        "library_metadata_inventory_frontier",
        METADATA_INVENTORY_FRONTIER_TABLE_DDL,
    )? && schema_object_sql_matches(
        connection,
        "index",
        "library_metadata_inventory_frontier_state",
        METADATA_INVENTORY_FRONTIER_STATE_INDEX_DDL,
    )?;
    if !columns_match || !marker_complete || !schema_matches {
        return Err(ScanError::new(
            "catalog_recovery_execution_contract_unverifiable",
            "The catalog cannot prove recovery candidate ownership and frontier lifecycle",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_relations = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_metadata_inventory_candidate_owners AS owner
               LEFT JOIN library_metadata_inventory_runs AS run ON run.id = owner.run_id
               LEFT JOIN library_change_queue AS queue ON queue.id = owner.change_id
               WHERE run.id IS NULL OR queue.id IS NULL
                  OR queue.root_id <> run.root_id
                  OR queue.root_generation <> run.root_generation
                  OR queue.scope <> 'path'
                  OR queue.relative_path <> owner.relative_path
                  OR queue.previous_relative_path IS NOT owner.previous_relative_path
             ) OR EXISTS(
               SELECT 1
               FROM library_metadata_inventory_frontier AS frontier
               LEFT JOIN library_metadata_inventory_runs AS run ON run.id = frontier.run_id
               WHERE run.id IS NULL
                  OR (run.status IN ('completed', 'failed', 'cancelled', 'superseded')
                    AND frontier.state <> 'completed')
                  OR frontier.ordinal <> (
                    SELECT COUNT(*)
                    FROM library_metadata_inventory_frontier AS earlier
                    WHERE earlier.run_id = frontier.run_id
                      AND earlier.ordinal < frontier.ordinal
                  )
                  OR (frontier.state = 'pending' AND (
                    frontier.ordinal <> (
                      SELECT MAX(last.ordinal)
                      FROM library_metadata_inventory_frontier AS last
                      WHERE last.run_id = frontier.run_id
                    )
                    OR frontier.resume_after_relative_path IS NOT NULL
                    OR frontier.enumerated_entry_count <> 0
                    OR NOT EXISTS(
                      SELECT 1
                      FROM library_metadata_inventory_entries AS entry
                      WHERE entry.run_id = frontier.run_id
                        AND entry.relative_path = frontier.relative_directory
                        AND entry.entry_kind = 'directory'
                    )
                  ))
                  OR (frontier.state = 'enumerating'
                    AND frontier.resume_after_relative_path IS NOT NULL
                    AND NOT EXISTS(
                      SELECT 1
                      FROM library_metadata_inventory_entries AS entry
                      WHERE entry.run_id = frontier.run_id
                        AND entry.relative_path = frontier.resume_after_relative_path
                    ))
             ) OR EXISTS(
               SELECT 1
               FROM library_metadata_inventory_runs AS run
               WHERE (run.status = 'running' AND (
                 run.enumeration_complete <> 0
                 OR (run.next_page_index = 1 AND (
                   run.staged_entry_count <> 0
                   OR EXISTS(
                     SELECT 1 FROM library_metadata_inventory_frontier AS frontier
                     WHERE frontier.run_id = run.id
                   )
                 ))
                 OR (run.next_page_index > 1 AND NOT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_frontier AS frontier
                   WHERE frontier.run_id = run.id
                     AND frontier.state IN ('pending', 'enumerating')
                 ))
                 OR EXISTS(
                   SELECT 1 FROM library_metadata_inventory_frontier AS frontier
                   WHERE frontier.run_id = run.id AND frontier.state = 'completed'
                 )
               ))
               OR (run.status = 'comparing' AND (
                 run.enumeration_complete <> 1
                 OR 1 <> (
                   SELECT COUNT(*) FROM library_metadata_inventory_frontier AS frontier
                   WHERE frontier.run_id = run.id AND frontier.state = 'completed'
                 )
                 OR 1 <> (
                   SELECT COUNT(*) FROM library_metadata_inventory_frontier AS frontier
                   WHERE frontier.run_id = run.id
                 )
               ))
             ) OR EXISTS(
               SELECT 1
               FROM library_recovery_authorities AS authority
               LEFT JOIN library_change_queue AS queue ON queue.id = authority.change_id
               LEFT JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
               WHERE queue.id IS NULL
                  OR (authority.retired_unix_ms IS NULL
                    AND queue.status NOT IN ('pending', 'leased', 'retry_wait'))
                  OR (authority.retired_unix_ms IS NOT NULL
                    AND queue.status NOT IN ('completed', 'superseded'))
                  OR (authority.retired_unix_ms IS NOT NULL
                    AND run.id IS NOT NULL AND run.status <> 'completed')
                  OR (run.status = 'completed' AND EXISTS(
                    SELECT 1
                    FROM library_metadata_inventory_candidate_owners AS owner
                    JOIN library_change_queue AS candidate ON candidate.id = owner.change_id
                    WHERE owner.run_id = run.id
                      AND candidate.status IN ('pending', 'leased', 'retry_wait')
                  ))
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_candidate_owners'
               )
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_metadata_inventory_frontier')
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_recovery_execution_contract_unverifiable",
            "The catalog cannot prove recovery candidate ownership and frontier lifecycle",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
