use super::{CatalogSnapshot, GalleryTimeline};

#[derive(Clone, Debug)]
pub struct GalleryQueryAnchor {
    pub requested_location_id: String,
    pub asset_id: Option<String>,
    pub fallback_ordinal: u64,
}

#[derive(Clone, Debug)]
pub struct GalleryQuerySnapshot {
    pub snapshot: CatalogSnapshot,
    pub timeline: GalleryTimeline,
    pub organization_counts: Option<super::organization_queue::OrganizationCounts>,
}
