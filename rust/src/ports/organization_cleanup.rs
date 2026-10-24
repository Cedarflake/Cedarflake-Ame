use crate::domain::ScanError;
use crate::domain::organization_cleanup::{
    OrganizationCleanupInspection, OrganizationCleanupProgress, OrganizationCleanupReceipt,
    OrganizationCleanupRequest,
};

pub(crate) trait OrganizationCleanupRepository {
    fn inspect_organization_cleanup(
        &mut self,
        cutoff_unix_ms: Option<i64>,
        progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
    ) -> Result<OrganizationCleanupInspection, ScanError>;

    fn apply_organization_cleanup(
        &mut self,
        request: &OrganizationCleanupRequest,
        progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
    ) -> Result<OrganizationCleanupReceipt, ScanError>;
}
