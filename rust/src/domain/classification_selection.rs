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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClassificationSelectionSummary {
    pub single_item: Option<ClassificationItemState>,
    pub group_count: u64,
    pub confirmable_count: u64,
    pub confirmed_count: u64,
    pub common_category_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationSelectionPhase {
    Collecting,
    Preparing,
    Ready,
    Saving,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationSelectionProgress {
    pub phase: ClassificationSelectionPhase,
    pub completed_groups: u64,
    pub total_groups: u64,
}
