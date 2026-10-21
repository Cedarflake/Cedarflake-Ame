use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rusqlite::Connection;

use super::{
    AUTOMATIC_CHECKPOINT_PAGES, INITIAL_IMPORT_CHECKPOINT_PAGES, ScanError, SqliteCatalog,
    configure_automatic_checkpoint, database_error,
};

#[derive(Default)]
pub(in crate::adapters::sqlite_catalog) struct ImportCheckpointPolicy {
    active_imports: AtomicUsize,
}

pub(crate) struct InitialImportWriteWindow {
    policy: Arc<ImportCheckpointPolicy>,
}

impl ImportCheckpointPolicy {
    fn enter(self: &Arc<Self>) -> InitialImportWriteWindow {
        self.active_imports.fetch_add(1, Ordering::AcqRel);
        InitialImportWriteWindow {
            policy: Arc::clone(self),
        }
    }

    pub(in crate::adapters::sqlite_catalog) fn apply(
        &self,
        connection: &Connection,
    ) -> Result<(), ScanError> {
        let pages = if self.active_imports.load(Ordering::Acquire) == 0 {
            AUTOMATIC_CHECKPOINT_PAGES
        } else {
            INITIAL_IMPORT_CHECKPOINT_PAGES
        };
        configure_automatic_checkpoint(connection, pages)
    }
}

impl Drop for InitialImportWriteWindow {
    fn drop(&mut self) {
        self.policy.active_imports.fetch_sub(1, Ordering::AcqRel);
    }
}

impl SqliteCatalog {
    pub(crate) fn initial_import_write_window(
        &self,
        scan_id: &str,
    ) -> Result<Option<InitialImportWriteWindow>, ScanError> {
        let is_initial_import = self
            .connection
            .query_row(
                "SELECT EXISTS (
                   SELECT 1 FROM scan_runs AS scan
                   JOIN library_roots AS root ON root.id = scan.root_id
                   WHERE scan.id = ?1 AND scan.status = 'running'
                     AND scan.scan_owner = 'foreground' AND root.active_scan_id IS NULL
                 ) AND NOT EXISTS (
                   SELECT 1 FROM library_roots WHERE active_scan_id IS NOT NULL
                 )",
                [scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        Ok(is_initial_import.then(|| self.write_admission.checkpoint_policy.enter()))
    }
}

#[cfg(test)]
mod tests;
