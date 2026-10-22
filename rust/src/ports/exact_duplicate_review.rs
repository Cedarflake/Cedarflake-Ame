use crate::domain::exact_duplicate_review::{
    ExactDuplicateDecision, ExactDuplicatePreference, ExactDuplicateReviewKey,
    ExactDuplicateReviewPage, ExactDuplicateReviewRange,
};
use crate::domain::exact_duplicates::FingerprintCandidate;
use crate::domain::{ExactDuplicateGroupCursor, ScanError};

pub(crate) struct ExactDuplicateReviewMembers {
    pub candidates: Vec<FingerprintCandidate>,
    pub decision: ExactDuplicateDecision,
}

pub(crate) trait ExactDuplicateReviewRepository {
    fn load_review_members(
        &self,
        key: &ExactDuplicateReviewKey,
        after: &str,
    ) -> Result<ExactDuplicateReviewMembers, ScanError>;

    fn load_review_groups(
        &self,
        after: Option<&ExactDuplicateGroupCursor>,
        ignored: bool,
    ) -> Result<ExactDuplicateReviewPage, ScanError>;

    fn seek_review_groups(
        &self,
        index: u64,
        ignored: bool,
    ) -> Result<ExactDuplicateReviewRange, ScanError>;

    fn set_duplicate_preference(
        &mut self,
        key: &ExactDuplicateReviewKey,
        preferred: Option<&ExactDuplicatePreference>,
    ) -> Result<ExactDuplicateDecision, ScanError>;

    fn set_duplicate_ignored(
        &mut self,
        key: &ExactDuplicateReviewKey,
        ignored: bool,
    ) -> Result<ExactDuplicateDecision, ScanError>;
}
