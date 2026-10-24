use std::path::Path;

use crate::domain::ScanError;
use crate::domain::organization_archive::{
    OrganizationArchiveInspection, OrganizationArchiveProgress, OrganizationArchiveReceipt,
};

pub(crate) trait OrganizationArchiveRepository {
    fn inspect_organization_archive(
        &self,
        archive: &Path,
        progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
    ) -> Result<OrganizationArchiveInspection, ScanError>;

    fn export_organization_archive(
        &self,
        destination: &Path,
        progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
    ) -> Result<OrganizationArchiveReceipt, ScanError>;
}
