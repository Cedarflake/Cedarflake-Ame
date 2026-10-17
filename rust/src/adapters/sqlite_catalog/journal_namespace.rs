use std::sync::Arc;

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter};

use crate::domain::{FileIdentityEvidence, LibraryChangeLane, ScanError};
use crate::ports::{
    IdentityJournalLocation, IdentityJournalRoot, PersistentJournalNamespaceReader,
};

use super::{
    SqliteCatalog, SqliteCatalogSession, database_error, sqlite_integer,
    validate_root_publication_identity,
};
pub(super) mod publication;
#[cfg(test)]
mod tests;

pub(crate) struct SqliteJournalNamespaceReader(pub(crate) Arc<SqliteCatalogSession>);

impl PersistentJournalNamespaceReader for SqliteJournalNamespaceReader {
    fn has_snapshot(&self, root: &IdentityJournalRoot) -> Result<bool, ScanError> {
        let catalog = self.0.open_in_lane(LibraryChangeLane::Journal)?;
        catalog.has_journal_directory_snapshot(root)
    }

    fn previous_locations(
        &self,
        root: &IdentityJournalRoot,
        identities: &[FileIdentityEvidence],
    ) -> Result<Vec<IdentityJournalLocation>, ScanError> {
        let catalog = self.0.open_in_lane(LibraryChangeLane::Journal)?;
        catalog.load_journal_identity_locations(root, identities)
    }
}

impl SqliteCatalog {
    fn has_journal_directory_snapshot(
        &self,
        root: &IdentityJournalRoot,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
               SELECT 1 FROM library_roots AS root
               JOIN library_change_root_state AS active ON active.root_id = root.id
               WHERE root.id = ?1 AND active.generation = ?2 AND active.is_active = 1
                 AND (EXISTS(
                   SELECT 1 FROM scan_directory_identities AS directory
                   WHERE directory.scan_id = root.active_scan_id AND directory.relative_path = ''
                 ) OR EXISTS(
                   SELECT 1 FROM library_metadata_inventory_spools AS spool
                   JOIN library_metadata_inventory_runs AS run ON run.id = spool.run_id
                   WHERE spool.root_id = root.id AND spool.root_generation = active.generation
                     AND spool.scope_kind = 'root' AND spool.state = 'ready'
                     AND run.status IN ('running', 'comparing')
                 ))
             )",
                params![
                    root.root_id,
                    sqlite_integer(root.root_generation.value(), "namespace root generation")?
                ],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    fn load_journal_identity_locations(
        &self,
        root: &IdentityJournalRoot,
        identities: &[FileIdentityEvidence],
    ) -> Result<Vec<IdentityJournalLocation>, ScanError> {
        if identities.len() > 256 {
            return Err(capacity());
        }
        for identity in identities {
            validate_root_publication_identity(identity)?;
        }
        if identities.is_empty() {
            return Ok(Vec::new());
        }
        let values = std::iter::repeat_n("(?)", identities.len())
            .collect::<Vec<_>>()
            .join(", ");
        // Bounded identity predicates allow composite-index seeks within each namespace.
        // Joining the roster can multiply work across the entire active publication.
        let query = format!(
            "WITH requested(identity_value) AS (VALUES {values}),
             selected_root AS (
               SELECT root.id, root.active_scan_id, active.generation
               FROM library_roots AS root
               JOIN library_change_root_state AS active ON active.root_id = root.id
               WHERE root.id = ? AND active.generation = ? AND active.is_active = 1
             ), locations(identity_value, relative_path, is_directory) AS (
               SELECT file.file_identity_value, file.relative_path, 0
               FROM selected_root AS root
               JOIN asset_locations AS file
                 ON file.scan_id = root.active_scan_id AND file.root_id = root.id
                AND file.file_identity_scheme = 'windows-file-id-128-v1'
                AND file.file_identity_value IN (SELECT identity_value FROM requested)
               UNION
               SELECT directory.identity_value, directory.relative_path, 1
               FROM selected_root AS root
               JOIN scan_directory_identities AS directory
                 ON directory.scan_id = root.active_scan_id
                AND directory.identity_value IN (SELECT identity_value FROM requested)
               UNION
               SELECT directory.directory_identity_value, directory.relative_directory, 1
               FROM selected_root AS root
               JOIN library_metadata_inventory_spools AS spool
                 ON spool.root_id = root.id AND spool.root_generation = root.generation
                AND spool.state IN ('enumerating', 'ready')
               JOIN library_metadata_inventory_runs AS run
                 ON run.id = spool.run_id AND run.status IN ('running', 'comparing')
               JOIN library_metadata_inventory_spool_directories AS directory
                 ON directory.run_id = spool.run_id
                AND directory.directory_identity_scheme = 'windows-file-id-128-v1'
                AND directory.directory_identity_value IN (SELECT identity_value FROM requested)
                AND directory.state IN ('enumerating', 'completed')
             )
             SELECT identity_value, relative_path, is_directory FROM locations
             ORDER BY identity_value, relative_path, is_directory LIMIT 4097"
        );
        let mut parameters: Vec<Value> = identities
            .iter()
            .map(|identity| Value::Text(identity.value.clone()))
            .collect();
        parameters.push(Value::Text(root.root_id.clone()));
        parameters.push(Value::Integer(sqlite_integer(
            root.root_generation.value(),
            "namespace root generation",
        )?));
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let result = {
            let mut statement = transaction.prepare(&query).map_err(database_error)?;
            statement
                .query_map(params_from_iter(parameters), |row| {
                    Ok(IdentityJournalLocation {
                        identity: FileIdentityEvidence {
                            scheme: "windows-file-id-128-v1".to_owned(),
                            value: row.get(0)?,
                        },
                        relative_path: row.get(1)?,
                        is_directory: row.get(2)?,
                    })
                })
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?
        };
        if result.len() > 4096 {
            return Err(capacity());
        }
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
}

fn capacity() -> ScanError {
    ScanError::new(
        "identity_journal_namespace_capacity",
        "The bounded namespace identity window is too large",
    )
}
