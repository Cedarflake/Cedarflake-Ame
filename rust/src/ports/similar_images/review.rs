use crate::domain::ScanError;
use crate::domain::exact_duplicates::FingerprintCandidate;
use crate::domain::similar_images::SimilarContentKey;
use crate::domain::similar_images::review::{
    SimilarExclusionResult, SimilarGroupDecision, SimilarMemberExclusion, SimilarMemberSelection,
    SimilarReviewContext, SimilarReviewPage, SimilarReviewRange, SimilarUndoResult,
    SimilarityDegree,
};

pub(crate) struct SimilarMemberCandidate {
    pub preferred: bool,
    pub content: SimilarContentKey,
    pub candidate: FingerprintCandidate,
    pub location_count: u64,
    pub distance: u8,
}

pub(crate) struct SimilarMemberCandidates {
    pub members: Vec<SimilarMemberCandidate>,
    pub total_members: u64,
    pub next: Option<SimilarContentKey>,
    pub decision: SimilarGroupDecision,
}

pub(crate) trait SimilarReviewRepository {
    fn load_similar_groups(
        &self,
        degree: SimilarityDegree,
        ignored: bool,
        after: &str,
    ) -> Result<SimilarReviewPage, ScanError>;
    fn seek_similar_groups(
        &self,
        degree: SimilarityDegree,
        ignored: bool,
        index: u64,
    ) -> Result<SimilarReviewRange, ScanError>;
    fn load_similar_members(
        &self,
        group_id: &str,
        degree: SimilarityDegree,
        after: Option<&SimilarContentKey>,
    ) -> Result<SimilarMemberCandidates, ScanError>;
    fn prefer_similar_member(
        &mut self,
        context: &SimilarReviewContext,
        selection: &SimilarMemberSelection,
        preferred: bool,
    ) -> Result<SimilarGroupDecision, ScanError>;
    fn ignore_similar_group(
        &mut self,
        context: &SimilarReviewContext,
        ignored: bool,
    ) -> Result<SimilarGroupDecision, ScanError>;
    fn exclude_similar_member(
        &mut self,
        context: &SimilarReviewContext,
        selection: &SimilarMemberSelection,
        exclusion_id: &str,
    ) -> Result<SimilarExclusionResult, ScanError>;
    fn undo_similar_exclusion(
        &mut self,
        exclusion: &SimilarMemberExclusion,
    ) -> Result<SimilarUndoResult, ScanError>;
}
