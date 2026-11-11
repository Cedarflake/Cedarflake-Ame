use rusqlite::Connection;

use crate::domain::file_operation_selection::FileOperationSelectionScope as Scope;
use crate::domain::organization::stale_selection;
use crate::domain::{GalleryDuplicateMode, ScanError};

use super::super::super::{database_error, gallery};

pub(super) fn selected(scope: &Scope) -> Result<gallery::BuiltGalleryQuery, ScanError> {
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = match scope {
        Scope::DuplicateCleanup { .. } => return Err(stale_selection()),
        Scope::Gallery {
            query,
            all_matching,
            ..
        }
        | Scope::MergedGalleryGroups {
            query,
            all_matching,
            ..
        } => {
            gallery::validate_gallery_query(query)?;
            let mut query = query.clone();
            if matches!(
                scope,
                Scope::MergedGalleryGroups {
                    all_matching: false,
                    ..
                }
            ) {
                // An explicitly selected card still identifies its group after the display
                // representative changes. Expansion freezes current members before confirmation.
                query.duplicate_mode = GalleryDuplicateMode::All;
            }
            let source = gallery::gallery_source(&query, &mut parameters);
            gallery::push_gallery_filters(&query, &mut clauses, &mut parameters);
            let membership = if *all_matching {
                "NOT EXISTS"
            } else {
                "EXISTS"
            };
            clauses.push(format!("{membership}(SELECT 1 FROM temp.file_operation_selection_input AS selected WHERE selected.asset_id = locations.asset_id)"));
            source
        }
        Scope::Locations => {
            clauses.push("EXISTS(SELECT 1 FROM temp.file_operation_selection_input AS selected
                WHERE selected.asset_id = locations.asset_id AND selected.location_id = locations.location_id)".into());
            "FROM library_roots AS roots JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id".into()
        }
    };
    clauses.push("root_state.is_active = 1".into());
    Ok(gallery::BuiltGalleryQuery {
        sql: format!("SELECT locations.asset_id, locations.location_id, locations.source_generation, root_state.generation, locations.file_size
            {source} JOIN library_change_root_state AS root_state ON root_state.root_id = locations.root_id
            WHERE {} ORDER BY locations.root_id, locations.location_id", clauses.join(" AND ")),
        parameters,
    })
}

pub(super) fn require_all_inputs(connection: &Connection, scope: &Scope) -> Result<(), ScanError> {
    if matches!(scope, Scope::Gallery { .. }) {
        let ambiguous: bool = connection.query_row(
            "SELECT EXISTS(SELECT asset_id FROM temp.file_operation_selection GROUP BY asset_id HAVING COUNT(*) > 1)",
            [],
            |row| row.get(0),
        ).map_err(database_error)?;
        if ambiguous {
            return Err(ScanError::new(
                "file_operation_locations_ambiguous",
                "Select the physical locations before applying this file action",
            ));
        }
    }
    if matches!(
        scope,
        Scope::Gallery {
            all_matching: true,
            ..
        } | Scope::MergedGalleryGroups {
            all_matching: true,
            ..
        }
    ) {
        return Ok(());
    }
    let location = if matches!(scope, Scope::Locations) {
        "AND actual.location_id = selected.location_id"
    } else {
        ""
    };
    let missing: bool = connection
        .query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM temp.file_operation_selection_input AS selected
         WHERE NOT EXISTS(SELECT 1 FROM temp.file_operation_selection AS actual
                          WHERE actual.asset_id = selected.asset_id {location}))"
            ),
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if missing {
        return Err(stale_selection());
    }
    Ok(())
}
