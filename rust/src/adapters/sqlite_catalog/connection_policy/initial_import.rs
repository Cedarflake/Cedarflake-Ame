use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rusqlite::Connection;

use super::{
    AUTOMATIC_CHECKPOINT_PAGES, INITIAL_IMPORT_CHECKPOINT_PAGES, ScanError, SqliteCatalog,
    configure_automatic_checkpoint, database_error,
};

const INITIAL_IMPORT_CACHE_KIB: i64 = 32 * 1024;

#[derive(Default)]
pub(in crate::adapters::sqlite_catalog) struct ImportCheckpointPolicy {
    active_imports: AtomicUsize,
}

pub(crate) struct InitialImportWriteWindow {
    policy: Arc<ImportCheckpointPolicy>,
}

struct InitialImportConnectionScope<'a> {
    catalog: &'a mut SqliteCatalog,
    previous_cache_size: Option<i64>,
    _window: Option<InitialImportWriteWindow>,
}

impl InitialImportConnectionScope<'_> {
    fn restore_cache(&mut self) -> Result<(), ScanError> {
        if let Some(previous) = self.previous_cache_size {
            self.catalog
                .connection
                .pragma_update(None, "cache_size", previous)
                .map_err(database_error)?;
            self.previous_cache_size = None;
        }
        Ok(())
    }
}

impl Drop for InitialImportConnectionScope<'_> {
    fn drop(&mut self) {
        let _ = self.restore_cache();
    }
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
    pub(crate) fn with_initial_import_writes<T>(
        &mut self,
        scan_id: &str,
        operation: impl FnOnce(&mut Self) -> Result<T, ScanError>,
    ) -> Result<T, ScanError> {
        let window = self.initial_import_write_window(scan_id)?;
        let previous_cache_size = if window.is_some() {
            let previous = self
                .connection
                .pragma_query_value(None, "cache_size", |row| row.get::<_, i64>(0))
                .map_err(database_error)?;
            self.connection
                .pragma_update(None, "cache_size", -INITIAL_IMPORT_CACHE_KIB)
                .map_err(database_error)?;
            Some(previous)
        } else {
            None
        };
        let mut scope = InitialImportConnectionScope {
            catalog: self,
            previous_cache_size,
            _window: window,
        };
        let result = operation(scope.catalog);
        let restored = scope.restore_cache();
        let value = result?;
        restored?;
        Ok(value)
    }

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
