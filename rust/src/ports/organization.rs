use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationAction, OrganizationAlbumPage, OrganizationAlbumReceipt,
    OrganizationPreparedSelection, OrganizationSelection, OrganizationWriteReceipt,
};

pub(crate) trait OrganizationRepository {
    fn undo_organization_operation(
        &mut self,
        operation_id: &str,
        original_id: &str,
        expected_revision: u64,
    ) -> Result<OrganizationWriteReceipt, ScanError>;

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

    fn change_organization(
        &mut self,
        operation_id: &str,
        prepared: &OrganizationPreparedSelection,
        action: &OrganizationAction,
    ) -> Result<OrganizationWriteReceipt, ScanError>;

    fn load_organization_albums(
        &self,
        after: Option<&str>,
    ) -> Result<OrganizationAlbumPage, ScanError>;

    fn create_organization_album(
        &mut self,
        operation_id: &str,
        expected_revision: u64,
        name: &str,
    ) -> Result<OrganizationAlbumReceipt, ScanError>;

    fn rename_organization_album(
        &mut self,
        operation_id: &str,
        expected_revision: u64,
        album_id: &str,
        name: &str,
    ) -> Result<OrganizationAlbumReceipt, ScanError>;

    fn delete_organization_album(
        &mut self,
        operation_id: &str,
        expected_revision: u64,
        album_id: &str,
    ) -> Result<OrganizationWriteReceipt, ScanError>;
}
