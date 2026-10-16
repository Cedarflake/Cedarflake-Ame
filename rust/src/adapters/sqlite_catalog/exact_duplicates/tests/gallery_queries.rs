use crate::domain::{GalleryDuplicateMode, GalleryQuery, GallerySortDirection, GallerySortKey};
use crate::ports::exact_duplicates::GalleryDuplicateRepository;

use super::*;

fn merged() -> GalleryQuery {
    GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    }
}

fn finish(catalog: &mut SqliteCatalog, run: &str) {
    let mut progress = ExactAnalysisProgress::running(run.to_owned());
    progress.phase = ExactAnalysisPhase::Completed;
    catalog.finish_exact_analysis(&progress).unwrap();
}

fn ids(catalog: &mut SqliteCatalog, query: &GalleryQuery) -> Vec<String> {
    catalog
        .load_snapshot(10, query, "query", None, None, None)
        .unwrap()
        .assets
        .into_iter()
        .map(|asset| asset.location_id)
        .collect()
}

#[test]
fn partial_analysis_preserves_gallery_revision_until_atomic_publication() {
    let (_directory, mut catalog) = fixture();
    let before = catalog
        .load_snapshot(1, &merged(), "query", None, None, None)
        .unwrap();
    assert!(publish(&mut catalog, "a", &"a".repeat(64)));
    assert!(publish(&mut catalog, "b", &"a".repeat(64)));
    assert_eq!(ids(&mut catalog, &merged()), ["a", "b", "c"]);
    assert!(
        catalog
            .load_gallery_duplicates(before.revision, &["a".to_owned()])
            .unwrap()
            .annotations
            .is_empty()
    );
    finish(&mut catalog, "run");
    let after = catalog
        .load_snapshot(1, &merged(), "query", None, None, None)
        .unwrap();
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(
        catalog
            .load_gallery_duplicates(before.revision, &["a".to_owned()])
            .unwrap_err()
            .code,
        "catalog_revision_changed"
    );
    assert_eq!(ids(&mut catalog, &merged()), ["a", "c"]);
    assert!(
        catalog
            .load_snapshot(
                1,
                &merged(),
                "query",
                before.next_cursor.as_ref(),
                None,
                None
            )
            .is_err()
    );
    catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "unchanged".to_owned(),
            started_unix_ms: 2,
        })
        .unwrap();
    finish(&mut catalog, "unchanged");
    assert_eq!(
        catalog
            .load_snapshot(1, &merged(), "query", None, None, None)
            .unwrap()
            .revision,
        after.revision
    );
}

#[test]
fn paging_timeline_layout_and_hidden_anchor_share_the_representative() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", &"a".repeat(64));
    publish(&mut catalog, "b", &"a".repeat(64));
    finish(&mut catalog, "run");
    let query = merged();
    let first = catalog
        .load_snapshot(1, &query, "query", None, None, None)
        .unwrap();
    let next = catalog
        .load_snapshot(1, &query, "query", first.next_cursor.as_ref(), None, None)
        .unwrap();
    assert_eq!(first.assets[0].location_id, "a");
    assert_eq!(next.assets[0].location_id, "c");
    assert!(next.next_cursor.is_none());
    let manifest = catalog
        .load_gallery_layout_manifest_chunk(10, &query, "query", None)
        .unwrap();
    assert_eq!(manifest.location_ids, ["a", "c"]);
    let chronological = GalleryQuery {
        sort_key: GallerySortKey::ModifiedTime,
        ..query.clone()
    };
    let timeline = catalog
        .load_gallery_timeline(&chronological, "dates")
        .unwrap();
    assert_eq!(timeline.total_items, 2);
    assert_eq!(
        timeline
            .buckets
            .iter()
            .map(|bucket| bucket.item_count)
            .sum::<u64>(),
        2
    );
    let hidden = candidate(&catalog, "b").location;
    let anchored = catalog
        .load_snapshot_around_asset(2, &query, "query", "b", &hidden.asset_id, 1)
        .unwrap();
    let resolution = anchored.query_anchor_resolution.unwrap();
    assert_eq!(resolution.requested_location_id, "b");
    assert_eq!(resolution.location_id.as_deref(), Some("a"));
    assert_eq!(resolution.ordinal, Some(0));
}

