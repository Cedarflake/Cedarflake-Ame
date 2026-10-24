use crate::domain::{GalleryQuery, ScanError};

use super::{BuiltGalleryQuery, push_gallery_filters, validate_gallery_query};

pub(in super::super) fn selected_locations(
    query: &GalleryQuery,
    all_matching: bool,
) -> Result<BuiltGalleryQuery, ScanError> {
    validate_gallery_query(query)?;
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let membership = if all_matching { "NOT EXISTS" } else { "EXISTS" };
    clauses.push(format!(
        "{membership}(SELECT 1 FROM temp.organization_batch_ids AS selected
         WHERE selected.asset_id = locations.asset_id)"
    ));
    Ok(BuiltGalleryQuery {
        sql: format!(
            "SELECT locations.asset_id, locations.location_id, locations.source_generation
             FROM library_roots AS roots
             JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
             WHERE {}",
            clauses.join(" AND "),
        ),
        parameters,
    })
}
