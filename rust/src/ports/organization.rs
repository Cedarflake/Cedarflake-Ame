use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationPreparedSelection, OrganizationSelection, OrganizationWriteReceipt,
};

pub(crate) trait OrganizationRepository {
    fn prepare_organization_selection(
        &self,
        selections: &[OrganizationSelection],
    ) -> Result<OrganizationPreparedSelection, ScanError>;

    fn set_organization_favorite(
        &mut self,
        operation_id: &str,
        prepared: &OrganizationPreparedSelection,
        favorite: bool,
    ) -> Result<OrganizationWriteReceipt, ScanError>;
}
