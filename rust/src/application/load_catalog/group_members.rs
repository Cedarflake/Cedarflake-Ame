use crate::domain::gallery_groups::{GalleryGroupKey, GalleryGroupMemberPage};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::catalog_reader::CatalogReader;

pub(crate) fn load(
    query: GalleryQuery,
    group: GalleryGroupKey,
    after: Option<String>,
) -> Result<GalleryGroupMemberPage, ScanError> {
    let storage = super::storage_paths()?;
    let catalog = super::open_catalog_reader(&storage.catalog_path)?;
    let mut page = catalog.load_gallery_group_members(
        &super::normalize_gallery_query(query),
        &group,
        after.as_deref().unwrap_or_default(),
    )?;
    for member in &mut page.members {
        let (current, root) = catalog.load_preview_context(&member.source.location_id)?;
        if let (Some(location), Some(root)) = (current, root)
            && location.source_generation == member.source.source_generation
        {
            member.availability =
                crate::application::exact_duplicates::inspect_candidate_availability(
                    &crate::domain::exact_duplicates::FingerprintCandidate { location, root },
                );
        }
    }
    Ok(page)
}
