use super::ScanError;

pub(crate) const MAX_ORGANIZATION_SELECTION: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrganizationSelection {
    pub asset_id: String,
    pub location_id: String,
    pub source_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrganizationGroupKey {
    Confirmed {
        digest: String,
        byte_length: u64,
    },
    Provisional {
        asset_id: String,
        source_generation: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationGroupTarget {
    pub key: OrganizationGroupKey,
    pub representative: OrganizationSelection,
    pub location_count: u64,
    pub membership_authority: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationPreparedSelection {
    pub catalog_revision: u64,
    pub organization_revision: u64,
    pub selections: Vec<OrganizationSelection>,
    pub groups: Vec<OrganizationGroupTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationWriteReceipt {
    pub operation_id: String,
    pub organization_revision: u64,
    pub changed_groups: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationDisposition {
    Pending,
    Deferred,
    Organized,
    Ignored,
}

impl OrganizationDisposition {
    pub(crate) fn stored(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Deferred => "deferred",
            Self::Organized => "organized",
            Self::Ignored => "ignored",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationAction {
    AssignAlbum { album_id: String },
    RemoveAlbum,
    Defer,
    Ignore,
    RestorePending,
}

impl OrganizationAction {
    pub(crate) fn decision(&self) -> (OrganizationDisposition, Option<&str>) {
        match self {
            Self::AssignAlbum { album_id } => (OrganizationDisposition::Organized, Some(album_id)),
            Self::RemoveAlbum | Self::Ignore => (OrganizationDisposition::Ignored, None),
            Self::Defer => (OrganizationDisposition::Deferred, None),
            Self::RestorePending => (OrganizationDisposition::Pending, None),
        }
    }

    pub(crate) fn request_name(&self) -> &'static str {
        match self {
            Self::AssignAlbum { .. } => "assign",
            Self::RemoveAlbum => "remove",
            Self::Defer => "defer",
            Self::Ignore => "ignore",
            Self::RestorePending => "pending",
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        if let Self::AssignAlbum { album_id } = self {
            validate_identifier(album_id)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationAlbum {
    pub album_id: String,
    pub name: String,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationAlbumPage {
    pub albums: Vec<OrganizationAlbum>,
    pub total_albums: u64,
    pub organization_revision: u64,
    pub next_album_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationAlbumReceipt {
    pub album_id: String,
    pub operation: OrganizationWriteReceipt,
}

pub(crate) fn normalize_album_name(name: &str) -> Result<String, ScanError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(ScanError::new(
            "organization_album_name_invalid",
            "An album name requires between 1 and 128 visible characters",
        ));
    }
    Ok(name.to_owned())
}

pub(crate) fn validate_selections(selections: &[OrganizationSelection]) -> Result<(), ScanError> {
    if selections.is_empty() || selections.len() > MAX_ORGANIZATION_SELECTION {
        return Err(ScanError::new(
            "organization_selection_invalid",
            "An organization command requires between 1 and 256 selected locations",
        ));
    }
    for selection in selections {
        validate_identifier(&selection.asset_id)?;
        validate_identifier(&selection.location_id)?;
        if selection.source_generation == 0 || selection.source_generation > i64::MAX as u64 {
            return Err(stale_selection());
        }
    }
    Ok(())
}

pub(crate) fn validate_identifier(value: &str) -> Result<(), ScanError> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(ScanError::new(
            "organization_identifier_invalid",
            "The organization identifier is empty, oversized or contains control characters",
        ));
    }
    Ok(())
}

pub(crate) fn stale_selection() -> ScanError {
    ScanError::new(
        "organization_selection_changed",
        "The selected image group changed; refresh the selection before saving",
    )
}
