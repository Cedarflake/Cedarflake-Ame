use rusqlite::types::Value;

use crate::domain::{GalleryCollection, GalleryQuery};

use super::{duplicates, normalize_relative_folder, organization, organization_queue};

pub(super) fn gallery_source(query: &GalleryQuery) -> &'static str {
    if query.organization_queue.is_some() || query.collection != GalleryCollection::Library {
        // Resolve organization predicates only inside published sources. Otherwise SQLite may
        // walk historical locations in display order before checking their active scan.
        "FROM library_roots AS roots
         CROSS JOIN asset_locations AS locations
           ON locations.root_id = roots.id AND locations.scan_id = roots.active_scan_id"
    } else {
        "FROM library_roots AS roots
         JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id"
    }
}

pub(super) fn push_gallery_filters(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if query.folds_duplicates()
        && (query.organization_queue.is_some() || query.collection != GalleryCollection::Library)
    {
        // The group subquery already applies the complete scope before choosing representatives.
        organization::push_group_filter(query, clauses, parameters);
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
    organization::push_collection_filter(query, clauses, parameters);
    organization_queue::push_filter(query, clauses, parameters);
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
    let search = query.search_text.trim();
    if !search.is_empty() {
        clauses.push(
            "(instr(lower(locations.absolute_path), lower(?)) > 0 OR \
             instr(lower(locations.relative_path), lower(?)) > 0)"
                .to_owned(),
        );
        parameters.extend([
            Value::Text(search.to_owned()),
            Value::Text(search.to_owned()),
        ]);
    }
}
