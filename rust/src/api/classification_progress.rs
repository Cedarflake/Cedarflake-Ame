use crate::application::classification_progress;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification_catalog::ClassificationWriteReceipt;
use crate::domain::classification_review::ClassificationReviewReceipt;
use crate::domain::classification_review::{
    ClassificationContinuation, ClassificationReviewSession,
};
use crate::domain::organization::OrganizationSelection;
use crate::domain::{GalleryQuery, ScanError};

pub struct ClassificationReviewPosition {
    pub session_id: String,
    pub sequence: u64,
}

pub struct ClassificationProgressReceipt {
    pub operation: ClassificationWriteReceipt,
    pub position: ClassificationReviewPosition,
}

impl From<ClassificationReviewSession> for ClassificationReviewPosition {
    fn from(value: ClassificationReviewSession) -> Self {
        Self {
            session_id: value.id.as_str().into(),
            sequence: value.position_sequence,
        }
    }
}

impl TryFrom<ClassificationReviewPosition> for ClassificationReviewSession {
    type Error = ScanError;
    fn try_from(value: ClassificationReviewPosition) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ClassificationRecordId::new(value.session_id).map_err(|error| {
                ScanError::new("classification_review_changed", error.to_string())
            })?,
            position_sequence: value.sequence,
        })
    }
}

impl From<ClassificationReviewReceipt> for ClassificationProgressReceipt {
    fn from(value: ClassificationReviewReceipt) -> Self {
        Self {
            operation: value.operation,
            position: value.session.into(),
        }
    }
}

pub fn load_classification_continuation() -> Result<ClassificationContinuation, ScanError> {
    classification_progress::continuation()
}

pub fn begin_classification_review(
    query: GalleryQuery,
) -> Result<ClassificationReviewPosition, ScanError> {
    classification_progress::begin(query).map(Into::into)
}

pub fn checkpoint_classification_review(
    position: ClassificationReviewPosition,
    target: OrganizationSelection,
) -> Result<(), ScanError> {
    classification_progress::checkpoint(&position.try_into()?, &target)
}

pub fn end_classification_review(session_id: String) -> Result<(), ScanError> {
    let id = ClassificationRecordId::new(session_id)
        .map_err(|error| ScanError::new("classification_review_changed", error.to_string()))?;
    classification_progress::end(&id)
}
