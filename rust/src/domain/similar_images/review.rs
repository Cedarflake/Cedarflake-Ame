use super::SimilarContentKey;
use crate::domain::organization::OrganizationSelection;
use crate::domain::{AssetLocationView, DuplicateLocationAvailability};

pub(crate) const SIMILARITY_REVIEW_PAGE_SIZE: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimilarityDegree {
    Strict,
    Standard,
    Broad,
}

impl SimilarityDegree {
    pub(crate) fn distance(self) -> u8 {
        match self {
            Self::Strict => 2,
            Self::Standard => 6,
            Self::Broad => 10,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimilarGroupDecision {
    pub ignored: bool,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub struct SimilarReviewGroup {
    pub group_id: String,
    pub member_count: u64,
    pub representative: AssetLocationView,
    pub decision: SimilarGroupDecision,
}

#[derive(Clone, Debug)]
pub struct SimilarReviewPage {
    pub groups: Vec<SimilarReviewGroup>,
    pub total_groups: u64,
    pub next_group_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SimilarReviewRange {
    pub start_index: u64,
    pub page: SimilarReviewPage,
}

#[derive(Clone, Debug)]
pub struct SimilarReviewMember {
    pub preferred: bool,
    pub content: SimilarContentKey,
    pub representative: AssetLocationView,
    pub availability: DuplicateLocationAvailability,
    pub location_count: u64,
    pub distance: u8,
}

#[derive(Clone, Debug)]
pub struct SimilarReviewMembers {
    pub members: Vec<SimilarReviewMember>,
    pub total_members: u64,
    pub next: Option<SimilarContentKey>,
    pub decision: SimilarGroupDecision,
}

#[derive(Clone, Debug)]
pub struct SimilarReviewContext {
    pub group_id: String,
    pub degree: SimilarityDegree,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub struct SimilarMemberSelection {
    pub content: SimilarContentKey,
    pub source: OrganizationSelection,
}

#[derive(Clone, Debug)]
pub struct SimilarMemberExclusion {
    pub group_id: String,
    pub content: SimilarContentKey,
    pub exclusion_id: String,
}

#[derive(Clone, Debug)]
pub struct SimilarExclusionResult {
    pub decision: SimilarGroupDecision,
    pub undo: SimilarMemberExclusion,
}

#[derive(Clone, Debug)]
pub struct SimilarUndoResult {
    pub decision: SimilarGroupDecision,
    pub restored: bool,
}
