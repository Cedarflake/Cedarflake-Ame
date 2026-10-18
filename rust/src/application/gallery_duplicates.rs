use crate::domain::exact_duplicates::{EXACT_DUPLICATE_PAGE_SIZE, GalleryDuplicateLocationPage};
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

pub(crate) fn load_gallery_duplicate_locations(
    digest: String,
    byte_length: u64,
    after_location_id: Option<String>,
) -> Result<GalleryDuplicateLocationPage, ScanError> {
    super::exact_duplicates::validate_digest(&digest)?;
    if let Some(id) = &after_location_id {
        super::exact_duplicates::validate_identifier(id)?;
    }
    let storage = super::storage_paths()?;
    let mut catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let mut page = catalog.load_gallery_duplicate_locations(
        &digest,
        byte_length,
        after_location_id.as_deref().unwrap_or(""),
    )?;
    drop(catalog);
    let next_location_id = if page.members.len() > EXACT_DUPLICATE_PAGE_SIZE {
        page.members.truncate(EXACT_DUPLICATE_PAGE_SIZE);
        page.members
            .last()
            .map(|member| member.location.location_id.clone())
    } else {
        None
    };
    let locations = page
        .members
        .into_iter()
        .map(|member| {
            let availability = super::exact_duplicates::inspect_candidate_availability(&member);
            let location = member.location;
            crate::domain::ExactDuplicateLocation {
                asset_id: location.asset_id,
                location_id: location.location_id,
                root_id: location.root_id,
                display_path: location.display_path,
                source_generation: location.source_generation,
                availability,
            }
        })
        .collect();
    Ok(GalleryDuplicateLocationPage {
        locations,
        next_location_id,
        location_count: page.location_count,
        pending_location_count: page.pending_location_count,
    })
}
