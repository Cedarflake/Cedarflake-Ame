use super::ScanError;
use super::file_operations::{FileAction, FileOperationDirectory};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileOrganizationRule {
    #[default]
    Direct,
    Category,
    Album,
    Date,
    Source,
    AlbumYear,
}

#[derive(Clone, Debug)]
pub struct FileExportBatch {
    pub batch_id: String,
    pub authorized_unix_ms: i64,
    pub action: FileAction,
    pub rule: FileOrganizationRule,
    pub selected_files: u64,
    pub root: FileOperationDirectory,
}

#[derive(Clone, Debug)]
pub struct FileExportSkip {
    pub ordinal: u64,
    pub source: String,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub enum FileExportReport {
    Pending,
    NotNeeded,
    Written(String),
    Failed(ScanError),
}

#[derive(Clone, Debug)]
pub struct FileExportRecord {
    pub id: u64,
    pub authorized_at_utc: String,
    pub batch: FileExportBatch,
    pub skipped_files: u64,
    pub report: FileExportReport,
}

#[derive(Clone, Debug)]
pub enum FileOrganizationDestination {
    Direct,
    Folder(String),
    AlbumYear { album: String, year: String },
    Skip(String),
}
