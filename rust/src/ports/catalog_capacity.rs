use std::path::Path;

use crate::domain::ScanError;

pub(crate) trait CatalogCapacity {
    fn available_for(&self, catalog_path: &Path) -> Result<u64, ScanError>;
}
