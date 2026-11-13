use rusqlite::types::Value;

use crate::domain::gallery_groups::{GalleryGroup, GalleryGroupFilter, GalleryGroupKey};

use crate::domain::{GalleryQuery, ScanError};

use super::super::{database_error, sqlite_unsigned};

pub(in super::super) fn is_filtered(query: &GalleryQuery) -> bool {
    query.group_filter != GalleryGroupFilter::Unrestricted
}

pub(super) fn source(query: &GalleryQuery, parameters: &mut Vec<Value>) -> String {
    let groups = grouped_query(query, parameters);
    format!(
        "FROM ({groups}) AS gallery_groups
         CROSS JOIN asset_locations AS locations INDEXED BY asset_locations_location_id
           ON locations.location_id = gallery_groups.representative
         JOIN library_roots AS roots
           ON roots.id = locations.root_id AND roots.active_scan_id = locations.scan_id"
    )
}

pub(in super::super) use members::inspection as inspection_members;
pub(in super::super) use members::query as member_query;
pub(in super::super) use members::visible as visible_member_query;
mod members;
pub(in super::super) mod selection;
#[cfg(test)]
mod tests;

pub(super) fn grouped_query(query: &GalleryQuery, parameters: &mut Vec<Value>) -> String {
    let members = member_query(query, parameters);
    match query.group_filter {
        GalleryGroupFilter::Similar => format!(
            "WITH members AS ({members}), visible_members AS (
               SELECT entry_id, similar_group_id, content_key,
                 COALESCE(MIN(CASE WHEN location_preferred THEN location_id END), MIN(location_id)) AS representative,
                 MAX(preferred) AS preferred, MIN(semantic_rank) AS semantic_rank
               FROM members GROUP BY entry_id, content_key
             )
             SELECT entry_id, NULL AS exact_digest, NULL AS exact_bytes, similar_group_id,
               COUNT(*) AS member_count,
               COALESCE(MIN(CASE WHEN preferred THEN representative END), MIN(representative)) AS representative,
               MIN(semantic_rank) AS semantic_rank
             FROM visible_members GROUP BY entry_id HAVING COUNT(*) > 1"
        ),
        _ => format!(
            "WITH members AS ({members})
             SELECT entry_id, exact_digest, exact_bytes, NULL AS similar_group_id, COUNT(*) AS member_count,
               COALESCE(MIN(CASE WHEN preferred THEN location_id END), MIN(location_id)) AS representative,
               MIN(semantic_rank) AS semantic_rank
             FROM members GROUP BY entry_id HAVING COUNT(*) > 1"
        ),
    }
}
pub(super) fn columns(query: &GalleryQuery) -> &'static str {
    if is_filtered(query) {
        "gallery_groups.exact_digest, gallery_groups.exact_bytes,
         gallery_groups.similar_group_id, gallery_groups.member_count"
    } else {
        "NULL AS exact_digest, NULL AS exact_bytes, NULL AS similar_group_id, NULL AS member_count"
    }
}

pub(super) fn identity(query: &GalleryQuery) -> &'static str {
    if is_filtered(query) {
        "gallery_groups.entry_id"
    } else {
        "locations.location_id"
    }
}

pub(in super::super) fn read(row: &rusqlite::Row<'_>) -> Result<Option<GalleryGroup>, ScanError> {
    let Some(key) = read_key(row).map_err(database_error)? else {
        return Ok(None);
    };
    Ok(Some(GalleryGroup {
        key,
        member_count: sqlite_unsigned(
            row.get("member_count").map_err(database_error)?,
            "group member count",
        )?,
    }))
}

pub(super) fn read_key(row: &rusqlite::Row<'_>) -> rusqlite::Result<Option<GalleryGroupKey>> {
    let digest: Option<String> = row.get("exact_digest")?;
    let similar_id: Option<String> = row.get("similar_group_id")?;
    let key = if let Some(digest) = digest {
        GalleryGroupKey::Exact {
            digest,
            byte_length: super::super::exact_duplicates::read_unsigned(
                row,
                row.as_ref().column_index("exact_bytes")?,
            )?,
        }
    } else if let Some(group_id) = similar_id {
        GalleryGroupKey::Similar { group_id }
    } else {
        return Ok(None);
    };
    Ok(Some(key))
}
