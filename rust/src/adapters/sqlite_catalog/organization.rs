use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationAction, OrganizationAlbum, OrganizationAlbumPage, OrganizationAlbumReceipt,
    OrganizationInspection, OrganizationPreparedSelection, OrganizationSelection,
    OrganizationWriteReceipt, normalize_album_name, validate_identifier, validate_selections,
};
use crate::ports::organization::OrganizationRepository;

use super::{SqliteCatalog, database_error};

pub(super) mod admission;
mod album_membership;
mod albums;
pub(super) mod association_lifetime;
mod authority;
mod batch;
pub(crate) use batch::SqliteOrganizationBatch;
mod changes;
mod cleanup;
mod conflicts;
mod favorites;
mod inspection;
pub(super) mod lifecycle;
mod native_recovery;
mod operations;
pub(super) mod publication;
pub(super) mod reconciliation;
pub(super) mod records;
pub(super) mod recovery;
mod restoration_schema;
mod review;
mod selection;
mod undo;

impl OrganizationRepository for SqliteCatalog {
    fn find_organization_album(
        &self,
        album_id: &str,
    ) -> Result<Option<OrganizationAlbum>, ScanError> {
        validate_identifier(album_id)?;
        albums::find(&self.connection, album_id)
    }

    fn inspect_organization_selection(
        &self,
        selections: &[OrganizationSelection],
    ) -> Result<OrganizationInspection, ScanError> {
        validate_selections(selections)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let result = inspection::load(&transaction, selections)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn undo_organization_operation(
        &mut self,
        operation_id: &str,
        original_id: &str,
        expected_revision: u64,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        validate_identifier(operation_id)?;
        validate_identifier(original_id)?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = undo::apply(&transaction, operation_id, original_id, expected_revision)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }

    fn prepare_organization_selection(
        &self,
        selections: &[OrganizationSelection],
    ) -> Result<OrganizationPreparedSelection, ScanError> {
        validate_selections(selections)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let prepared = selection::prepare(&transaction, selections)?;
        transaction.commit().map_err(database_error)?;
        Ok(prepared)
    }

    fn set_organization_favorite(
        &mut self,
        operation_id: &str,
        prepared: &OrganizationPreparedSelection,
        favorite: bool,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        validate_identifier(operation_id)?;
        authority::validate_prepared(prepared)?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = favorites::save(&transaction, operation_id, prepared, favorite)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }

    fn change_organization(
        &mut self,
        operation_id: &str,
        prepared: &OrganizationPreparedSelection,
        action: &OrganizationAction,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        validate_identifier(operation_id)?;
        authority::validate_prepared(prepared)?;
        action.validate()?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = changes::save(&transaction, operation_id, prepared, action)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }

    fn load_organization_albums(
        &self,
        after: Option<&str>,
    ) -> Result<OrganizationAlbumPage, ScanError> {
        if let Some(after) = after {
            validate_identifier(after)?;
        }
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let page = albums::load(&transaction, after)?;
        transaction.commit().map_err(database_error)?;
        Ok(page)
    }

    fn create_organization_album(
        &mut self,
        operation_id: &str,
        expected_revision: u64,
        name: &str,
    ) -> Result<OrganizationAlbumReceipt, ScanError> {
        validate_identifier(operation_id)?;
        let name = normalize_album_name(name)?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = albums::create(&transaction, operation_id, expected_revision, &name)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }

    fn rename_organization_album(
        &mut self,
        operation_id: &str,
        expected_revision: u64,
        album_id: &str,
        name: &str,
    ) -> Result<OrganizationAlbumReceipt, ScanError> {
        validate_identifier(operation_id)?;
        validate_identifier(album_id)?;
        let name = normalize_album_name(name)?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = albums::rename(
            &transaction,
            operation_id,
            expected_revision,
            album_id,
            &name,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }

    fn delete_organization_album(
        &mut self,
        operation_id: &str,
        expected_revision: u64,
        album_id: &str,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        validate_identifier(operation_id)?;
        validate_identifier(album_id)?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = albums::retire(&transaction, operation_id, expected_revision, album_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests;
