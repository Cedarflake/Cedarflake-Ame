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
