use crate::adapters::sqlite_catalog::tests::support::{
    TEST_QUERY_ID, load_default_snapshot, publish_fixture, publish_gallery_fixture,
    publish_gallery_query_fixture,
};
use crate::domain::{GalleryQueryAnchor, LibraryFolderPageDisposition};
use crate::ports::GalleryQueryRepository;

mod cache;

use super::super::gallery_snapshot::{
    set_after_query_anchor_read_hook, set_after_query_snapshot_read_hook,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[test]
fn query_snapshot_keeps_page_timeline_and_layout_in_one_read_transaction() {
    let directory = tempdir().expect("temporary catalog");
    let path = directory.path().join("catalog.sqlite3");
    let mut reader = SqliteCatalog::open(path.clone()).expect("reader");
    let location_ids: Vec<_> = (0..=MAX_LAYOUT_MANIFEST_CHUNK_ITEMS)
        .map(|index| format!("location-{index}"))
        .collect();
    let locations: Vec<_> = location_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), None, index as i64))
        .collect();
    publish_gallery_fixture(&mut reader, "initial", "root", "C:\\Fixture", &locations);
    let mut writer = SqliteCatalog::open(path).expect("independent writer");
    set_after_query_snapshot_read_hook(move || {
        publish_fixture(&mut writer, "later", "other", "C:\\Other", "third");
    });

    let result = reader
        .load_query_snapshot(1, &GalleryQuery::default(), TEST_QUERY_ID, None)
        .expect("coherent query snapshot during publication");
    assert_eq!(result.snapshot.revision, result.timeline.revision);
    assert_eq!(result.snapshot.query_id, result.timeline.query_id);
    assert_eq!(result.snapshot.assets.len(), 1);
    assert_eq!(result.timeline.total_items, location_ids.len() as u64);
    let layout = result
        .layout_manifest
        .as_ref()
        .expect("coherent compact layout");
    assert_eq!(layout.len(), 2);
    assert_eq!(layout[0].revision, result.snapshot.revision);
    assert_eq!(
        layout[0].location_ids.len(),
        MAX_LAYOUT_MANIFEST_CHUNK_ITEMS as usize
    );
    let continuation = layout[0]
        .next_cursor
        .as_ref()
        .expect("non-terminal block cursor");
    assert_eq!(
        continuation.next_ordinal,
        u64::from(MAX_LAYOUT_MANIFEST_CHUNK_ITEMS)
    );
    assert_eq!(
        continuation.after.location_id,
        *layout[0].location_ids.last().expect("last location")
    );
    assert_eq!(layout[1].start_ordinal, continuation.next_ordinal);
    assert_eq!(layout[1].location_ids, ["location-0"]);
    assert!(layout[1].next_cursor.is_none());
    let cursor = result.snapshot.next_cursor.expect("bounded cursor");
    let current = reader
        .load_query_snapshot(1, &GalleryQuery::default(), TEST_QUERY_ID, None)
        .expect("next query observes publication");
    assert!(current.snapshot.revision > result.snapshot.revision);
    assert_eq!(current.timeline.total_items, location_ids.len() as u64 + 1);
    assert_eq!(
        current
            .layout_manifest
            .as_ref()
            .expect("new layout")
            .iter()
            .map(|chunk| chunk.location_ids.len())
            .sum::<usize>(),
        location_ids.len() + 1
    );
    assert_eq!(
        load_default_snapshot(&mut reader, 1, Some(&cursor))
            .expect_err("ordinary pagination still rejects stale cursors")
            .code,
        "catalog_cursor_stale"
    );
}

#[test]
fn query_snapshot_keeps_anchor_resolution_and_page_in_one_read_transaction() {
    for uses_asset_identity in [false, true] {
        let directory = tempdir().expect("temporary catalog");
        let path = directory.path().join("catalog.sqlite3");
        let mut reader = SqliteCatalog::open(path.clone()).expect("reader");
        publish_gallery_fixture(
            &mut reader,
            "initial",
            "root",
            "C:\\Fixture",
            &[("first", None, 1), ("second", None, 2)],
        );
        let initial = load_default_snapshot(&mut reader, 2, None).expect("initial locations");
        let target = initial
            .assets
            .iter()
            .find(|asset| asset.location_id == "first")
            .expect("anchor target");
        let anchor = GalleryQueryAnchor {
            requested_location_id: target.location_id.clone(),
            group_key: None,
            asset_id: uses_asset_identity.then(|| target.asset_id.clone()),
            fallback_ordinal: 0,
        };
        let mut writer = SqliteCatalog::open(path).expect("independent writer");
        set_after_query_anchor_read_hook(move || {
            publish_gallery_fixture(
                &mut writer,
                "replacement",
                "root",
                "C:\\Fixture",
                &[("third", None, 3)],
            );
        });

        let result = reader
            .load_query_snapshot(2, &GalleryQuery::default(), TEST_QUERY_ID, Some(&anchor))
            .expect("coherent anchored snapshot during replacement");
        let resolution = result
            .snapshot
            .query_anchor_resolution
            .expect("resolved anchor");
        assert_eq!(result.snapshot.revision, initial.revision);
        assert_eq!(result.timeline.revision, initial.revision);
        assert_eq!(result.timeline.total_items, 2);
        assert_eq!(resolution.location_id.as_deref(), Some("first"));
        assert!(
            result
                .snapshot
                .assets
                .iter()
                .any(|asset| asset.location_id == "first")
        );
        let current = load_default_snapshot(&mut reader, 2, None).expect("replacement is visible");
        assert!(current.revision > initial.revision);
        assert_eq!(current.assets[0].location_id, "third");
    }
}

#[test]
fn folder_revision_change_returns_a_complete_replacement_window() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_gallery_query_fixture(
        &mut catalog,
        "initial",
        "root",
        "C:\\Fixture",
        &[
            ("album", "Album/one.png", None, None, 1),
            ("other", "Other/two.png", None, None, 2),
        ],
    );
    let first = catalog
        .load_folder_page("root", "", 1, None)
        .expect("first folder page");
    assert_eq!(first.disposition, LibraryFolderPageDisposition::Replace);
    let cursor = first.next_cursor.expect("next cursor");
    let next = catalog
        .load_folder_page("root", "", 1, Some(&cursor))
        .expect("continuation");
    assert_eq!(next.disposition, LibraryFolderPageDisposition::Append);
    publish_gallery_query_fixture(
        &mut catalog,
        "replacement",
        "root",
        "C:\\Fixture",
        &[
            ("before", "Before/new.png", None, None, 3),
            ("other", "Other/two.png", None, None, 2),
        ],
    );
    let replacement = catalog
        .load_folder_page("root", "", 1, Some(&cursor))
        .expect("new folder window");
    assert_eq!(
        replacement.disposition,
        LibraryFolderPageDisposition::Replace
    );
    assert_eq!(replacement.folders[0].relative_path, "Before");
    let fresh_cursor = replacement.next_cursor.expect("fresh continuation");
    assert_eq!(fresh_cursor.revision, replacement.revision);
    assert!(fresh_cursor.revision > cursor.revision);
}
