use crate::domain::ScanError;
use crate::domain::organization::OrganizationSelection;
use crate::domain::organization_conflicts::{
    OrganizationConflictAlbums, OrganizationConflictAnnotations, OrganizationConflictStatus,
};
use crate::ports::organization_conflicts::OrganizationConflictRepository;

pub(crate) fn albums(
    selection: OrganizationSelection,
    after: Option<String>,
) -> Result<OrganizationConflictAlbums, ScanError> {
    super::organization::open_catalog()?
        .load_organization_conflict_albums(&selection, after.as_deref())
}

pub(crate) fn status() -> Result<OrganizationConflictStatus, ScanError> {
    super::organization::open_catalog()?.load_organization_conflict_status()
}

pub(crate) fn annotations(
    revision: u64,
    location_ids: Vec<String>,
) -> Result<OrganizationConflictAnnotations, ScanError> {
    super::organization::open_catalog()?
        .load_organization_conflict_annotations(revision, &location_ids)
}
