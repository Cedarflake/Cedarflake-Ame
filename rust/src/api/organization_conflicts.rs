use crate::domain::ScanError;
use crate::domain::organization::OrganizationSelection;
use crate::domain::organization_conflicts::{
    OrganizationConflictAlbums, OrganizationConflictAnnotations, OrganizationConflictStatus,
};

pub fn load_organization_conflict_albums(
    selection: OrganizationSelection,
    after: Option<String>,
) -> Result<OrganizationConflictAlbums, ScanError> {
    crate::application::organization_conflicts::albums(selection, after)
}

pub fn load_organization_conflict_status() -> Result<OrganizationConflictStatus, ScanError> {
    crate::application::organization_conflicts::status()
}

pub fn load_organization_conflict_annotations(
    revision: u64,
    location_ids: Vec<String>,
) -> Result<OrganizationConflictAnnotations, ScanError> {
    crate::application::organization_conflicts::annotations(revision, location_ids)
}
