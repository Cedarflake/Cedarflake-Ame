use crate::domain::{GalleryQuery, GalleryTimeIntent, GalleryTimeSnapshot, ScanError};

use super::{
    finish_loaded_snapshot, gallery_query_identity, normalize_gallery_query,
    prepare_catalog_reader, storage_paths,
};

pub fn load_catalog_time_snapshot(
    max_items: u32,
    query: GalleryQuery,
    intent: GalleryTimeIntent,
) -> Result<GalleryTimeSnapshot, ScanError> {
    let query = normalize_gallery_query(query);
    let query_id = gallery_query_identity(&query);
    let storage = storage_paths()?;
    let mut result = prepare_catalog_reader(&storage.catalog_path)?
        .load_time_snapshot(max_items, &query, &query_id, &intent)?;
    finish_loaded_snapshot(&storage, &mut result.snapshot)?;
    Ok(result)
}
