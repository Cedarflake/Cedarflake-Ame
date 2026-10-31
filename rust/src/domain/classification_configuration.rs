use super::classification::work::{ClassificationDispatchMode, ClassificationDispatchState};
use super::classification::{ClassificationAnalysisSetup, ClassificationRecordId};
use super::classification_catalog::ClassificationCatalogState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationConfigurationAction {
    Save,
    Enable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationControlAction {
    Pause,
    Continue,
    Disable,
    Stop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationCategoryInput {
    pub id: String,
    pub label: String,
    pub descriptions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationSettings {
    pub revision: u64,
    pub dispatch_revision: u64,
    pub mode: ClassificationDispatchMode,
    pub categories: Vec<ClassificationCategoryInput>,
    pub run_id: Option<String>,
    pub installation_selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationConfigurationSnapshot {
    pub catalog: ClassificationCatalogState,
    pub dispatch: ClassificationDispatchState,
}

pub struct ClassificationConfigurationChange {
    pub operation_id: ClassificationRecordId,
    pub expected_catalog_revision: u64,
    pub expected_dispatch_revision: u64,
    pub setup: ClassificationAnalysisSetup,
    pub action: ClassificationConfigurationAction,
}
