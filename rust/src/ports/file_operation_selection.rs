use crate::domain::file_operation_exports::FileOrganizationRule;
use crate::domain::file_operation_selection::{
    DuplicateCleanupGroup, FileOperationCandidate, FileOperationSelectionSummary,
};
use crate::domain::organization::OrganizationSelection;
use crate::domain::{AssetLocationView, ScanError};

pub trait FileOperationSelectionRepository: Send {
    fn append_ids(&mut self, asset_ids: &[String]) -> Result<(), ScanError>;
    fn append_locations(&mut self, locations: &[OrganizationSelection]) -> Result<(), ScanError>;
    fn append_groups(&mut self, groups: &[DuplicateCleanupGroup]) -> Result<(), ScanError>;
    fn prepare(&mut self) -> Result<FileOperationSelectionSummary, ScanError>;
    fn freeze_rule(&mut self, rule: FileOrganizationRule) -> Result<(), ScanError>;
    fn page(&self, after_ordinal: Option<u64>) -> Result<Vec<FileOperationCandidate>, ScanError>;
    fn retained_page(&self, after_location: &str) -> Result<Vec<AssetLocationView>, ScanError>;
}
