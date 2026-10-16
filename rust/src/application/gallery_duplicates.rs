use crate::domain::{GalleryDuplicatePage, LibraryChangeLane, ScanError};
use crate::ports::exact_duplicates::GalleryDuplicateRepository;

pub(crate) fn load_gallery_duplicates(
    revision: u64,
    location_ids: Vec<String>,
) -> Result<GalleryDuplicatePage, ScanError> {
    if location_ids.len() > 256
        || location_ids
            .iter()
            .any(|id| id.is_empty() || id.len() > 512)
    {
        return Err(ScanError::new(
            "gallery_duplicate_window_invalid",
            "Duplicate annotations require at most 256 current location identifiers",
        ));
    }
    let storage = super::storage_paths()?;
    let mut catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    catalog.load_gallery_duplicates(revision, &location_ids)
}
