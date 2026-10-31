use super::classification::{ClassificationObservation, ClassificationRevision, Taxonomy};
use super::organization::OrganizationSelection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationCatalogState {
    pub revision: u64,
    pub taxonomy: Option<Taxonomy>,
    pub configuration: Option<ClassificationRevision>,
    pub active_run_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedClassificationSelection {
    pub selections: Vec<OrganizationSelection>,
    pub groups: Vec<ClassificationObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationWriteReceipt {
    pub operation_id: String,
    pub revision: u64,
    pub changed_groups: u64,
}
