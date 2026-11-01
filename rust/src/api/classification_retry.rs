use crate::domain::ScanError;
use crate::domain::classification::work::ClassificationRetryOutcome;
use crate::domain::organization::OrganizationSelection;

pub fn retry_failed_classification(
    expected_dispatch_revision: u64,
    target: OrganizationSelection,
) -> Result<ClassificationRetryOutcome, ScanError> {
    crate::application::classification_retry::retry(expected_dispatch_revision, target)
}
