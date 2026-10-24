use super::organization::{OrganizationAction, OrganizationAlbum};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationBatchAction {
    Favorite { favorite: bool },
    Organize { action: OrganizationAction },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationBatchSummary {
    pub catalog_revision: u64,
    pub organization_revision: u64,
    pub group_count: u64,
    pub favorite_count: u64,
    pub assigned_count: u64,
    pub conflicting_count: u64,
    pub common_album: Option<OrganizationAlbum>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationBatchPhase {
    Collecting,
    Preparing,
    Ready,
    Saving,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationBatchProgress {
    pub phase: OrganizationBatchPhase,
    pub completed_groups: u64,
    pub total_groups: u64,
}
