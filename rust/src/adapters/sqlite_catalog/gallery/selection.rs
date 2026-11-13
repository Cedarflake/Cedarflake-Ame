use crate::domain::{GalleryQuery, ScanError};

use super::{BuiltGalleryQuery, push_gallery_filters, validate_gallery_query};

pub(in super::super) fn selected_locations(
    query: &GalleryQuery,
    all_matching: bool,
) -> Result<BuiltGalleryQuery, ScanError> {
    validate_gallery_query(query)?;
    if super::groups::is_filtered(query) {
        let members = super::groups::selection::selected_members(
            query,
            super::groups::selection::Input::Organization,
            all_matching,
        );
        return Ok(BuiltGalleryQuery {
            sql: format!(
                "SELECT DISTINCT asset_id, location_id, source_generation FROM ({})",
                members.sql
            ),
            parameters: members.parameters,
        });
    }
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = super::gallery_source(query, &mut parameters);
    push_gallery_filters(query, &mut clauses, &mut parameters);
    let membership = if all_matching { "NOT EXISTS" } else { "EXISTS" };
    clauses.push(format!(
        "{membership}(SELECT 1 FROM temp.organization_batch_ids AS selected
         WHERE selected.asset_id = locations.asset_id)"
    ));
    Ok(BuiltGalleryQuery {
        sql: format!(
            "SELECT locations.asset_id, locations.location_id, locations.source_generation
             {source}
             WHERE {}",
            clauses.join(" AND "),
        ),
        parameters,
    })
}
