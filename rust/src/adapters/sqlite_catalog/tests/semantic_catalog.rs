use crate::adapters::SqliteCatalog;
use crate::domain::GalleryQuery;
use crate::ports::semantic_catalog::SemanticCatalog;

use super::support::publish_gallery_query_fixture;

#[test]
fn semantic_candidate_window_does_not_sort_the_remaining_library() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    for root in 0..2 {
        let ids: Vec<_> = (0..2048)
            .map(|index| format!("image-{index:04}-{root}.png"))
            .collect();
        let locations: Vec<_> = ids
            .iter()
            .map(|id| (id.as_str(), id.as_str(), None, None, 1))
            .collect();
        publish_gallery_query_fixture(
            &mut catalog,
            &format!("scan-{root}"),
            &format!("root-{root}"),
            &format!("C:/Fixture/{root}"),
            &locations,
        );
    }
    let steps = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&steps);
    catalog
        .connection
        .progress_handler(
            1000,
            Some(move || {
                observed.fetch_add(1000, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    let page = catalog
        .semantic_candidates(&GalleryQuery::default(), "")
        .unwrap();
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    assert_eq!(page.candidates.len(), 256);
    assert_eq!(page.candidates[0].source.location_id, "image-0000-0.png");
    assert_eq!(page.candidates[255].source.location_id, "image-0127-1.png");
    let work = steps.load(Ordering::Relaxed);
    assert!(
        work < 50_000,
        "A 256-item page must not sort 4096 candidates: {work}"
    );
}

#[test]
fn semantic_gallery_pages_and_position_seeks_keep_relevance_order() {
    use crate::domain::semantic_results::SemanticResultManifest;
    use crate::domain::{GallerySortKey, GalleryTimeAnchor};
    use crate::ports::CatalogRepository;

    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let ids: Vec<_> = (0..12)
        .map(|index| format!("image-{index:02}.png"))
        .collect();
    let locations: Vec<_> = ids
        .iter()
        .map(|id| (id.as_str(), id.as_str(), None, None, 1))
        .collect();
    publish_gallery_query_fixture(&mut catalog, "scan", "root", "C:/Fixture", &locations);
    let candidates = catalog
        .semantic_candidates(&GalleryQuery::default(), "")
        .unwrap();
    let ranking = [8, 1, 10, 3, 6, 0, 11, 2, 9, 4, 7, 5];
    let manifest = SemanticResultManifest {
        id: "a".repeat(64),
        catalog_revision: candidates.revision,
        feature_generation: 1,
        eligible_items: 12,
        indexed_items: 12,
        storage_full: false,
        items: ranking
            .iter()
            .map(|index| candidates.candidates[*index].source.clone())
            .collect(),
    };
    for sort_key in [
        GallerySortKey::CaptureTime,
        GallerySortKey::FileName,
        GallerySortKey::ModifiedTime,
    ] {
        let query = GalleryQuery {
            semantic_results: Some(manifest.clone()),
            sort_key,
            ..Default::default()
        };
        let timeline = catalog
            .load_gallery_timeline(&query, "semantic-query")
            .unwrap();
        assert_eq!(timeline.total_items, 12);
        assert!(timeline.scroll_sections.is_empty());
        assert_eq!(timeline.buckets.len(), 1);
        assert_eq!(timeline.buckets[0].month_key, None);
        for start in [0, 7] {
            let anchor = GalleryTimeAnchor {
                revision: timeline.revision,
                query_id: "semantic-query".to_owned(),
                month_key: None,
                item_offset: start,
            };
            let page = catalog
                .load_snapshot(3, &query, "semantic-query", None, None, Some(&anchor))
                .unwrap();
            let expected: Vec<_> = ranking[start as usize..start as usize + 3]
                .iter()
                .map(|index| ids[*index].as_str())
                .collect();
            assert_eq!(
                page.assets
                    .iter()
                    .map(|asset| asset.location_id.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
            if start == 0 {
                let next = catalog
                    .load_snapshot(
                        3,
                        &query,
                        "semantic-query",
                        page.next_cursor.as_ref(),
                        None,
                        None,
                    )
                    .unwrap();
                assert_eq!(
                    next.assets
                        .iter()
                        .map(|asset| asset.location_id.as_str())
                        .collect::<Vec<_>>(),
                    ranking[3..6]
                        .iter()
                        .map(|index| ids[*index].as_str())
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn semantic_snapshot_keeps_its_scope_while_background_publication_advances() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let ids: Vec<_> = (0..257)
        .map(|index| format!("image-{index:03}.png"))
        .collect();
    let locations: Vec<_> = ids
        .iter()
        .map(|id| (id.as_str(), id.as_str(), None, None, 1))
        .collect();
    publish_gallery_query_fixture(&mut catalog, "old", "root", "C:/Fixture", &locations);
    let mut writer = SqliteCatalog::open(path).unwrap();
    let snapshot = catalog.semantic_snapshot().unwrap();
    let first = snapshot.candidates(&GalleryQuery::default(), "").unwrap();
    assert_eq!(first.candidates.len(), 256);
    assert_eq!(
        first.candidates.last().unwrap().source.location_id,
        "image-255.png"
    );

    publish_gallery_query_fixture(
        &mut writer,
        "new",
        "root",
        "C:/Fixture",
        &[("new.png", "new.png", None, None, 2)],
    );
    let last = snapshot
        .candidates(&GalleryQuery::default(), "image-255.png")
        .unwrap();
    assert_eq!(last.revision, first.revision);
    assert_eq!(last.candidates.len(), 1);
    assert_eq!(last.candidates[0].source.location_id, "image-256.png");
    let mut streamed = Vec::new();
    let mut sizes = Vec::new();
    let revision = snapshot
        .scan(&GalleryQuery::default(), &mut |window| {
            sizes.push(window.len());
            streamed.extend(
                window
                    .into_iter()
                    .map(|candidate| candidate.source.location_id),
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(sizes, [256, 1]);
    assert_eq!(streamed, ids);
    assert_eq!(revision, first.revision);
    drop(snapshot);

    let current = catalog
        .semantic_candidates(&GalleryQuery::default(), "")
        .unwrap();
    assert!(current.revision > first.revision);
    assert_eq!(current.candidates.len(), 1);
    assert_eq!(current.candidates[0].source.location_id, "new.png");
}
