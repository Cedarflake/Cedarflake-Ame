use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::work::{
    ClassificationRecoveryCursor, ClassificationRecoveryPage, ClassificationRecoveryRoot,
};
use crate::domain::organization::OrganizationSelection;

pub trait ClassificationAvailabilityRepository {
    fn refresh_classification_sources(
        &mut self,
        session: &ClassificationRecordId,
        sources: &[OrganizationSelection],
        now_unix_ms: u64,
    ) -> Result<u64, ScanError>;

    fn recover_classification_root(
        &mut self,
        session: &ClassificationRecordId,
        root: &ClassificationRecoveryRoot,
        cursor: Option<&ClassificationRecoveryCursor>,
        now_unix_ms: u64,
    ) -> Result<ClassificationRecoveryPage, ScanError>;
}
