use crate::domain::file_operation_journal::FileOperationRecord;
use crate::domain::file_operation_publication::FileOperationDestinationPublication;
use crate::domain::{IncrementalCatalogRoot, ScanError};

use super::file_operation_journal::FileOperationJournal;

pub trait FileOperationCatalog: FileOperationJournal {
    fn publication_roots(&self) -> Result<Vec<IncrementalCatalogRoot>, ScanError>;

    /// The application retains the native result's source/namespace guards through this call.
    fn publish_operation(
        &mut self,
        id: u64,
        expected_version: u64,
        destination: Option<&FileOperationDestinationPublication>,
        now_unix_ms: i64,
    ) -> Result<FileOperationRecord, ScanError>;
}
