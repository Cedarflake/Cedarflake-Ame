use crate::application::similar_images::review;
use crate::domain::ScanError;
use crate::domain::similar_images::SimilarContentKey;
use crate::domain::similar_images::review::{
    SimilarExclusionResult, SimilarGroupDecision, SimilarMemberExclusion, SimilarMemberSelection,
    SimilarReviewContext, SimilarReviewMembers, SimilarReviewPage, SimilarReviewRange,
    SimilarUndoResult, SimilarityDegree,
};

pub fn load_similar_groups(
    degree: SimilarityDegree,
    ignored: bool,
    after: Option<String>,
) -> Result<SimilarReviewPage, ScanError> {
    review::groups(degree, ignored, after)
}

pub fn seek_similar_groups(
    degree: SimilarityDegree,
    ignored: bool,
    index: u64,
) -> Result<SimilarReviewRange, ScanError> {
    review::seek(degree, ignored, index)
}

pub fn load_similar_members(
    group_id: String,
    degree: SimilarityDegree,
    after: Option<SimilarContentKey>,
) -> Result<SimilarReviewMembers, ScanError> {
    review::members(group_id, degree, after)
}

pub fn set_similar_preference(
    context: SimilarReviewContext,
    selection: SimilarMemberSelection,
    preferred: bool,
) -> Result<SimilarGroupDecision, ScanError> {
    review::prefer(context, selection, preferred)
}

pub fn set_similar_ignored(
    context: SimilarReviewContext,
    ignored: bool,
) -> Result<SimilarGroupDecision, ScanError> {
    review::ignore(context, ignored)
}

pub fn exclude_similar_member(
    context: SimilarReviewContext,
    selection: SimilarMemberSelection,
    exclusion_id: String,
) -> Result<SimilarExclusionResult, ScanError> {
    review::exclude(context, selection, exclusion_id)
}

pub fn undo_similar_exclusion(
    exclusion: SimilarMemberExclusion,
) -> Result<SimilarUndoResult, ScanError> {
    review::undo(exclusion)
}
