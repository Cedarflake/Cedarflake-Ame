use crate::domain::organization::{
    OrganizationPreparedSelection, OrganizationSelection, OrganizationWriteReceipt,
    validate_identifier, validate_selections,
};
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::organization::OrganizationRepository;

fn open_catalog() -> Result<crate::adapters::SqliteCatalog, ScanError> {
    let storage = super::storage_paths()?;
    super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Live)
}

pub(crate) fn prepare_selection(
    selections: Vec<OrganizationSelection>,
) -> Result<OrganizationPreparedSelection, ScanError> {
    validate_selections(&selections)?;
    open_catalog()?.prepare_organization_selection(&selections)
}

pub(crate) fn set_favorite(
    operation_id: String,
    prepared: OrganizationPreparedSelection,
    favorite: bool,
) -> Result<OrganizationWriteReceipt, ScanError> {
    validate_identifier(&operation_id)?;
    validate_selections(&prepared.selections)?;
    open_catalog()?.set_organization_favorite(&operation_id, &prepared, favorite)
}
