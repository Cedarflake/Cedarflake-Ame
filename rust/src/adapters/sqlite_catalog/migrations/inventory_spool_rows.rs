use rusqlite::Connection;

use crate::domain::ScanError;

use super::database_error;

#[cfg(test)]
mod tests;

pub(super) fn validate(
    connection: &Connection,
    expected_contract_version: i64,
) -> Result<(), ScanError> {
    let invalid_relations = connection
        .query_row(
            "WITH directory_ordinals AS (
               SELECT ordinal,
                      ROW_NUMBER() OVER (PARTITION BY run_id ORDER BY ordinal) - 1 AS expected
               FROM library_metadata_inventory_spool_directories
             ), entry_counts AS MATERIALIZED (
               SELECT run_id, directory_relative_path, COUNT(*) AS actual_count
               FROM library_metadata_inventory_spool_entries
               WHERE directory_relative_path IS NOT NULL
               GROUP BY run_id, directory_relative_path
             )
             SELECT EXISTS(
               SELECT 1
               FROM library_metadata_inventory_spools AS spool
               LEFT JOIN library_metadata_inventory_runs AS run ON run.id = spool.run_id
               LEFT JOIN library_recovery_authorities AS authority
                 ON authority.change_id = spool.authority_change_id
               WHERE run.id IS NULL OR authority.change_id IS NULL
                  OR (?1 = 1 AND run.status <> 'running')
                  OR (?1 >= 2 AND run.status NOT IN ('running', 'comparing', 'completed'))
                  OR authority.retired_unix_ms IS NOT NULL
                  OR authority.run_id <> spool.run_id
                  OR authority.root_id <> spool.root_id
                  OR authority.root_generation <> spool.root_generation
                  OR run.root_id <> spool.root_id
                  OR run.root_generation <> spool.root_generation
                  OR run.scope_kind <> spool.scope_kind
                  OR run.scope_relative_path <> spool.scope_relative_path
             ) OR EXISTS(
               SELECT 1 FROM directory_ordinals WHERE ordinal <> expected
             ) OR EXISTS(
               SELECT 1
               FROM library_metadata_inventory_spool_directories AS directory
               LEFT JOIN entry_counts AS entries
                 ON entries.run_id = directory.run_id
                AND entries.directory_relative_path = directory.relative_directory
               WHERE directory.state = 'completed' AND (
                 directory.directory_identity_scheme IS NULL
                 OR directory.source_entry_count <> COALESCE(entries.actual_count, 0)
               )
             ) OR EXISTS(
               SELECT 1 FROM library_metadata_inventory_spools AS spool
               WHERE (spool.state = 'ready' AND EXISTS(
                 SELECT 1 FROM library_metadata_inventory_spool_directories AS directory
                 WHERE directory.run_id = spool.run_id AND directory.state <> 'completed'
               )) OR (spool.state = 'enumerating' AND NOT EXISTS(
                 SELECT 1 FROM library_metadata_inventory_spool_directories AS directory
                 WHERE directory.run_id = spool.run_id
                   AND directory.state IN ('pending', 'enumerating')
               ))
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check('library_metadata_inventory_spools')
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_spool_directories'
               )
             ) OR EXISTS(
               SELECT 1 FROM pragma_foreign_key_check(
                 'library_metadata_inventory_spool_entries'
               )
             )",
            [expected_contract_version],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_metadata_inventory_spool_contract_unverifiable",
            "The catalog cannot prove its bounded metadata inventory source spool",
        ));
    }
    Ok(())
}
