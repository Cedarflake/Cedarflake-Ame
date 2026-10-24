use crate::domain::organization::{
    OrganizationAction, OrganizationAlbum, OrganizationAlbumPage, OrganizationAlbumReceipt,
    OrganizationInspection, OrganizationPreparedSelection, OrganizationSelection,
    OrganizationWriteReceipt, normalize_album_name, validate_identifier, validate_selections,
};
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::organization::OrganizationRepository;

pub(super) fn open_catalog() -> Result<crate::adapters::SqliteCatalog, ScanError> {
    let storage = super::storage_paths()?;
    super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Live)
}

pub(crate) fn inspect_selection(
    selections: Vec<OrganizationSelection>,
) -> Result<OrganizationInspection, ScanError> {
    validate_selections(&selections)?;
    open_catalog()?.inspect_organization_selection(&selections)
}

pub(crate) fn undo(
    operation_id: String,
    original_id: String,
    expected_revision: u64,
) -> Result<OrganizationWriteReceipt, ScanError> {
    validate_identifier(&operation_id)?;
    validate_identifier(&original_id)?;
    open_catalog()?.undo_organization_operation(&operation_id, &original_id, expected_revision)
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

pub(crate) fn change(
    operation_id: String,
    prepared: OrganizationPreparedSelection,
    action: OrganizationAction,
) -> Result<OrganizationWriteReceipt, ScanError> {
    validate_identifier(&operation_id)?;
    validate_selections(&prepared.selections)?;
    action.validate()?;
    open_catalog()?.change_organization(&operation_id, &prepared, &action)
}

pub(crate) fn load_albums(after: Option<String>) -> Result<OrganizationAlbumPage, ScanError> {
    if let Some(after) = &after {
        validate_identifier(after)?;
    }
    open_catalog()?.load_organization_albums(after.as_deref())
}

pub(crate) fn find_album(album_id: String) -> Result<Option<OrganizationAlbum>, ScanError> {
    validate_identifier(&album_id)?;
    open_catalog()?.find_organization_album(&album_id)
}

pub(crate) fn create_album(
    operation_id: String,
    expected_revision: u64,
    name: String,
) -> Result<OrganizationAlbumReceipt, ScanError> {
    validate_identifier(&operation_id)?;
    let name = normalize_album_name(&name)?;
    open_catalog()?.create_organization_album(&operation_id, expected_revision, &name)
}

pub(crate) fn rename_album(
    operation_id: String,
    expected_revision: u64,
    album_id: String,
    name: String,
) -> Result<OrganizationAlbumReceipt, ScanError> {
    validate_identifier(&operation_id)?;
    validate_identifier(&album_id)?;
    let name = normalize_album_name(&name)?;
    open_catalog()?.rename_organization_album(&operation_id, expected_revision, &album_id, &name)
}

pub(crate) fn delete_album(
    operation_id: String,
    expected_revision: u64,
    album_id: String,
) -> Result<OrganizationWriteReceipt, ScanError> {
    validate_identifier(&operation_id)?;
    validate_identifier(&album_id)?;
    open_catalog()?.delete_organization_album(&operation_id, expected_revision, &album_id)
}
