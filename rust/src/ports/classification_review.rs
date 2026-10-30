use crate::domain::ScanError;
use crate::domain::classification::{ClassificationAction, ClassificationRecordId};
use crate::domain::classification_catalog::PreparedClassificationSelection;
use crate::domain::classification_review::{
    ClassificationReviewBookmark, ClassificationReviewReceipt, ClassificationReviewScope,
    ClassificationReviewSession,
};
use crate::domain::organization::OrganizationSelection;

pub trait ClassificationReviewRepository {
    fn begin_classification_review(
        &mut self,
        scope: &ClassificationReviewScope,
    ) -> Result<ClassificationReviewSession, ScanError>;
    fn end_classification_review(
        &mut self,
        session_id: &ClassificationRecordId,
    ) -> Result<(), ScanError>;
    fn checkpoint_classification_review(
        &mut self,
        session: &ClassificationReviewSession,
        selection: &OrganizationSelection,
    ) -> Result<(), ScanError>;
    fn load_classification_bookmark(
        &self,
    ) -> Result<Option<ClassificationReviewBookmark>, ScanError>;
    fn change_current_classification_review(
        &mut self,
        operation_id: &ClassificationRecordId,
        session: &ClassificationReviewSession,
        prepared: &PreparedClassificationSelection,
        action: &ClassificationAction,
    ) -> Result<ClassificationReviewReceipt, ScanError>;
}
