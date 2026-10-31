use rusqlite::types::Value;

use crate::domain::{GalleryCollection, GalleryQuery};

use super::{
    duplicates, normalize_relative_folder, organization, organization_groups, organization_queue,
};

pub(super) fn gallery_source(query: &GalleryQuery, parameters: &mut Vec<Value>) -> String {
    if query.organization_queue.is_some() || query.collection != GalleryCollection::Library {
        let roots = if let Some(condition) = organization::presence_condition(query, parameters) {
            // An empty decision scope needs no location or group work.
            // The one-row relation preserves multiplicity when matching records do exist.
            format!(
                "FROM (SELECT 1 FROM organization_records AS record WHERE {condition} LIMIT 1)
                   AS organization_presence CROSS JOIN library_roots AS roots"
            )
        } else {
            "FROM library_roots AS roots".to_owned()
        };
        // Resolve organization predicates only inside published sources. Otherwise SQLite may
        // walk historical locations in display order before checking their active scan.
        format!(
            "{roots} CROSS JOIN asset_locations AS locations
               ON locations.root_id = roots.id AND locations.scan_id = roots.active_scan_id"
        )
    } else {
        "FROM library_roots AS roots
         JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id"
            .to_owned()
    }
}

pub(super) fn push_gallery_filters(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if organization_groups::uses_grouped_organization(query) {
        // The group subquery already applies the complete scope before choosing representatives.
        organization_groups::push_group_filter(query, clauses, parameters);
    } else {
        push_source_filters(query, clauses, parameters);
        duplicates::push_duplicate_filter(query, clauses, parameters);
    }
}

pub(super) fn push_source_filters(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    push_membership_filters(query, clauses, parameters);
    push_location_filters(query, clauses, parameters);
}

pub(super) fn push_location_filters(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if let Some(root_id) = &query.root_id {
        clauses.push("locations.root_id = ?".to_owned());
        parameters.push(Value::Text(root_id.clone()));
    }
    if let Some(folder) = &query.folder_relative_path {
        let folder = normalize_relative_folder(folder);
        if query.include_descendants {
            clauses.push(
                "(locations.parent_relative_path = ? OR \
                 substr(locations.parent_relative_path, 1, length(?) + 1) = ? || '/')"
                    .to_owned(),
            );
            parameters.extend([
                Value::Text(folder.clone()),
                Value::Text(folder.clone()),
                Value::Text(folder),
            ]);
        } else {
            clauses.push("locations.parent_relative_path = ?".to_owned());
            parameters.push(Value::Text(folder));
        }
    }
    super::search::push_text_filter(&query.search_text, clauses, parameters);
    super::metadata_filters::push_filters(&query.metadata_filter, clauses, parameters);
}

pub(super) fn push_membership_filters(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    organization::push_collection_filter(query, clauses, parameters);
    organization_queue::push_filter(query, clauses, parameters);
    super::classification::push_filter(query, clauses, parameters);
}

pub(super) fn push_anchor_filters(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if !organization_groups::uses_grouped_organization(query) {
        push_gallery_filters(query, clauses, parameters);
        return;
    }
    let mut membership = Vec::new();
    push_membership_filters(query, &mut membership, parameters);
    let mut group = Vec::new();
    organization_groups::push_group_filter(query, &mut group, parameters);
    if membership.is_empty() {
        clauses.extend(group);
        return;
    }
    // CASE keeps a rejected point lookup from materializing every eligible group.
    // Group selection still owns the physical scope and preferred representative.
    clauses.push(format!(
        "CASE WHEN {} THEN ({}) ELSE 0 END",
        membership.join(" AND "),
        group.join(" AND "),
    ));
}
