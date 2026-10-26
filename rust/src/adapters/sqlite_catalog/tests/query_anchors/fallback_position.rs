use super::*;

#[test]
fn missing_anchor_clamps_position_and_preserves_the_centered_window() {
    let directory = tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    publish_gallery_query_fixture(
        &mut catalog,
        "positions",
        "root",
        "C:\\Fixture",
        &[
            ("a", "1.png", None, Some(1), 1),
            ("b", "2.png", None, Some(2), 2),
            ("c", "3.png", None, Some(3), 3),
            ("d", "4.png", None, Some(4), 4),
            ("e", "5.png", None, Some(5), 5),
            ("f", "6.png", None, Some(6), 6),
            ("g", "7.png", None, Some(7), 7),
            ("h", "8.png", None, Some(8), 8),
            ("i", "9.png", None, Some(9), 9),
        ],
    );
    for sort_key in [GallerySortKey::FileName, GallerySortKey::ModifiedTime] {
        for (direction, expected) in [
            (
                GallerySortDirection::Ascending,
                ["a", "b", "c", "d", "e", "f", "g", "h", "i"],
            ),
            (
                GallerySortDirection::Descending,
                ["i", "h", "g", "f", "e", "d", "c", "b", "a"],
            ),
        ] {
            let query = GalleryQuery {
                sort_key: sort_key.clone(),
                sort_direction: direction,
                ..GalleryQuery::default()
            };
            for limit in [1, 4] {
                for requested in [0, 1, 5, 8, 9, u64::MAX] {
                    let page = catalog
                        .load_snapshot_around_asset(
                            limit,
                            &query,
                            "positions",
                            "removed",
                            "removed-asset",
                            requested,
                        )
                        .unwrap();
                    let ordinal = requested.min(8) as usize;
                    let start = ordinal.saturating_sub(limit as usize / 2);
                    let end = (start + limit as usize).min(expected.len());
                    let resolution = page.query_anchor_resolution.unwrap();
                    assert_eq!(resolution.requested_location_id, "removed");
                    assert_eq!(resolution.location_id.as_deref(), Some(expected[ordinal]));
                    assert_eq!(resolution.ordinal, Some(ordinal as u64));
                    assert_eq!(resolution.window_start_ordinal, start as u64);
                    assert_eq!(
                        page.assets
                            .iter()
                            .map(|asset| asset.location_id.as_str())
                            .collect::<Vec<_>>(),
                        expected[start..end],
                    );
                    assert_eq!(page.previous_cursor.is_some(), start > 0);
                    assert_eq!(page.next_cursor.is_some(), end < expected.len());
                }
            }
            let empty = GalleryQuery {
                search_text: "absent-image".into(),
                ..query
            };
            for requested in [0, u64::MAX] {
                let page = catalog
                    .load_snapshot_around_asset(
                        4,
                        &empty,
                        "empty",
                        "removed",
                        "removed-asset",
                        requested,
                    )
                    .unwrap();
                assert!(page.assets.is_empty());
                let resolution = page.query_anchor_resolution.unwrap();
                assert_eq!(resolution.location_id, None);
                assert_eq!(resolution.ordinal, None);
                assert_eq!(resolution.window_start_ordinal, 0);
            }
        }
    }
}
