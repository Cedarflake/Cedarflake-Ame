use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    AuthorizedFileOperation, FileOperationEvent, FileOperationRecord,
};

pub trait FileOperationJournal: Send {
    fn authorize(
        &mut self,
        items: &[AuthorizedFileOperation],
    ) -> Result<Vec<FileOperationRecord>, ScanError>;
    fn load(&self, id: u64) -> Result<FileOperationRecord, ScanError>;
    fn advance(
        &mut self,
        id: u64,
        expected_version: u64,
        event: FileOperationEvent,
        now_unix_ms: i64,
    ) -> Result<FileOperationRecord, ScanError>;
    fn history(
        &self,
        before_id: Option<u64>,
        limit: usize,
    ) -> Result<Vec<FileOperationRecord>, ScanError>;
    fn unfinished(
        &self,
        after_id: u64,
        limit: usize,
    ) -> Result<Vec<FileOperationRecord>, ScanError>;
}
