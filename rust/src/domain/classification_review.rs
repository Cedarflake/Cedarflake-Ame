use super::classification::{
    CategoryId, ClassificationRecordId, ClassificationReview, EffectiveClassification,
};
use super::classification_catalog::ClassificationWriteReceipt;
use super::{GalleryDuplicateMode, GalleryQuery};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClassificationReviewFilter {
    #[default]
    All,
    Pending,
    Confirmed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationReviewScope {
    pub gallery: GalleryQuery,
    pub category: Option<CategoryId>,
    pub review: ClassificationReviewFilter,
}

impl Default for ClassificationReviewScope {
    fn default() -> Self {
        Self {
            gallery: GalleryQuery {
                duplicate_mode: GalleryDuplicateMode::Merged,
                ..Default::default()
            },
            category: None,
            review: ClassificationReviewFilter::All,
        }
    }
}

impl ClassificationReviewScope {
    pub fn includes(&self, classification: &EffectiveClassification) -> bool {
        if self.category.is_some() && self.category != classification.category {
            return false;
        }
        match self.review {
            ClassificationReviewFilter::All => true,
            ClassificationReviewFilter::Pending => matches!(
                classification.review,
                ClassificationReview::NeedsReview
                    | ClassificationReview::Deferred
                    | ClassificationReview::Conflict
            ),
            ClassificationReviewFilter::Confirmed => {
                classification.review == ClassificationReview::Confirmed
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationReviewSession {
    pub id: ClassificationRecordId,
    pub position_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationReviewPosition {
    pub admission_id: u64,
    pub root_id: String,
    pub location_id: String,
    pub primary_missing: bool,
    pub primary_text: String,
    pub primary_number: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationReviewBookmark {
    pub record_id: String,
    pub scope: ClassificationReviewScope,
    pub position: ClassificationReviewPosition,
    pub after_current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationReviewReceipt {
    pub operation: ClassificationWriteReceipt,
    pub session: ClassificationReviewSession,
}
