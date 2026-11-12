use crate::domain::ScanError;
use crate::domain::file_operation_exports::{
    FileExportBatch, FileExportRecord, FileExportReport, FileExportSkip,
};
use crate::domain::file_operations::FileOperationDirectory;

pub trait FileExportRepository {
    fn begin_export(&mut self, batch: &FileExportBatch) -> Result<(), ScanError>;
    fn load_export(&self, batch_id: &str) -> Result<Option<FileExportRecord>, ScanError>;
    fn export_history(
        &self,
        before_id: Option<u64>,
        limit: usize,
    ) -> Result<Vec<FileExportRecord>, ScanError>;
    fn record_export_skips(
        &mut self,
        batch_id: &str,
        skipped: &[FileExportSkip],
    ) -> Result<(), ScanError>;
    fn export_skips(
        &self,
        batch_id: &str,
        after_ordinal: Option<u64>,
        limit: usize,
    ) -> Result<Vec<FileExportSkip>, ScanError>;
    fn save_export_report(
        &mut self,
        batch_id: &str,
        report: &FileExportReport,
    ) -> Result<(), ScanError>;
}

pub trait FileExportReportWriter {
    fn append(&mut self, text: &str) -> Result<(), ScanError>;
    fn finish(self: Box<Self>) -> Result<String, ScanError>;
}

pub trait NativeFileExports: Send + Sync {
    fn organization_directory(
        &self,
        root: &FileOperationDirectory,
        components: &[&str],
    ) -> Result<FileOperationDirectory, ScanError>;
    fn begin_export_report(
        &self,
        root: &FileOperationDirectory,
    ) -> Result<Box<dyn FileExportReportWriter>, ScanError>;
}

pub trait NativeFilePreparation:
    super::file_operations::NativeFileOperations + NativeFileExports
{
}
impl<T: super::file_operations::NativeFileOperations + NativeFileExports> NativeFilePreparation
    for T
{
}
