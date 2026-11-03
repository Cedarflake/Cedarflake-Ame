use crate::domain::{AssetLocationView, FileIdentityEvidence, ScanError};

use super::{CatalogRepository, IncrementalCatalogRepository};

pub(crate) trait ScanStagingRepository:
    CatalogRepository + IncrementalCatalogRepository
{
    fn with_initial_import_writes<T>(
        &mut self,
        scan_id: &str,
        operation: impl FnOnce(&mut Self) -> Result<T, ScanError>,
    ) -> Result<T, ScanError>
    where
        Self: Sized;

    fn load_retained_import_location(
        &self,
        scan_id: &str,
        location_id: &str,
    ) -> Result<Option<AssetLocationView>, ScanError>;
    fn complete_directory_with_identity(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
        identity: Option<&FileIdentityEvidence>,
    ) -> Result<(), ScanError>;
    fn finish_retained_import_inventory(&mut self, scan_id: &str) -> Result<(), ScanError>;
}
