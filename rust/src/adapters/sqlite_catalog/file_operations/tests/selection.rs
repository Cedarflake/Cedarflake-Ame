use std::sync::{Arc, atomic::AtomicBool};

use rusqlite::params;

use super::*;
use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};
use crate::domain::exact_duplicates::{
    ContentFingerprint, ExactAnalysisPhase, ExactAnalysisProgress, ExactAnalysisRun,
    FingerprintCandidate,
};
use crate::domain::file_operation_selection::FileOperationSelectionScope as Scope;
use crate::domain::organization::OrganizationSelection;
use crate::domain::{GalleryDuplicateMode, GalleryQuery};
use crate::ports::IncrementalCatalogRepository;
use crate::ports::exact_duplicate_review::ExactDuplicateReviewRepository;
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;

fn open(path: &std::path::Path, scope: Scope) -> SqliteFileOperationSelection {
    SqliteFileOperationSelection::new(
        SqliteCatalog::open(path.to_path_buf()).unwrap(),
        scope,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
}

#[test]
fn file_operation_selection_freezes_visible_files_or_explicitly_selected_groups() {
    let (directory, mut journal, _) = fixture();
    super::super::super::tests::publish_gallery_fixture(
        &mut journal.catalog,
        "scan-b",
        "root-b",
        "D:\\Fixture",
        &[("photo-b", None, 40), ("photo-c", None, 50)],
    );
    journal.catalog.connection.execute_batch(
        "INSERT INTO library_root_publication_namespaces(root_id, root_generation, identity_scheme, identity_value,
          authority_kind, established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1', '0000000000000001:00000000000000000000000000000002', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state;",
    ).unwrap();
    journal
        .catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "selected-duplicates".into(),
            started_unix_ms: 1,
        })
        .unwrap();
    let key = ExactDuplicateReviewKey {
        digest: "a".repeat(64),
        byte_length: 20,
    };
    for id in ["photo-a", "photo-b"] {
        let location = journal.catalog.load_active_location(id).unwrap().unwrap();
        let root = journal
            .catalog
            .load_incremental_catalog_root(&location.root_id)
            .unwrap()
            .unwrap();
        assert!(
            journal
                .catalog
                .publish_fingerprint(
                    "selected-duplicates",
                    &FingerprintCandidate { location, root },
                    &ContentFingerprint {
                        digest: key.digest.clone(),
                        byte_length: 20
                    },
                    &AtomicBool::new(false),
                )
                .unwrap()
        );
    }
    let mut progress = ExactAnalysisProgress::running("selected-duplicates".into());
    progress.phase = ExactAnalysisPhase::Completed;
    journal.catalog.finish_exact_analysis(&progress).unwrap();
    let first = journal
        .catalog
        .load_active_location("photo-a")
        .unwrap()
        .unwrap();
    journal
        .catalog
        .set_duplicate_preference(
            &key,
            Some(&ExactDuplicatePreference {
                location_id: first.location_id.clone(),
                asset_id: first.asset_id.clone(),
                source_generation: first.source_generation,
            }),
        )
        .unwrap();
    let revision = journal
        .catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap() as u64;
    let path = directory.path().join("catalog.sqlite3");
    let query = GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..GalleryQuery::default()
    };
    let mut selected = open(
        &path,
        Scope::Gallery {
            query: Box::new(query.clone()),
            catalog_revision: revision,
            all_matching: false,
        },
    );
    selected
        .append_ids(std::slice::from_ref(&first.asset_id))
        .unwrap();
    assert_eq!(selected.prepare().unwrap().file_count, 1);
    let mut group_query = query.clone();
    group_query.root_id = Some(first.root_id.clone());
    let mut group = open(
        &path,
        Scope::MergedGalleryGroups {
            query: Box::new(group_query.clone()),
            catalog_revision: revision,
            all_matching: false,
        },
    );
    group
        .append_ids(std::slice::from_ref(&first.asset_id))
        .unwrap();
    let summary = group.prepare().unwrap();
    assert_eq!(summary.file_count, 2);
    assert_eq!(summary.merged_group_count, Some(1));
    assert_eq!(
        group
            .page(None)
            .unwrap()
            .iter()
            .map(|item| item.location.location_id.as_str())
            .collect::<Vec<_>>(),
        ["photo-a", "photo-b"],
    );
    let second = journal
        .catalog
        .load_active_location("photo-b")
        .unwrap()
        .unwrap();
    journal
        .catalog
        .set_duplicate_preference(
            &key,
            Some(&ExactDuplicatePreference {
                location_id: second.location_id.clone(),
                asset_id: second.asset_id.clone(),
                source_generation: second.source_generation,
            }),
        )
        .unwrap();
    assert_eq!(
        selected.page(None).unwrap()[0].location.location_id,
        first.location_id
    );
    assert_eq!(group.prepare().unwrap().file_count, 2);
    let revision = journal
        .catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap() as u64;
    for scope in [
        Scope::Gallery {
            query: Box::new(query.clone()),
            catalog_revision: revision,
            all_matching: true,
        },
        Scope::MergedGalleryGroups {
            query: Box::new(query),
            catalog_revision: revision,
            all_matching: true,
        },
    ] {
        let mut all = open(&path, scope);
        all.append_ids(std::slice::from_ref(&second.asset_id))
            .unwrap();
        assert_eq!(all.prepare().unwrap().file_count, 1);
        assert_eq!(all.page(None).unwrap()[0].location.location_id, "photo-c");
    }
    journal.catalog.connection.execute(
        "UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = ?1",
        [&first.location_id],
    ).unwrap();
    journal
        .catalog
        .connection
        .execute(
            "UPDATE catalog_state SET next_source_generation = next_source_generation + 100",
            [],
        )
        .unwrap();
    assert!(selected.page(None).is_err());
    assert!(group.page(None).is_err());
    journal
        .catalog
        .connection
        .execute(
            "INSERT INTO gallery_retained_fingerprints VALUES (?1, ?2, 'blake3-256-full-v1')",
            params![
                (first.source_generation + 100) as i64,
                first.source_generation as i64
            ],
        )
        .unwrap();
    let mut pending = open(
        &path,
        Scope::MergedGalleryGroups {
            query: Box::new(group_query),
            catalog_revision: revision,
            all_matching: false,
        },
    );
    pending
        .append_ids(std::slice::from_ref(&first.asset_id))
        .unwrap();
    assert_eq!(
        pending.prepare().unwrap_err().code,
        "organization_evidence_pending"
    );
}

