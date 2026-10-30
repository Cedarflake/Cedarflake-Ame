use rusqlite::params;

use crate::domain::ScanError;
use crate::domain::classification::{ClassificationAction, ClassificationRecordId};
use crate::domain::classification_catalog::PreparedClassificationSelection;
use crate::domain::classification_review::{
    ClassificationReviewBookmark, ClassificationReviewReceipt, ClassificationReviewScope,
    ClassificationReviewSession,
};
use crate::domain::organization::{OrganizationSelection, validate_selections};
use crate::ports::classification_review::ClassificationReviewRepository;

use super::super::{SqliteCatalog, database_error};
use super::codec;

mod commands;
pub(in crate::adapters::sqlite_catalog) mod lifecycle;
pub(in crate::adapters::sqlite_catalog) mod position;
pub(in crate::adapters::sqlite_catalog) mod scope;

impl ClassificationReviewRepository for SqliteCatalog {
    fn begin_classification_review(
        &mut self,
        requested: &ClassificationReviewScope,
    ) -> Result<ClassificationReviewSession, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        scope::admit(&transaction, requested)?;
        let id: String = transaction
            .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
            .map_err(database_error)?;
        transaction.execute(
            "INSERT INTO classification_review_session(singleton, session_id, position_sequence, scope)
             VALUES (1, ?1, 0, ?2) ON CONFLICT(singleton) DO UPDATE SET session_id = excluded.session_id,
               position_sequence = 0, scope = excluded.scope, checkpoint_digest = NULL",
            params![id, scope::encode(requested)?],
        ).map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(ClassificationReviewSession {
            id: codec::identifier(id)?,
            position_sequence: 0,
        })
    }

    fn end_classification_review(
        &mut self,
        session_id: &ClassificationRecordId,
    ) -> Result<(), ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        transaction
            .execute(
                "DELETE FROM classification_review_session WHERE session_id = ?1",
                [session_id.as_str()],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    fn checkpoint_classification_review(
        &mut self,
        session: &ClassificationReviewSession,
        selection: &OrganizationSelection,
    ) -> Result<(), ScanError> {
        validate_selections(std::slice::from_ref(selection))?;
        let transaction = self.begin_user_interactive_write()?;
        position::save(&transaction, session, selection)?;
        transaction.commit().map_err(database_error)
    }

    fn load_classification_bookmark(
        &self,
    ) -> Result<Option<ClassificationReviewBookmark>, ScanError> {
        position::load(&self.connection)
    }

    fn change_current_classification_review(
        &mut self,
        operation_id: &ClassificationRecordId,
        session: &ClassificationReviewSession,
        prepared: &PreparedClassificationSelection,
        action: &ClassificationAction,
    ) -> Result<ClassificationReviewReceipt, ScanError> {
        validate_selections(&prepared.selections)?;
        let transaction = self.begin_user_interactive_write()?;
        let receipt = commands::apply(&transaction, operation_id, session, prepared, action)?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }
}

fn stale() -> ScanError {
    ScanError::new(
        "classification_review_changed",
        "The classification review scope or position changed; reload the current work",
    )
}
