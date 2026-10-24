use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationPreparedSelection, OrganizationSelection, OrganizationWriteReceipt,
    validate_identifier, validate_selections,
};
use crate::ports::organization::OrganizationRepository;

use super::{SqliteCatalog, database_error};

mod authority;
mod favorites;
mod records;
mod selection;

impl OrganizationRepository for SqliteCatalog {
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
}

#[cfg(test)]
mod tests;
