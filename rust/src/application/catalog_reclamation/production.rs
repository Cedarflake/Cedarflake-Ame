use std::path::PathBuf;

use crate::ports::CatalogSpaceRepository;
use crate::ports::catalog_capacity::CatalogCapacity;

pub(super) fn maintenance(path: PathBuf) -> impl CatalogSpaceRepository {
    crate::adapters::SqliteCatalogSpaceMaintenance::new(path)
}

pub(super) fn capacity() -> impl CatalogCapacity {
    crate::adapters::LocalCatalogCapacity
}
