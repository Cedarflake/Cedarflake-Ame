use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationPreparedSelection, OrganizationSelection, OrganizationWriteReceipt,
};

pub fn prepare_organization_selection(
    selections: Vec<OrganizationSelection>,
) -> Result<OrganizationPreparedSelection, ScanError> {
    crate::application::organization::prepare_selection(selections)
}

pub fn set_organization_favorite(
    operation_id: String,
    prepared: OrganizationPreparedSelection,
    favorite: bool,
) -> Result<OrganizationWriteReceipt, ScanError> {
    crate::application::organization::set_favorite(operation_id, prepared, favorite)
}
