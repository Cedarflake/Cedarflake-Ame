use crate::domain::organization_queue::OrganizationQueue;

use super::fingerprint_cost::bounded_read;
use super::*;

#[test]
fn collection_and_queue_reads_skip_unpublished_source_history() {
    let (_directory, mut catalog, album) = grouped();
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "INSERT INTO scan_runs(id, root_id, status, started_unix_ms, completed_unix_ms, preview_edge)
             VALUES('unpublished-history', 'root-a', 'cancelled', 0, 1, 512);
             WITH RECURSIVE sequence(value) AS (
               VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 25000
             )
             INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id,
               absolute_path, relative_path, preview_path, file_size, modified_unix_ms,
               width, height, preview_status, source_generation)
             SELECT 'unpublished-history', current.asset_id, 'history-' || value, current.root_id,
               current.absolute_path, current.relative_path, '', current.file_size, 0,
               40, 50, 'pending', current.source_generation
             FROM sequence CROSS JOIN asset_locations AS current
             WHERE current.scan_id = 'scan-a' AND current.location_id = 'a';
             INSERT INTO asset_locations(scan_id, asset_id, location_id, root_id,
               absolute_path, relative_path, preview_path, file_size, modified_unix_ms,
               width, height, preview_status, source_generation)
             SELECT 'unpublished-history', asset_id, location_id, root_id,
               absolute_path, relative_path, '', file_size, 0,
               40, 50, 'pending', source_generation
             FROM asset_locations WHERE scan_id = 'scan-a' AND location_id = 'a';",
        )
        .unwrap();
    for (query, expected_ids, physical) in [
        (album.clone(), vec!["a", "b"], 3),
        (
            GalleryQuery {
                collection: GalleryCollection::Favorites,
                ..album.clone()
            },
            vec!["a", "c"],
            2,
        ),
        (
            GalleryQuery {
                collection: GalleryCollection::Favorites,
                duplicate_mode: GalleryDuplicateMode::Merged,
                ..album.clone()
            },
            vec!["a"],
            2,
        ),
        (
            GalleryQuery {
                root_id: Some("root-c".into()),
                ..album.clone()
            },
            vec!["c"],
            1,
        ),
        (
            GalleryQuery {
                organization_queue: Some(OrganizationQueue::Pending),
                ..GalleryQuery::default()
            },
            vec![],
            0,
        ),
    ] {
        let snapshot = bounded_read(&mut catalog, |catalog| {
            catalog.load_query_snapshot(10, &query, "published-scope", None)
        });
        let actual_ids = snapshot
            .snapshot
            .assets
            .iter()
            .map(|asset| asset.location_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(actual_ids, expected_ids);
        assert_eq!(snapshot.timeline.total_items, expected_ids.len() as u64);
        assert_eq!(snapshot.timeline.total_source_items, physical);
        let target = selected(&catalog, "a");
        let anchored = bounded_read(&mut catalog, |catalog| {
            catalog.load_snapshot_around_asset(
                10,
                &query,
                "published-scope",
                "a",
                &target.asset_id,
                0,
            )
        });
        assert_eq!(
            anchored
                .assets
                .iter()
                .map(|asset| asset.location_id.as_str())
                .collect::<Vec<_>>(),
            expected_ids,
        );
        assert_eq!(
            anchored
                .query_anchor_resolution
                .unwrap()
                .location_id
                .as_deref(),
            expected_ids.first().copied(),
        );
        let layout = bounded_read(&mut catalog, |catalog| {
            catalog.load_gallery_layout_manifest_chunk(10, &query, "published-scope", None)
        });
        assert_eq!(layout.location_ids, expected_ids);
        assert_eq!(layout.total_items, snapshot.timeline.total_items);
        if let Some(counts) = snapshot.organization_counts {
            assert_eq!((counts.total, counts.organized, counts.pending), (2, 2, 0));
        }
    }
}
