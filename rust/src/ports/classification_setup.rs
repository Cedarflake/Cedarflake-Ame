use crate::domain::ScanError;
use crate::domain::classification::work::ClassificationDispatchState;
use crate::domain::classification::{
    ClassificationAnalysisSetup, ClassificationRecordId, ClassificationSetupActivation,
};

pub trait ClassificationSetupRepository {
    fn load_classification_setup(
        &self,
        run_id: &ClassificationRecordId,
    ) -> Result<Option<ClassificationAnalysisSetup>, ScanError>;

    fn activate_classification_setup(
        &mut self,
        expected_catalog_revision: u64,
        expected_dispatch_revision: u64,
        setup: &ClassificationAnalysisSetup,
        activation: ClassificationSetupActivation,
    ) -> Result<ClassificationDispatchState, ScanError>;
}
