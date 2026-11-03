use crate::domain::classification_configuration::ClassificationSettings;
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_configuration::ClassificationNavigationRepository;

use super::{StoragePaths, open_catalog, settings, storage_paths};

pub(crate) fn load_navigation() -> Result<ClassificationSettings, ScanError> {
    load_with_storage(&storage_paths()?)
}

fn load_with_storage(storage: &StoragePaths) -> Result<ClassificationSettings, ScanError> {
    let catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let snapshot = catalog.load_classification_navigation()?;
    let mut result = settings(snapshot.configuration, storage)?;
    result.categories.retain(|category| {
        snapshot
            .populated_categories
            .iter()
            .any(|id| id.as_str() == category.id)
    });
    Ok(result)
}
