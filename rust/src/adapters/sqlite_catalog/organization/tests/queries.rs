use crate::adapters::sqlite_catalog::gallery_snapshot::set_after_query_snapshot_read_hook;
use crate::domain::{
    GalleryCollection, GalleryDuplicateMode, GalleryQuery, GallerySortDirection, GallerySortKey,
};
use crate::ports::GalleryQueryRepository;

use super::transitions::{change, create_album};
use super::*;

pub(super) mod evolution;
mod fingerprint_cost;
mod queues;

fn album_query(album_id: &str) -> GalleryQuery {
    GalleryQuery {
        collection: GalleryCollection::Album {
            album_id: album_id.into(),
        },
        sort_key: GallerySortKey::FileName,
        sort_direction: GallerySortDirection::Ascending,
        ..GalleryQuery::default()
    }
}

fn assign(catalog: &mut SqliteCatalog, operation: &str, ids: &[&str], album: &str) {
    change(
        catalog,
        operation,
        ids,
        OrganizationAction::AssignAlbum {
            album_id: album.into(),
        },
    );
}

pub(super) fn publish_batch(catalog: &mut SqliteCatalog, ids: &[&str], digest: char) {
    for id in ids {
        publish(catalog, id, digest);
    }
    let generations = ids
        .iter()
        .map(|id| selected(catalog, id).source_generation)
        .collect::<Vec<_>>();
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
}

fn ids(catalog: &mut SqliteCatalog, query: &GalleryQuery) -> Vec<String> {
    catalog
        .load_snapshot(10, query, "query", None, None, None)
        .unwrap()
        .assets
        .into_iter()
        .map(|a| a.location_id)
        .collect()
}

