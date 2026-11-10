use crate::domain::ScanError;
use crate::domain::file_operation_exports::FileOrganizationRule;
use crate::domain::file_operation_selection::{
    FileOperationCandidate, FileOperationSelectionSummary,
};
use crate::domain::organization::OrganizationSelection;

pub trait FileOperationSelectionRepository: Send {
    fn append_ids(&mut self, asset_ids: &[String]) -> Result<(), ScanError>;
    fn append_locations(&mut self, locations: &[OrganizationSelection]) -> Result<(), ScanError>;
    fn prepare(&mut self) -> Result<FileOperationSelectionSummary, ScanError>;
    fn freeze_rule(&mut self, rule: FileOrganizationRule) -> Result<(), ScanError>;
    fn page(&self, after_ordinal: Option<u64>) -> Result<Vec<FileOperationCandidate>, ScanError>;
}
