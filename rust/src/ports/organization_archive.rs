use std::path::Path;

use crate::domain::ScanError;
use crate::domain::organization_archive::{
    OrganizationArchiveProgress, OrganizationArchiveReceipt,
};

pub(crate) trait OrganizationArchiveRepository {
    fn export_organization_archive(
        &self,
        destination: &Path,
        progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
    ) -> Result<OrganizationArchiveReceipt, ScanError>;
}
