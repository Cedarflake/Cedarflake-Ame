use super::{
    AssetLocationView, DuplicateLocationAvailability, ExactDuplicateGroup,
    ExactDuplicateGroupCursor,
};

#[derive(Clone, Debug)]
pub struct ExactDuplicateReviewKey {
    pub digest: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ExactDuplicateDecision {
    pub preferred_location_id: Option<String>,
    pub preferred_asset_id: Option<String>,
    pub ignored: bool,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateReviewGroup {
    pub group: ExactDuplicateGroup,
    pub representative: AssetLocationView,
    pub decision: ExactDuplicateDecision,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateReviewPage {
    pub groups: Vec<ExactDuplicateReviewGroup>,
    pub total_groups: u64,
    pub next: Option<ExactDuplicateGroupCursor>,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicatePreference {
    pub location_id: String,
    pub asset_id: String,
    pub source_generation: u64,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateReviewLocation {
    pub asset: AssetLocationView,
    pub availability: DuplicateLocationAvailability,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateReviewLocations {
    pub locations: Vec<ExactDuplicateReviewLocation>,
    pub next_location_id: Option<String>,
    pub decision: ExactDuplicateDecision,
}
