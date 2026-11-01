#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationItemReview {
    Unclassified,
    Suggested,
    NeedsReview,
    Deferred,
    Confirmed,
    Conflict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationReviewReason {
    Uncertain,
    Uncalibrated,
    Sampling,
    Deferred,
    Conflict,
    AnalysisFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationAnnotation {
    pub location_id: String,
    pub reason: ClassificationReviewReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationItemState {
    pub asset_id: String,
    pub category_id: Option<String>,
    pub review: ClassificationItemReview,
    pub can_confirm: bool,
    pub review_reason: Option<ClassificationReviewReason>,
}
