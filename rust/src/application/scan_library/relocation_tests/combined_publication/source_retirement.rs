use crate::application::preview::materialize_preview_with_storage;
use crate::domain::{PreviewRequest, PreviewStatus};

use super::*;

#[test]
fn completed_combination_can_retire_both_old_sources_without_ghosts_or_losing_previews() {
    let mut fixture = CombinedFixture::new();
    for path in [
        fixture.base.original.join("unmoved.png"),
        fixture.peer_path.join("unmoved.png"),
        fixture.peer_path.join("historical.png"),
    ] {
        fs::remove_file(path).unwrap();
    }
    fs::remove_dir(&fixture.base.original).unwrap();
    fs::remove_dir(&fixture.peer_path).unwrap();
    fixture.bytes = [&fixture.moved_path, &fixture.copied_path]
        .into_iter()
        .map(|path| (path.clone(), fs::read(path).unwrap()))
        .collect();

    fixture.scan("replacement-complete");
    let published = fixture.base.snapshot();
    fixture.assert_old_publications_retained(&published);
    assert_eq!(published.assets.len(), 6);
    let replacement = published
        .roots
        .iter()
        .find(|root| root.active_scan_id.as_deref() == Some("replacement-complete"))
        .unwrap();
    let mut ready = Vec::new();
    for location in published
        .assets
        .iter()
        .filter(|location| location.root_id == replacement.root_id)
    {
        let preview = materialize_preview_with_storage(
            PreviewRequest {
                location_id: location.location_id.clone(),
                expected_root_id: location.root_id.clone(),
                expected_scan_id: location.scan_id.clone(),
                expected_source_revision: location.source_revision.clone(),
                expected_source_generation: location.source_generation,
                preview_edge: 256,
                retry_failed: false,
                protected_location_ids: Vec::new(),
            },
            fixture.base.storage.clone(),
        )
        .unwrap();
        assert!(matches!(preview.preview_status, PreviewStatus::Ready));
        ready.push(preview);
    }
    assert_eq!(ready.len(), 2);
    let preview_bytes: Vec<_> = ready
        .iter()
        .map(|location| fs::read(&location.preview_path).unwrap())
        .collect();

    for (index, old_root) in fixture.original_snapshot.roots.iter().enumerate() {
        let mut catalog = SqliteCatalog::open(fixture.base.storage.catalog_path.clone()).unwrap();
        assert!(catalog.unregister_root(&old_root.root_id).unwrap());
        assert!(!catalog.unregister_root(&old_root.root_id).unwrap());
        drop(catalog);
        let reopened = fixture.base.snapshot();
        assert_eq!(reopened.roots.len(), 2 - index);
        assert!(
            reopened
                .roots
                .iter()
                .all(|root| root.root_id != old_root.root_id)
        );
        assert!(
            reopened
                .assets
                .iter()
                .all(|location| location.root_id != old_root.root_id)
        );
        for expected in &ready {
            let retained = reopened
                .assets
                .iter()
                .find(|location| location.location_id == expected.location_id)
                .unwrap();
            assert_eq!(retained.asset_id, expected.asset_id);
            assert_eq!(retained.file_identity, expected.file_identity);
            assert_eq!(retained.source_revision, expected.source_revision);
            assert_eq!(retained.source_generation, expected.source_generation);
            assert_eq!(retained.preview_path, expected.preview_path);
            assert!(matches!(retained.preview_status, PreviewStatus::Ready));
        }
    }

    let mut catalog = SqliteCatalog::open(fixture.base.storage.catalog_path.clone()).unwrap();
    let snapshot = fixture.base.snapshot();
    assert_eq!(snapshot.assets.len(), 2);
    assert_eq!(snapshot.roots[0].asset_count, 2);
    let query = GalleryQuery::default();
    let timeline = catalog
        .load_gallery_timeline(&query, "retired-sources")
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
    let layout = catalog
        .load_gallery_layout_manifest_chunk(100, &query, "retired-sources", None)
        .unwrap();
    assert_eq!(layout.total_items, 2);
    assert_eq!(layout.location_ids.len(), 2);
    assert!(
        layout
            .location_ids
            .iter()
            .all(|id| ready.iter().any(|location| &location.location_id == id))
    );
    let monitored_roots = catalog.load_incremental_catalog_roots().unwrap();
    assert_eq!(monitored_roots.len(), 1);
    assert_eq!(monitored_roots[0].root_id, replacement.root_id);
    for (location, bytes) in ready.iter().zip(preview_bytes) {
        assert_eq!(fs::read(&location.preview_path).unwrap(), bytes);
    }
    for (path, bytes) in &fixture.bytes {
        assert_eq!(&fs::read(path).unwrap(), bytes);
    }
}
