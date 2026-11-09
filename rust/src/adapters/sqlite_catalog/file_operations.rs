use std::path::PathBuf;

use crate::domain::file_operation_journal::{
    AuthorizedFileOperation, FILE_OPERATION_PAGE_SIZE, FileOperationEvent, FileOperationRecord,
    invalid,
};
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::file_operation_journal::FileOperationJournal;

use super::{SqliteCatalog, SqliteCatalogSession, database_error};

mod codec;
mod reads;
pub(super) mod validation;
mod writes;

pub struct SqliteFileOperationJournal {
    catalog: SqliteCatalog,
}

impl SqliteFileOperationJournal {
    pub fn open(path: PathBuf) -> Result<Self, ScanError> {
        Ok(Self {
            catalog: SqliteCatalogSession::validate(path)?.open_in_lane(LibraryChangeLane::Live)?,
        })
    }
}

impl FileOperationJournal for SqliteFileOperationJournal {
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
