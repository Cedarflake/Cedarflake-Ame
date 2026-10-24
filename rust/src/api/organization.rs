use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationAction, OrganizationAlbumPage, OrganizationAlbumReceipt, OrganizationInspection,
    OrganizationPreparedSelection, OrganizationSelection, OrganizationWriteReceipt,
};

pub fn inspect_organization_selection(
    selections: Vec<OrganizationSelection>,
) -> Result<OrganizationInspection, ScanError> {
    crate::application::organization::inspect_selection(selections)
}

pub fn undo_organization_operation(
    operation_id: String,
    original_id: String,
    expected_revision: u64,
) -> Result<OrganizationWriteReceipt, ScanError> {
    crate::application::organization::undo(operation_id, original_id, expected_revision)
}

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

pub fn change_organization(
    operation_id: String,
    prepared: OrganizationPreparedSelection,
    action: OrganizationAction,
) -> Result<OrganizationWriteReceipt, ScanError> {
    crate::application::organization::change(operation_id, prepared, action)
}

pub fn load_organization_albums(after: Option<String>) -> Result<OrganizationAlbumPage, ScanError> {
    crate::application::organization::load_albums(after)
}

pub fn create_organization_album(
    operation_id: String,
    expected_revision: u64,
    name: String,
) -> Result<OrganizationAlbumReceipt, ScanError> {
    crate::application::organization::create_album(operation_id, expected_revision, name)
}

pub fn rename_organization_album(
    operation_id: String,
    expected_revision: u64,
    album_id: String,
    name: String,
) -> Result<OrganizationAlbumReceipt, ScanError> {
    crate::application::organization::rename_album(operation_id, expected_revision, album_id, name)
}

pub fn delete_organization_album(
    operation_id: String,
    expected_revision: u64,
    album_id: String,
) -> Result<OrganizationWriteReceipt, ScanError> {
    crate::application::organization::delete_album(operation_id, expected_revision, album_id)
}
