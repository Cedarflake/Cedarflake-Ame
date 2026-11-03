use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationAction, ClassificationRecordId, ClassificationRun, PredictionEvidence, Taxonomy,
};
use crate::domain::classification_catalog::{
    ClassificationCatalogState, ClassificationWriteReceipt, PreparedClassificationSelection,
};
use crate::domain::organization::OrganizationSelection;

pub trait ClassificationRepository {
    fn load_classification_state(&self) -> Result<ClassificationCatalogState, ScanError>;

    fn configure_classification(
        &mut self,
        operation_id: &ClassificationRecordId,
        expected_revision: u64,
        taxonomy: &Taxonomy,
    ) -> Result<ClassificationWriteReceipt, ScanError>;

    fn begin_classification_run(
        &mut self,
        run: &ClassificationRun,
        refinement: Option<&Taxonomy>,
    ) -> Result<(), ScanError>;

    fn publish_classification_prediction(
        &mut self,
        source: &OrganizationSelection,
        evidence: &PredictionEvidence,
    ) -> Result<bool, ScanError>;

    fn prepare_classification_selection(
        &self,
        selections: &[OrganizationSelection],
    ) -> Result<PreparedClassificationSelection, ScanError>;

    fn change_classification(
        &mut self,
        operation_id: &ClassificationRecordId,
        prepared: &PreparedClassificationSelection,
        action: &ClassificationAction,
    ) -> Result<ClassificationWriteReceipt, ScanError>;

    fn undo_classification(
        &mut self,
        operation_id: &ClassificationRecordId,
        original_id: &ClassificationRecordId,
    ) -> Result<ClassificationWriteReceipt, ScanError>;
}
