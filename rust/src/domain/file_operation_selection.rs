use super::file_operation_exports::{FileOrganizationDestination, FileOrganizationRule};
use super::file_operations::FileAction;
use super::{AssetLocationView, GalleryQuery};

#[derive(Clone, Debug)]
pub enum FileOperationSelectionScope {
    Gallery {
        query: Box<GalleryQuery>,
        catalog_revision: u64,
        all_matching: bool,
    },
    Locations,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileOperationSelectionSummary {
    pub file_count: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct FileOperationCandidate {
    pub ordinal: u64,
    pub location: AssetLocationView,
    pub root_generation: u64,
    pub organization_destination: FileOrganizationDestination,
}

#[derive(Clone, Debug)]
pub struct FileOperationCommand {
    pub action: FileAction,
    pub destination_directory: Option<String>,
    pub new_name: Option<String>,
    pub organization_rule: FileOrganizationRule,
}

#[derive(Clone, Debug)]
pub struct FileOperationPreparationProgress {
    pub prepared_files: u64,
    pub total_files: u64,
    pub current_path: Option<String>,
}
