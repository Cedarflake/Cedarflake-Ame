use crate::domain::ScanError;
use crate::domain::classification::work::ClassificationRetryOutcome;
use crate::domain::organization::OrganizationSelection;
use crate::ports::classification_work::ClassificationRetryRepository;

use super::classification_dispatch::{ClassificationDispatchClock, MonotonicDispatchClock};

pub(crate) fn retry(
    expected_dispatch_revision: u64,
    target: OrganizationSelection,
) -> Result<ClassificationRetryOutcome, ScanError> {
    super::organization::open_catalog()?.retry_failed_classification(
        expected_dispatch_revision,
        &target,
        MonotonicDispatchClock::default().unix_ms(),
    )
}
