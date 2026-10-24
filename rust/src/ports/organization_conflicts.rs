use crate::domain::ScanError;
use crate::domain::organization::OrganizationSelection;
use crate::domain::organization_conflicts::{
    OrganizationConflictAlbums, OrganizationConflictAnnotations, OrganizationConflictStatus,
};

pub(crate) trait OrganizationConflictRepository {
    fn load_organization_conflict_albums(
        &self,
        selection: &OrganizationSelection,
        after: Option<&str>,
    ) -> Result<OrganizationConflictAlbums, ScanError>;

    fn load_organization_conflict_status(&self) -> Result<OrganizationConflictStatus, ScanError>;

    fn load_organization_conflict_annotations(
        &self,
        revision: u64,
        location_ids: &[String],
    ) -> Result<OrganizationConflictAnnotations, ScanError>;
}
