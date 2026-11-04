use rusqlite::params;

use crate::domain::{
    FileIdentityEvidence, IncrementalCatalogRoot, LeasedLibraryChange, LibraryChangeIntent,
    LibraryChangeQueuePolicy, MetadataInventoryRun, ScanError,
};
use crate::ports::search_index::{IndexedFileHint, IndexedRecoveryRepository};

use super::{SqliteCatalog, database_error};

impl IndexedRecoveryRepository for SqliteCatalog {
    fn begin_search_index_hints(&self) -> Result<(), ScanError> {
        self.connection
            .execute_batch(
                "PRAGMA temp_store = FILE;
             PRAGMA temp.cache_size = -2048;
             DROP TABLE IF EXISTS temp.startup_index_hints;
             CREATE TEMP TABLE startup_index_hints(
               relative_path TEXT PRIMARY KEY, file_size INTEGER, modified_unix_ms INTEGER,
               is_image_candidate INTEGER NOT NULL
             ) WITHOUT ROWID;",
            )
            .map_err(database_error)
    }

    fn stage_search_index_hints(&mut self, hints: &[IndexedFileHint]) -> Result<(), ScanError> {
        if hints.len() > 256 {
            return Err(ScanError::new(
                "search_index_page_too_large",
                "Index hints exceed one bounded page",
            ));
        }
        // This connection-local scratch table grants no catalog or absence authority.
        let transaction = self.connection.transaction().map_err(database_error)?;
        {
            let mut insert = transaction
                .prepare_cached(
                    "INSERT OR REPLACE INTO temp.startup_index_hints VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(database_error)?;
            for hint in hints {
                insert
                    .execute(params![
                        hint.relative_path,
                        hint.file_size.and_then(|size| i64::try_from(size).ok()),
                        hint.modified_unix_ms,
                        hint.is_image_candidate
                    ])
                    .map_err(database_error)?;
            }
        }
        transaction.commit().map_err(database_error)
    }

    fn changed_search_index_paths(
        &self,
        root: &IncrementalCatalogRoot,
        limit: u32,
    ) -> Result<Vec<String>, ScanError> {
        if limit == 0 || limit > 128 {
            return Err(ScanError::new(
                "search_index_candidate_limit_invalid",
                "Index hint admission requires a bounded path window",
            ));
        }
        let mut statement = self
            .connection
            .prepare_cached(
                "SELECT hint.relative_path FROM temp.startup_index_hints AS hint
             LEFT JOIN asset_locations AS location
               ON location.root_id = ?1 AND location.scan_id = ?2
               AND location.relative_path = hint.relative_path
             WHERE (location.location_id IS NULL AND hint.is_image_candidate = 1)
               OR (location.location_id IS NOT NULL AND (hint.file_size IS NOT location.file_size
                 OR hint.modified_unix_ms IS NOT location.modified_unix_ms))
             UNION ALL
             SELECT location.relative_path FROM asset_locations AS location
             WHERE location.root_id = ?1 AND location.scan_id = ?2
               AND NOT EXISTS(SELECT 1 FROM temp.startup_index_hints AS hint
                 WHERE hint.relative_path = location.relative_path)
             LIMIT ?3",
            )
            .map_err(database_error)?;
        statement
            .query_map(params![root.root_id, root.active_scan_id, limit], |row| {
                row.get(0)
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)
    }

    fn bind_indexed_recovery_namespace(
        &mut self,
        run: &MetadataInventoryRun,
        lease: &LeasedLibraryChange,
        identity: &FileIdentityEvidence,
        now_unix_ms: i64,
    ) -> Result<(), ScanError> {
        self.initialize_metadata_inventory_spool(run, lease, identity, None, Some(""), now_unix_ms)
            .map(|_| ())
    }

    fn publish_indexed_recovery_candidates(
        &mut self,
        lease: &LeasedLibraryChange,
        run_id: &str,
        intents: &[LibraryChangeIntent],
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<u32, ScanError> {
        self.publish_indexed_recovery_candidates(lease, run_id, intents, now_unix_ms, policy)
    }
}
