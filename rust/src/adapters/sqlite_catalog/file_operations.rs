use std::path::PathBuf;

use crate::domain::file_operation_journal::{
    AuthorizedFileOperation, FILE_OPERATION_PAGE_SIZE, FileOperationEvent, FileOperationRecord,
    invalid,
};
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::file_operation_journal::FileOperationJournal;

use super::{SqliteCatalog, SqliteCatalogSession, database_error};

mod codec;
pub(super) mod observed_removals;
mod publication;
mod reads;
mod selection;
pub(crate) use selection::SqliteFileOperationSelection;
pub(super) mod scan_removals;
pub(super) mod validation;
mod writes;

pub struct SqliteFileOperationJournal {
    catalog: SqliteCatalog,
}

impl SqliteFileOperationJournal {
    pub(crate) fn from_catalog(catalog: SqliteCatalog) -> Self {
        Self { catalog }
    }

    pub fn open(path: PathBuf) -> Result<Self, ScanError> {
        Ok(Self {
            catalog: SqliteCatalogSession::validate(path)?.open_in_lane(LibraryChangeLane::Live)?,
        })
    }
}

impl FileOperationJournal for SqliteFileOperationJournal {
    fn cancel_unstarted(
        &mut self,
        ids: &[u64],
        now_unix_ms: i64,
    ) -> Result<Vec<FileOperationRecord>, ScanError> {
        use crate::domain::file_operation_journal::FileOperationPhase;
        check_page(ids.len())?;
        let transaction = self.catalog.begin_user_interactive_write()?;
        let mut records = Vec::with_capacity(ids.len());
        for id in ids {
            let record = reads::load(&transaction, *id)?;
            records.push(
                if matches!(
                    record.checkpoint.phase,
                    FileOperationPhase::Prepared | FileOperationPhase::NameOccupied
                ) {
                    writes::advance(
                        &transaction,
                        *id,
                        record.checkpoint.version,
                        FileOperationEvent::Cancelled,
                        now_unix_ms,
                    )?
                } else {
                    record
                },
            );
        }
        transaction.commit().map_err(database_error)?;
        Ok(records)
    }

    fn batch_count(&self, batch_id: &str) -> Result<u64, ScanError> {
        reads::batch_count(&self.catalog.connection, batch_id)
    }

    fn batch_page(
        &self,
        batch_id: &str,
        after_ordinal: Option<u64>,
        limit: usize,
    ) -> Result<Vec<FileOperationRecord>, ScanError> {
        check_page(limit)?;
        reads::batch_page(&self.catalog.connection, batch_id, after_ordinal, limit)
    }

    fn authorize(
        &mut self,
        items: &[AuthorizedFileOperation],
    ) -> Result<Vec<FileOperationRecord>, ScanError> {
        check_page(items.len())?;
        let transaction = self.catalog.begin_user_interactive_write()?;
        let records = writes::authorize(&transaction, items)?;
        transaction.commit().map_err(database_error)?;
        Ok(records)
    }

    fn load(&self, id: u64) -> Result<FileOperationRecord, ScanError> {
        reads::load(&self.catalog.connection, id)
    }

    fn advance(
        &mut self,
        id: u64,
        expected_version: u64,
        event: FileOperationEvent,
        now_unix_ms: i64,
    ) -> Result<FileOperationRecord, ScanError> {
        let transaction = self.catalog.begin_user_interactive_write()?;
        let record = writes::advance(&transaction, id, expected_version, event, now_unix_ms)?;
        transaction.commit().map_err(database_error)?;
        Ok(record)
    }

    fn history(
        &self,
        before_id: Option<u64>,
        limit: usize,
    ) -> Result<Vec<FileOperationRecord>, ScanError> {
        check_page(limit)?;
        reads::history(&self.catalog.connection, before_id, limit)
    }

    fn unfinished(
        &self,
        after_id: u64,
        limit: usize,
    ) -> Result<Vec<FileOperationRecord>, ScanError> {
        check_page(limit)?;
        reads::unfinished(&self.catalog.connection, after_id, limit)
    }
}

fn check_page(limit: usize) -> Result<(), ScanError> {
    if limit == 0 || limit > FILE_OPERATION_PAGE_SIZE {
        return Err(invalid(
            "An operation-journal page must contain 1 to 128 items",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
