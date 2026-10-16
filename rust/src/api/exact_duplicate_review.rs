use crate::domain::exact_duplicate_review::{
    ExactDuplicateDecision, ExactDuplicatePreference, ExactDuplicateReviewKey,
    ExactDuplicateReviewLocations, ExactDuplicateReviewPage,
};
use crate::domain::{ExactDuplicateGroupCursor, ScanError};

pub fn load_exact_duplicate_review_groups(
    after: Option<ExactDuplicateGroupCursor>,
    ignored: bool,
) -> Result<ExactDuplicateReviewPage, ScanError> {
    crate::application::exact_duplicate_review::load_groups(after, ignored)
}

pub fn load_exact_duplicate_review_locations(
    key: ExactDuplicateReviewKey,
    after: Option<String>,
) -> Result<ExactDuplicateReviewLocations, ScanError> {
    crate::application::exact_duplicate_review::load_locations(key, after)
}

pub fn set_exact_duplicate_preference(
    key: ExactDuplicateReviewKey,
    preferred: Option<ExactDuplicatePreference>,
) -> Result<ExactDuplicateDecision, ScanError> {
    crate::application::exact_duplicate_review::prefer(key, preferred)
}

pub fn set_exact_duplicate_ignored(
    key: ExactDuplicateReviewKey,
    ignored: bool,
) -> Result<ExactDuplicateDecision, ScanError> {
    crate::application::exact_duplicate_review::ignore(key, ignored)
}
