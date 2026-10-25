use crate::domain::organization::{
    OrganizationAction, OrganizationPreparedSelection, OrganizationSelection, validate_identifier,
    validate_selections,
};
use crate::domain::organization_review::{
    OrganizationContinuation, OrganizationReviewSession, OrganizationReviewWriteReceipt,
};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::organization_review::OrganizationReviewRepository;

use super::super::{SqliteCatalog, database_error};

mod continuation;
mod operations;
pub(in super::super) mod ordering;
mod position;
pub(in super::super) mod scope;

pub(super) use scope::{forget_source, transfer_source};

impl OrganizationReviewRepository for SqliteCatalog {
    fn load_organization_admission_boundary(&self) -> Result<u64, ScanError> {
        let boundary: i64 = self
            .connection
            .query_row(
                "SELECT COALESCE(MAX(value), 0) FROM (
               SELECT seq AS value FROM sqlite_sequence WHERE name = 'organization_admissions'
               UNION ALL SELECT MAX(admission_id) FROM organization_recovery_admissions)",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        super::super::sqlite_unsigned(boundary, "new-arrival boundary")
    }

    fn begin_organization_review(
        &mut self,
        query: &GalleryQuery,
    ) -> Result<OrganizationReviewSession, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        scope::validate(&transaction, query)?;
        scope::save_active(&transaction, query)?;
        let session_id: String = transaction
            .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO organization_review_session(singleton, session_id, position_sequence)
                 VALUES (1, ?1, 0)
                 ON CONFLICT(singleton) DO UPDATE SET session_id = excluded.session_id,
                   position_sequence = 0, checkpoint_digest = NULL",
                [&session_id],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(OrganizationReviewSession {
            session_id,
            position_sequence: 0,
        })
    }

    fn end_organization_review(&mut self, session_id: &str) -> Result<(), ScanError> {
        validate_identifier(session_id)?;
        let transaction = self.begin_user_interactive_write()?;
        transaction
            .execute(
                "DELETE FROM organization_review_scopes WHERE slot = 'active'
                 AND EXISTS(SELECT 1 FROM organization_review_session WHERE session_id = ?1)",
                [session_id],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    fn checkpoint_organization_review(
        &mut self,
        requested: &OrganizationReviewSession,
        selection: &OrganizationSelection,
    ) -> Result<OrganizationReviewSession, ScanError> {
        requested.validate()?;
        validate_selections(std::slice::from_ref(selection))?;
        let transaction = self.begin_user_interactive_write()?;
        position::save(&transaction, requested, selection)?;
        transaction.commit().map_err(database_error)?;
        Ok(requested.clone())
    }

    fn load_organization_continuation(
        &self,
        query: Option<&GalleryQuery>,
    ) -> Result<OrganizationContinuation, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let result = continuation::load(&transaction, query)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn change_current_organization_review(
        &mut self,
        operation_id: &str,
        session: &OrganizationReviewSession,
        prepared: &OrganizationPreparedSelection,
        action: &OrganizationAction,
    ) -> Result<OrganizationReviewWriteReceipt, ScanError> {
        validate_identifier(operation_id)?;
        session.validate()?;
        super::authority::validate_prepared(prepared)?;
        action.validate()?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = operations::save(&transaction, operation_id, session, prepared, action)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }
}
