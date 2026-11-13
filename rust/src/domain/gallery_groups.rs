use super::AssetLocationView;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GalleryGroupFilter {
    #[default]
    Unrestricted,
    Exact,
    Similar,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GalleryGroupKey {
    Exact { digest: String, byte_length: u64 },
    Similar { group_id: String },
}

impl GalleryGroupKey {
    pub(crate) fn entry_id(&self) -> String {
        match self {
            Self::Exact {
                digest,
                byte_length,
            } => format!("exact:{digest}:{byte_length}"),
            Self::Similar { group_id } => format!("similar:{group_id}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryGroup {
    pub key: GalleryGroupKey,
    pub member_count: u64,
}

#[derive(Clone, Debug)]
pub struct GalleryGroupMember {
    pub source: AssetLocationView,
    pub content: super::similar_images::SimilarContentKey,
    pub preferred: bool,
    pub availability: super::exact_duplicates::DuplicateLocationAvailability,
}

#[derive(Clone, Debug)]
pub struct GalleryGroupMemberPage {
    pub members: Vec<GalleryGroupMember>,
    pub total: u64,
    pub pending: u64,
    pub next_location_id: Option<String>,
    pub decision_revision: u64,
}

/// A gallery card has its own identity; overlapping groups may share a cover source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryEntry {
    pub source: AssetLocationView,
    pub group: Option<GalleryGroup>,
}

impl GalleryEntry {
    pub(crate) fn entry_id(&self) -> String {
        self.group.as_ref().map_or_else(
            || self.source.location_id.clone(),
            |group| group.key.entry_id(),
        )
    }
}

impl From<AssetLocationView> for GalleryEntry {
    fn from(source: AssetLocationView) -> Self {
        Self {
            source,
            group: None,
        }
    }
}

impl std::ops::Deref for GalleryEntry {
    type Target = AssetLocationView;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}
