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
            let source = if !all_matching && query.semantic_results.is_none() {
                // Start from the selected identities even before SQLite has table statistics.
                "FROM temp.file_operation_selection_input AS selected
                 CROSS JOIN asset_locations AS locations INDEXED BY asset_locations_asset_id
                   ON locations.asset_id = selected.asset_id
                 CROSS JOIN library_roots AS roots
                   ON roots.id = locations.root_id AND roots.active_scan_id = locations.scan_id".into()
            } else {
                gallery::gallery_source(&query, &mut parameters)
            };
            gallery::push_gallery_filters(&query, &mut clauses, &mut parameters);
            let membership = if *all_matching {
                "NOT EXISTS(SELECT 1 FROM temp.file_operation_selection_input AS selected
                    WHERE selected.asset_id = locations.asset_id)"
            } else {
                "locations.asset_id IN (SELECT asset_id FROM temp.file_operation_selection_input)"
            };
            clauses.push(membership.into());
            source
        }
        Scope::Locations => {
            "FROM temp.file_operation_selection_input AS selected
             CROSS JOIN asset_locations AS locations INDEXED BY asset_locations_location_id
               ON locations.location_id = selected.location_id AND locations.asset_id = selected.asset_id
             CROSS JOIN library_roots AS roots
               ON roots.id = locations.root_id AND roots.active_scan_id = locations.scan_id".into()
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

#[cfg(test)]
mod tests {
    use super::Scope;
    use crate::adapters::sqlite_catalog::file_operations::{
        SqliteFileOperationSelection, tests::fixture,
    };
    use crate::adapters::sqlite_catalog::{SqliteCatalog, tests::publish_gallery_fixture};
    use crate::domain::exact_duplicates::ExactAnalysisRun;
    use crate::domain::organization::OrganizationSelection;
    use crate::domain::{GalleryDuplicateMode, GalleryQuery};
    use crate::ports::{
        CatalogRepository, exact_duplicates::ExactDuplicateRepository,
        file_operation_selection::FileOperationSelectionRepository,
    };
    use std::sync::{Arc, atomic::AtomicBool};

    fn open(path: &std::path::Path, scope: Scope) -> SqliteFileOperationSelection {
        SqliteFileOperationSelection::new(
            SqliteCatalog::open(path.to_path_buf()).unwrap(),
            scope,
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap()
    }
    #[test]
    fn explicit_file_selection_does_not_scan_unrelated_locations_or_fingerprints() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (directory, mut journal, _) = fixture();
        let ids: Vec<_> = (0..2048)
            .map(|index| format!("unrelated-{index}"))
            .collect();
        let locations: Vec<_> = ids.iter().map(|id| (id.as_str(), None, 1)).collect();
        publish_gallery_fixture(
            &mut journal.catalog,
            "scan-unrelated",
            "root-unrelated",
            "D:\\Fixture",
            &locations,
        );
        journal
            .catalog
            .begin_exact_analysis(&ExactAnalysisRun {
                run_id: "selection-cost".into(),
                started_unix_ms: 1,
            })
            .unwrap();
        journal.catalog.connection.execute_batch(
        "INSERT INTO content_fingerprints
           SELECT source_generation, 'blake3-256-full-v1', printf('%064x', source_generation),
                  file_size, file_identity_scheme, file_identity_value, source_revision_token,
                  'selection-cost'
           FROM asset_locations;
         INSERT INTO gallery_content_fingerprints SELECT source_generation, scheme FROM content_fingerprints;",
    ).unwrap();
        let target = journal
            .catalog
            .load_active_location("photo-a")
            .unwrap()
            .unwrap();
        let path = directory.path().join("catalog.sqlite3");
        for scope in [
            Scope::Gallery {
                query: Box::new(GalleryQuery::default()),
                catalog_revision: 0,
                all_matching: false,
            },
            Scope::MergedGalleryGroups {
                query: Box::new(GalleryQuery {
                    duplicate_mode: GalleryDuplicateMode::Merged,
                    ..GalleryQuery::default()
                }),
                catalog_revision: 0,
                all_matching: false,
            },
            Scope::Locations,
        ] {
            let label = format!("{scope:?}");
            let physical = matches!(scope, Scope::Locations);
            let mut selected = open(&path, scope);
            if physical {
                selected
                    .append_locations(&[OrganizationSelection {
                        asset_id: target.asset_id.clone(),
                        location_id: target.location_id.clone(),
                        source_generation: target.source_generation,
                    }])
                    .unwrap();
            } else {
                selected
                    .append_ids(std::slice::from_ref(&target.asset_id))
                    .unwrap();
            }
            let steps = Arc::new(AtomicUsize::new(0));
            let observed = Arc::clone(&steps);
            selected
                .catalog
                .connection
                .progress_handler(
                    100,
                    Some(move || {
                        observed.fetch_add(100, Ordering::Relaxed);
                        false
                    }),
                )
                .unwrap();
            let summary = selected.prepare().unwrap();
            selected
                .catalog
                .connection
                .progress_handler(0, None::<fn() -> bool>)
                .unwrap();
            assert_eq!(summary.file_count, 1);
            assert_eq!(
                selected.page(None).unwrap()[0].location.location_id,
                "photo-a"
            );
            assert!(
                steps.load(Ordering::Relaxed) < 8000,
                "{label}: one selected file must not walk unrelated catalog/evidence rows: {} steps",
                steps.load(Ordering::Relaxed)
            );
        }
    }
}
