use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rusqlite::{params, params_from_iter};

use crate::domain::ScanError;
use crate::domain::file_operation_journal::FILE_OPERATION_PAGE_SIZE;
use crate::domain::file_operation_selection::{
    FileOperationCandidate, FileOperationSelectionScope as Scope, FileOperationSelectionSummary,
};
use crate::domain::organization::{
    OrganizationSelection, stale_selection, validate_identifier, validate_selections,
};
use crate::ports::CatalogRepository;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;

use super::super::{
    SqliteCatalog, database_error, gallery, load_catalog_revision, sqlite_integer, sqlite_unsigned,
};

mod query;

pub(crate) struct SqliteFileOperationSelection {
    catalog: SqliteCatalog,
    scope: Scope,
    prepared: Option<FileOperationSelectionSummary>,
}

impl SqliteFileOperationSelection {
    pub(crate) fn new(
        catalog: SqliteCatalog,
        scope: Scope,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Self, ScanError> {
        if let Scope::Gallery { query, .. } = &scope {
            gallery::validate_gallery_query(query)?;
        }
        catalog.connection.execute_batch(
            "PRAGMA temp_store = FILE; PRAGMA temp.cache_size = -2048;
             CREATE TEMP TABLE file_operation_selection_input (
               asset_id TEXT NOT NULL, location_id TEXT NOT NULL, source_generation INTEGER NOT NULL,
               PRIMARY KEY(asset_id, location_id)) WITHOUT ROWID;
             CREATE TEMP TABLE file_operation_selection (
               ordinal INTEGER PRIMARY KEY, asset_id TEXT NOT NULL, location_id TEXT NOT NULL UNIQUE,
               source_generation INTEGER NOT NULL, root_generation INTEGER NOT NULL, file_size INTEGER NOT NULL);
             CREATE INDEX temp.file_operation_selection_asset ON file_operation_selection(asset_id);"
        ).map_err(database_error)?;
        catalog
            .connection
            .progress_handler(1000, Some(move || cancelled.load(Ordering::Acquire)))
            .map_err(database_error)?;
        Ok(Self {
            catalog,
            scope,
            prepared: None,
        })
    }

    fn require_collecting(&self) -> Result<(), ScanError> {
        if self.prepared.is_some() {
            return Err(stale_selection());
        }
        Ok(())
    }
}

impl FileOperationSelectionRepository for SqliteFileOperationSelection {
    fn append_ids(&mut self, ids: &[String]) -> Result<(), ScanError> {
        self.require_collecting()?;
        if !matches!(self.scope, Scope::Gallery { .. }) || ids.len() > 256 {
            return Err(stale_selection());
        }
        let transaction = self
            .catalog
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        for id in ids {
            validate_identifier(id)?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO temp.file_operation_selection_input VALUES (?1, '', 0)",
                    [id],
                )
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)
    }

    fn append_locations(&mut self, locations: &[OrganizationSelection]) -> Result<(), ScanError> {
        self.require_collecting()?;
        if !matches!(self.scope, Scope::Locations) {
            return Err(stale_selection());
        }
        validate_selections(locations)?;
        let transaction = self
            .catalog
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        for location in locations {
            let inserted = transaction
                .execute(
                    "INSERT OR IGNORE INTO temp.file_operation_selection_input VALUES (?1, ?2, ?3)",
                    params![
                        location.asset_id,
                        location.location_id,
                        sqlite_integer(location.source_generation, "selected generation")?
                    ],
                )
                .map_err(database_error)?;
            if inserted == 0 {
                let generation: i64 = transaction.query_row(
                    "SELECT source_generation FROM temp.file_operation_selection_input WHERE asset_id = ?1 AND location_id = ?2",
                    params![location.asset_id, location.location_id],
                    |row| row.get(0),
                ).map_err(database_error)?;
                if sqlite_unsigned(generation, "selected generation")? != location.source_generation
                {
                    return Err(stale_selection());
                }
            }
        }
        transaction.commit().map_err(database_error)
    }

    fn prepare(&mut self) -> Result<FileOperationSelectionSummary, ScanError> {
        if let Some(summary) = &self.prepared {
            return Ok(summary.clone());
        }
        let transaction = self
            .catalog
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        if let Scope::Gallery {
            catalog_revision, ..
        } = self.scope
            && load_catalog_revision(&transaction)? != catalog_revision
        {
            return Err(stale_selection());
        }
        let selected = query::selected(&self.scope)?;
        transaction.execute(&format!("INSERT INTO temp.file_operation_selection(asset_id, location_id, source_generation, root_generation, file_size) {}", selected.sql), params_from_iter(selected.parameters)).map_err(database_error)?;
        query::require_all_inputs(&transaction, &self.scope)?;
        let (count, bytes) = transaction
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(file_size), 0) FROM temp.file_operation_selection",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(database_error)?;
        if count == 0 {
            return Err(stale_selection());
        }
        let summary = FileOperationSelectionSummary {
            file_count: sqlite_unsigned(count, "selected count")?,
            total_bytes: sqlite_unsigned(bytes, "selected bytes")?,
        };
        transaction.commit().map_err(database_error)?;
        self.prepared = Some(summary.clone());
        Ok(summary)
    }

    fn page(&self, after_ordinal: Option<u64>) -> Result<Vec<FileOperationCandidate>, ScanError> {
        if self.prepared.is_none() {
            return Err(stale_selection());
        }
        let after = after_ordinal
            .map(|value| sqlite_integer(value, "selection cursor"))
            .transpose()?
            .unwrap_or(0);
        let mut statement = self.catalog.connection.prepare(
            "SELECT ordinal, asset_id, location_id, source_generation, root_generation FROM temp.file_operation_selection
             WHERE ordinal > ?1 ORDER BY ordinal LIMIT ?2").map_err(database_error)?;
        let rows = statement
            .query_map(params![after, FILE_OPERATION_PAGE_SIZE as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(database_error)?;
        rows.map(|row| {
            let (ordinal, asset_id, location_id, generation, root_generation) =
                row.map_err(database_error)?;
            let location = self
                .catalog
                .load_active_location(&location_id)?
                .ok_or_else(stale_selection)?;
            if location.asset_id != asset_id
                || location.source_generation != sqlite_unsigned(generation, "selected generation")?
            {
                return Err(stale_selection());
            }
            Ok(FileOperationCandidate {
                ordinal: sqlite_unsigned(ordinal, "selected ordinal")?,
                location,
                root_generation: sqlite_unsigned(root_generation, "selected root generation")?,
            })
        })
        .collect()
    }
}