#[test]
fn scope_keeps_its_own_copy_and_badges_include_other_volumes() {
    let (_directory, mut catalog) = fixture();
    for id in ["a", "b", "c"] {
        publish(&mut catalog, id, &"a".repeat(64));
    }
    finish(&mut catalog, "run");
    let root_query = GalleryQuery {
        root_id: Some("root-b".to_owned()),
        ..merged()
    };
    assert_eq!(ids(&mut catalog, &root_query), ["c"]);
    assert_eq!(ids(&mut catalog, &GalleryQuery::default()).len(), 3);
    let snapshot = catalog
        .load_snapshot(10, &root_query, "root", None, None, None)
        .unwrap();
    let annotations = catalog
        .load_gallery_duplicates(snapshot.revision, &["c".to_owned()])
        .unwrap();
    assert_eq!(annotations.annotations.len(), 1);
    assert_eq!(annotations.annotations[0].location_id, "c");
    assert_eq!(annotations.annotations[0].group.location_count, 3);
    assert_eq!(annotations.annotations[0].group.physical_file_count, 3);
    catalog.connection.execute_batch("UPDATE asset_locations SET relative_path = 'Album/c.png', parent_relative_path = 'Album' WHERE location_id = 'c'").unwrap();
    let folder_query = GalleryQuery {
        folder_relative_path: Some("Album".to_owned()),
        include_descendants: false,
        search_text: "c.png".to_owned(),
        ..root_query
    };
    assert_eq!(ids(&mut catalog, &folder_query), ["c"]);
}

#[test]
fn changed_and_removed_locations_stop_folding_without_reading_sources() {
    let (_directory, mut catalog) = fixture();
    for id in ["a", "b", "c"] {
        publish(&mut catalog, id, &"a".repeat(64));
    }
    finish(&mut catalog, "run");
    assert_eq!(ids(&mut catalog, &merged()), ["a"]);
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET source_revision_token = 'changed' WHERE location_id = 'b'",
        )
        .unwrap();
    assert_eq!(ids(&mut catalog, &merged()), ["a", "b"]);
    catalog
        .connection
        .execute_batch("DELETE FROM asset_locations WHERE location_id = 'a'")
        .unwrap();
    assert_eq!(ids(&mut catalog, &merged()), ["b", "c"]);
    let page = catalog
        .load_snapshot(10, &merged(), "query", None, None, None)
        .unwrap();
    assert!(
        catalog
            .load_gallery_duplicates(page.revision, &["b".to_owned(), "c".to_owned()])
            .unwrap()
            .annotations
            .is_empty()
    );
}

#[test]
fn hardlink_paths_have_one_physical_file_but_separate_locations() {
    let (_directory, mut catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET source_generation = (
           SELECT source_generation FROM asset_locations WHERE location_id = 'a'
         ) WHERE location_id = 'b'",
        )
        .unwrap();
    publish(&mut catalog, "a", &"a".repeat(64));
    publish(&mut catalog, "c", &"a".repeat(64));
    finish(&mut catalog, "run");
    let page = catalog
        .load_snapshot(10, &merged(), "query", None, None, None)
        .unwrap();
    assert_eq!(page.assets.len(), 1);
    let annotations = catalog
        .load_gallery_duplicates(page.revision, &["a".to_owned(), "b".to_owned()])
        .unwrap();
    assert_eq!(annotations.annotations.len(), 2);
    for annotation in annotations.annotations {
        assert_eq!(annotation.group.location_count, 3);
        assert_eq!(annotation.group.physical_file_count, 2);
    }
}

#[test]
fn failed_gallery_publication_rolls_back_terminal_outcome_and_revision() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", &"a".repeat(64));
    publish(&mut catalog, "b", &"a".repeat(64));
    let before = catalog
        .load_snapshot(10, &merged(), "query", None, None, None)
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_projection BEFORE INSERT ON gallery_content_fingerprints
         BEGIN SELECT RAISE(ABORT, 'fixture publication failure'); END",
        )
        .unwrap();
    let mut progress = ExactAnalysisProgress::running("run".to_owned());
    progress.phase = ExactAnalysisPhase::Cancelled;
    assert!(catalog.finish_exact_analysis(&progress).is_err());
    assert_eq!(
        catalog
            .connection
            .query_row("SELECT COUNT(*) FROM exact_analysis_outcomes", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    let after = catalog
        .load_snapshot(10, &merged(), "query", None, None, None)
        .unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.assets.len(), 3);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_projection")
        .unwrap();
    catalog.finish_exact_analysis(&progress).unwrap();
    assert_eq!(ids(&mut catalog, &merged()), ["a", "c"]);
}