fn grouped() -> (tempfile::TempDir, SqliteCatalog, GalleryQuery) {
    let (directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let album = create_album(&mut catalog, "Album");
    assign(&mut catalog, "assign", &["c", "b"], &album);
    (directory, catalog, album_query(&album))
}

#[test]
fn album_pages_counts_layout_and_viewer_anchor_always_fold_the_same_group() {
    let (_directory, mut catalog, query) = grouped();
    assert_eq!(query.duplicate_mode, GalleryDuplicateMode::All);
    let first = catalog
        .load_query_snapshot(1, &query, "query", None)
        .unwrap();
    assert_eq!(first.snapshot.assets[0].location_id, "a");
    assert_eq!(
        (
            first.timeline.total_items,
            first.timeline.total_source_items
        ),
        (2, 3)
    );
    let second = catalog
        .load_snapshot(
            1,
            &query,
            "query",
            first.snapshot.next_cursor.as_ref(),
            None,
            None,
        )
        .unwrap();
    assert_eq!(second.assets[0].location_id, "b");
    assert!(second.next_cursor.is_none());
    let backward = catalog
        .load_snapshot(
            1,
            &query,
            "query",
            None,
            second.previous_cursor.as_ref(),
            None,
        )
        .unwrap();
    assert_eq!(backward.assets[0].location_id, "a");
    let layout = catalog
        .load_gallery_layout_manifest_chunk(1, &query, "query", None)
        .unwrap();
    assert_eq!(layout.location_ids, ["a"]);
    assert_eq!(layout.total_items, 2);
    let next_layout = catalog
        .load_gallery_layout_manifest_chunk(1, &query, "query", layout.next_cursor.as_ref())
        .unwrap();
    assert_eq!(next_layout.location_ids, ["b"]);
    let hidden = selected(&catalog, "c");
    let anchored = catalog
        .load_snapshot_around_asset(1, &query, "query", "c", &hidden.asset_id, 1)
        .unwrap();
    let resolved = anchored.query_anchor_resolution.unwrap();
    assert_eq!(resolved.location_id.as_deref(), Some("a"));
    assert_eq!(resolved.ordinal, Some(0));
    let timeline = catalog
        .load_gallery_timeline(
            &GalleryQuery {
                sort_key: GallerySortKey::ModifiedTime,
                ..query
            },
            "dates",
        )
        .unwrap();
    assert_eq!(
        timeline
            .buckets
            .iter()
            .map(|bucket| bucket.item_count)
            .sum::<u64>(),
        2
    );
}

#[test]
fn preferred_copy_and_physical_scope_use_the_same_collection_membership() {
    let (_directory, mut catalog, query) = grouped();
    let preferred = selected(&catalog, "c");
    catalog
        .set_duplicate_preference(
            &ExactDuplicateReviewKey {
                digest: "a".repeat(64),
                byte_length: 20,
            },
            Some(&ExactDuplicatePreference {
                location_id: "c".into(),
                asset_id: preferred.asset_id,
                source_generation: preferred.source_generation,
            }),
        )
        .unwrap();
    assert_eq!(ids(&mut catalog, &query), ["b", "c"]);
    let scoped = GalleryQuery {
        root_id: Some("root-a".into()),
        ..query.clone()
    };
    assert_eq!(ids(&mut catalog, &scoped), ["a", "b"]);
    catalog.connection.execute("UPDATE asset_locations SET relative_path = 'Album/c.png', parent_relative_path = 'Album' WHERE location_id = 'c'", []).unwrap();
    let folder = GalleryQuery {
        root_id: Some("root-c".into()),
        folder_relative_path: Some("Album".into()),
        include_descendants: false,
        search_text: "c.png".into(),
        ..query
    };
    let snapshot = catalog
        .load_query_snapshot(10, &folder, "folder", None)
        .unwrap();
    assert_eq!(snapshot.snapshot.assets[0].location_id, "c");
    assert_eq!(
        (
            snapshot.timeline.total_items,
            snapshot.timeline.total_source_items
        ),
        (1, 1)
    );
}

#[test]
fn favorite_scope_is_independent_and_respects_unfolded_browsing() {
    let (_directory, mut catalog, query) = grouped();
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let favorites = GalleryQuery {
        collection: GalleryCollection::Favorites,
        ..query.clone()
    };
    assert_eq!(ids(&mut catalog, &favorites), ["a", "c"]);
    let folded = GalleryQuery {
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..favorites
    };
    let snapshot = catalog
        .load_query_snapshot(10, &folded, "favorites", None)
        .unwrap();
    assert_eq!(snapshot.snapshot.assets.len(), 1);
    assert_eq!(
        (
            snapshot.timeline.total_items,
            snapshot.timeline.total_source_items
        ),
        (1, 2)
    );
    change(
        &mut catalog,
        "remove",
        &["c"],
        OrganizationAction::RemoveAlbum,
    );
    assert_eq!(ids(&mut catalog, &query), ["b"]);
    assert_eq!(ids(&mut catalog, &folded), ["a"]);
}

#[test]
fn empty_missing_and_retired_albums_have_distinct_results() {
    let (_directory, mut catalog) = fixture();
    let album = create_album(&mut catalog, "Empty");
    let query = album_query(&album);
    let snapshot = catalog
        .load_query_snapshot(1, &query, "empty", None)
        .unwrap();
    assert!(snapshot.snapshot.assets.is_empty());
    assert_eq!(snapshot.timeline.total_items, 0);
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .delete_organization_album("delete", revision, &album)
        .unwrap();
    for query in [query, album_query("missing")] {
        assert_eq!(
            catalog
                .load_query_snapshot(1, &query, "gone", None)
                .unwrap_err()
                .code,
            "organization_album_unavailable"
        );
        assert_eq!(
            catalog
                .load_gallery_layout_manifest_chunk(1, &query, "gone", None)
                .unwrap_err()
                .code,
            "organization_album_unavailable"
        );
    }
    assert_eq!(
        catalog
            .load_gallery_timeline(&album_query(""), "invalid")
            .unwrap_err()
            .code,
        "organization_identifier_invalid"
    );
}

#[test]
fn collection_counts_and_pages_share_a_snapshot_during_an_album_move() {
    let (directory, mut catalog, query) = grouped();
    let destination = create_album(&mut catalog, "Destination");
    let path = directory.path().join("catalog.sqlite3");
    set_after_query_snapshot_read_hook(move || {
        let mut writer = SqliteCatalog::open(path).unwrap();
        assign(&mut writer, "move", &["c"], &destination);
    });
    let previous = catalog
        .load_query_snapshot(1, &query, "query", None)
        .unwrap();
    assert_eq!(previous.snapshot.assets[0].location_id, "a");
    assert_eq!(
        (
            previous.timeline.total_items,
            previous.timeline.total_source_items
        ),
        (2, 3)
    );
    assert_eq!(previous.timeline.revision, previous.snapshot.revision);
    let current = catalog
        .load_query_snapshot(1, &query, "query", None)
        .unwrap();
    assert_eq!(current.snapshot.assets[0].location_id, "b");
    assert_eq!(current.timeline.total_items, 1);
    assert!(current.snapshot.revision > previous.snapshot.revision);
    assert_eq!(
        catalog
            .load_snapshot(
                1,
                &query,
                "query",
                previous.snapshot.next_cursor.as_ref(),
                None,
                None
            )
            .unwrap_err()
            .code,
        "catalog_cursor_stale"
    );
}
