use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationAction, ClassificationRecordId, ClassificationRun, PredictionEvidence, Taxonomy,
};
use crate::domain::classification_catalog::{
    ClassificationCatalogState, ClassificationWriteReceipt, PreparedClassificationSelection,
};
use crate::domain::organization::{OrganizationSelection, validate_selections};
use crate::ports::classification::ClassificationRepository;

use super::{SqliteCatalog, database_error};

pub(super) mod codec;
mod configuration;
mod observations;
mod operations;
mod predictions;
mod request;
pub(super) mod review;
mod undo;
pub(super) mod validation;
mod work;
mod writes;

impl ClassificationRepository for SqliteCatalog {
    fn load_classification_state(&self) -> Result<ClassificationCatalogState, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let state = configuration::load(&transaction)?;
        transaction.commit().map_err(database_error)?;
        Ok(state)
    }

    fn configure_classification(
        &mut self,
        operation_id: &ClassificationRecordId,
        expected_revision: u64,
        taxonomy: &Taxonomy,
    ) -> Result<ClassificationWriteReceipt, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let receipt =
            configuration::configure(&transaction, operation_id, expected_revision, taxonomy)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }

    fn begin_classification_run(
        &mut self,
        run: &ClassificationRun,
        refinement: Option<&Taxonomy>,
    ) -> Result<(), ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        predictions::begin(&transaction, run, refinement)?;
        transaction.commit().map_err(database_error)
    }

    fn publish_classification_prediction(
        &mut self,
        source: &OrganizationSelection,
        evidence: &PredictionEvidence,
    ) -> Result<bool, ScanError> {
        validate_selections(std::slice::from_ref(source))?;
        let transaction = self.begin_user_interactive_write()?;
        let published = predictions::publish(&transaction, source, evidence)?;
        transaction.commit().map_err(database_error)?;
        Ok(published)
    }

    fn prepare_classification_selection(
        &self,
        selections: &[OrganizationSelection],
    ) -> Result<PreparedClassificationSelection, ScanError> {
        validate_selections(selections)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let result = observations::prepare(&transaction, selections)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn change_classification(
        &mut self,
        operation_id: &ClassificationRecordId,
        prepared: &PreparedClassificationSelection,
        action: &ClassificationAction,
    ) -> Result<ClassificationWriteReceipt, ScanError> {
        validate_selections(&prepared.selections)?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = writes::apply(&transaction, operation_id, prepared, action)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }

    fn undo_classification(
        &mut self,
        operation_id: &ClassificationRecordId,
        original_id: &ClassificationRecordId,
    ) -> Result<ClassificationWriteReceipt, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let receipt = undo::apply(&transaction, operation_id, original_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }
}

fn invalid_record() -> ScanError {
    ScanError::new(
        "classification_record_invalid",
        "The saved classification record is incomplete or incompatible",
    )
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_changed",
        "Classification changed; refresh the current selection before saving",
    )
}

#[cfg(test)]
mod tests;
