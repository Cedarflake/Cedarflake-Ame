use super::exact_duplicate_review::ExactDuplicateReviewKey;
use super::file_operation_exports::{FileOrganizationDestination, FileOrganizationRule};
use super::file_operations::FileAction;
use super::similar_images::review::SimilarityDegree;
use super::{AssetLocationView, GalleryQuery};

#[derive(Clone, Debug)]
pub enum FileOperationSelectionScope {
    Gallery {
        query: Box<GalleryQuery>,
        catalog_revision: u64,
        all_matching: bool,
    },
    MergedGalleryGroups {
        query: Box<GalleryQuery>,
        catalog_revision: u64,
        all_matching: bool,
    },
    Locations,
    DuplicateCleanup {
        kind: DuplicateCleanupKind,
        all_groups: bool,
    },
}

#[derive(Clone, Debug)]
pub enum DuplicateCleanupKind {
    Exact,
    Similar { degree: SimilarityDegree },
}

#[derive(Clone, Debug)]
pub enum DuplicateCleanupGroup {
    Exact { key: ExactDuplicateReviewKey },
    Similar { group_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateCleanupSummary {
    pub group_count: u64,
    pub skipped_groups: u64,
    pub kept_files: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileOperationSelectionSummary {
    pub file_count: u64,
    pub total_bytes: u64,
    pub merged_group_count: Option<u64>,
    pub cleanup: Option<DuplicateCleanupSummary>,
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