#[test]
fn file_operation_selection_reads_current_state_before_freezing_confirmation() {
    let (directory, journal, _) = fixture();
    let connection = &journal.catalog.connection;
    let original = journal
        .catalog
        .load_active_location("photo-a")
        .unwrap()
        .unwrap();
    let revision = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap() as u64;
    let path = directory.path().join("catalog.sqlite3");
    let query = GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..GalleryQuery::default()
    };
    connection.execute_batch(
        "UPDATE catalog_state SET revision = revision + 1, next_source_generation = next_source_generation + 1;
         UPDATE asset_locations SET source_generation = source_generation + 1, file_size = 45;",
    ).unwrap();
    for scope in [
        Scope::Gallery {
            query: Box::new(query.clone()),
            catalog_revision: revision,
            all_matching: false,
        },
        Scope::MergedGalleryGroups {
            query: Box::new(query.clone()),
            catalog_revision: revision,
            all_matching: false,
        },
        Scope::Locations,
    ] {
        let locations = matches!(scope, Scope::Locations);
        let mut selected = open(&path, scope);
        if locations {
            selected
                .append_locations(&[OrganizationSelection {
                    asset_id: original.asset_id.clone(),
                    location_id: original.location_id.clone(),
                    source_generation: original.source_generation,
                }])
                .unwrap();
        } else {
            selected
                .append_ids(std::slice::from_ref(&original.asset_id))
                .unwrap();
        }
        let summary = selected.prepare().unwrap();
        assert_eq!((summary.file_count, summary.total_bytes), (1, 45));
        let frozen = selected.page(None).unwrap();
        assert_eq!(frozen[0].location.location_id, original.location_id);
        assert_eq!(
            frozen[0].location.source_generation,
            original.source_generation + 1
        );
        connection
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .unwrap();
        assert_eq!(selected.prepare().unwrap(), summary);
        assert_eq!(selected.page(None).unwrap().len(), 1);
        connection
            .execute(
                "UPDATE asset_locations SET source_generation = source_generation + 1",
                [],
            )
            .unwrap();
        assert!(selected.page(None).is_err());
        connection
            .execute(
                "UPDATE asset_locations SET source_generation = source_generation - 1",
                [],
            )
            .unwrap();
    }
    let mut all = open(
        &path,
        Scope::Gallery {
            query: Box::new(query),
            catalog_revision: revision,
            all_matching: true,
        },
    );
    assert_eq!(
        all.prepare().unwrap_err().code,
        "organization_selection_changed"
    );
}

#[test]
fn file_operation_selection_rejects_conflicting_physical_generations() {
    let (directory, journal, _) = fixture();
    let location = journal
        .catalog
        .load_active_location("photo-a")
        .unwrap()
        .unwrap();
    let mut selected = open(&directory.path().join("catalog.sqlite3"), Scope::Locations);
    let mut item = OrganizationSelection {
        asset_id: location.asset_id,
        location_id: location.location_id,
        source_generation: location.source_generation,
    };
    selected
        .append_locations(std::slice::from_ref(&item))
        .unwrap();
    item.source_generation += 1;
    assert!(selected.append_locations(&[item]).is_err());
    assert_eq!(selected.prepare().unwrap().file_count, 1);
}
