use super::*;

struct CombinedFixture {
    base: Fixture,
    original_snapshot: CatalogSnapshot,
    moved_path: PathBuf,
    copied_path: PathBuf,
    peer_path: PathBuf,
    bytes: Vec<(PathBuf, Vec<u8>)>,
}

impl CombinedFixture {
    fn new() -> Self {
        let base = Fixture::new();
        let peer_path = base._source.path().join("second-source");
        fs::create_dir(&peer_path).unwrap();
        for source in [&base.original, &peer_path] {
            RgbImage::from_pixel(9, 7, Rgb([190, 60, 50]))
                .save(source.join("unmoved.png"))
                .unwrap();
        }
        fs::copy(
            base.original.join("historical.png"),
            peer_path.join("historical.png"),
        )
        .unwrap();
        run_scan_with_storage(
            request("first-complete", &base.original),
            |_| true,
            base.storage.clone(),
        )
        .unwrap();
        run_scan_with_storage(
            request("peer-complete", &peer_path),
            |_| true,
            base.storage.clone(),
        )
        .unwrap();
        let original_snapshot = base.snapshot();
        assert_eq!(original_snapshot.roots.len(), 2);
        assert_eq!(original_snapshot.assets.len(), 4);

        let moved_path = base.replacement.join("from-first/historical.png");
        let copied_path = base.replacement.join("from-second/historical.png");
        fs::create_dir_all(moved_path.parent().unwrap()).unwrap();
        fs::create_dir_all(copied_path.parent().unwrap()).unwrap();
        fs::rename(base.original.join("historical.png"), &moved_path).unwrap();
        fs::copy(peer_path.join("historical.png"), &copied_path).unwrap();
        let bytes = [
            base.original.join("unmoved.png"),
            peer_path.join("unmoved.png"),
            peer_path.join("historical.png"),
            moved_path.clone(),
            copied_path.clone(),
        ]
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
        Self {
            base,
            original_snapshot,
            moved_path,
            copied_path,
            peer_path,
            bytes,
        }
    }

    fn assert_old_publications_retained(&self, snapshot: &CatalogSnapshot) {
        for before in &self.original_snapshot.roots {
            let after = snapshot
                .roots
                .iter()
                .find(|root| root.root_id == before.root_id)
                .unwrap();
            assert_eq!(after.path, before.path);
            assert_eq!(after.active_scan_id, before.active_scan_id);
        }
        for before in &self.original_snapshot.assets {
            let after = snapshot
                .assets
                .iter()
                .find(|asset| asset.location_id == before.location_id)
                .unwrap();
            assert_eq!(after.asset_id, before.asset_id);
            assert_eq!(after.scan_id, before.scan_id);
            assert_eq!(after.absolute_path, before.absolute_path);
            assert_eq!(after.file_identity, before.file_identity);
        }
        for (path, bytes) in &self.bytes {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
    }

    fn scan(&self, id: &str) {
        run_scan_with_storage(
            request(id, &self.base.replacement),
            |_| true,
            self.base.storage.clone(),
        )
        .unwrap();
    }
}

#[test]
fn combined_import_cancellation_preserves_old_publications_and_reopened_retry() {
    let fixture = CombinedFixture::new();
    let mut staged_before_cancel = false;
    let mut cancelled = false;
    run_scan_with_storage(
        request("combined-cancelled", &fixture.base.replacement),
        |event| {
            if let ScanEvent::Finalizing {
                scan_id,
                total_items,
                ..
            } = &event
            {
                assert_eq!(*total_items, 2);
                staged_before_cancel = true;
                assert!(cancel_scan(scan_id));
            }
            cancelled |= matches!(event, ScanEvent::Cancelled { .. });
            true
        },
        fixture.base.storage.clone(),
    )
    .unwrap();
    assert!(staged_before_cancel && cancelled);
    let cancelled_snapshot = fixture.base.snapshot();
    fixture.assert_old_publications_retained(&cancelled_snapshot);
    assert_eq!(cancelled_snapshot.assets.len(), 4);
    assert!(
        cancelled_snapshot
            .roots
            .iter()
            .all(|root| root.active_scan_id.as_deref() != Some("combined-cancelled"))
    );

    fixture.scan("combined-retry");
    let published = fixture.base.snapshot();
    fixture.assert_old_publications_retained(&published);
    assert_eq!(published.roots.len(), 3);
    assert_eq!(published.assets.len(), 6);
    let old_moved = fixture
        .original_snapshot
        .assets
        .iter()
        .find(|asset| {
            asset.root_id == fixture.base.root.root_id && asset.relative_path == "historical.png"
        })
        .unwrap();
    let moved = published
        .assets
        .iter()
        .find(|asset| {
            asset.scan_id == "combined-retry" && asset.relative_path == "from-first/historical.png"
        })
        .unwrap();
    assert_eq!(moved.asset_id, old_moved.asset_id);
    assert_eq!(moved.file_identity, old_moved.file_identity);
    let copied = published
        .assets
        .iter()
        .find(|asset| {
            asset.scan_id == "combined-retry" && asset.relative_path == "from-second/historical.png"
        })
        .unwrap();
    assert!(fixture.original_snapshot.assets.iter().all(|old| old.asset_id != copied.asset_id && old.file_identity != copied.file_identity));
}

#[test]
fn reorganized_destination_keeps_identity_boundaries_when_both_original_roots_are_gone() {
    let mut fixture = CombinedFixture::new();
    let organized = fixture.base.replacement.join("整理/2026");
    fs::create_dir_all(&organized).unwrap();
    let moved = organized.join("renamed.png");
    let changed_identity = organized.join("different-volume-observation.png");
    fs::rename(&fixture.moved_path, &moved).unwrap();
    fs::rename(&fixture.copied_path, &changed_identity).unwrap();
    for path in [
        fixture.base.original.join("unmoved.png"),
        fixture.peer_path.join("unmoved.png"),
        fixture.peer_path.join("historical.png"),
    ] {
        fs::remove_file(path).unwrap();
    }
    fs::remove_dir(&fixture.base.original).unwrap();
    fs::remove_dir(&fixture.peer_path).unwrap();
    fixture.bytes = [&moved, &changed_identity]
        .into_iter()
        .map(|path| (path.clone(), fs::read(path).unwrap()))
        .collect();

    fixture.scan("reorganized-complete");
    let published = fixture.base.snapshot();
    fixture.assert_old_publications_retained(&published);
    assert_eq!(published.roots.len(), 3);
    assert_eq!(published.assets.len(), 6);
    let new_locations: Vec<_> = published
        .assets
        .iter()
        .filter(|asset| asset.scan_id == "reorganized-complete")
        .collect();
    assert_eq!(new_locations.len(), 2);
    let retained_identity = new_locations
        .iter()
        .find(|asset| asset.relative_path == "整理/2026/renamed.png")
        .unwrap();
    let original = fixture
        .original_snapshot
        .assets
        .iter()
        .find(|asset| {
            asset.root_id == fixture.base.root.root_id && asset.relative_path == "historical.png"
        })
        .unwrap();
    assert_eq!(retained_identity.asset_id, original.asset_id);
    assert_eq!(retained_identity.file_identity, original.file_identity);
    let new_identity = new_locations
        .iter()
        .find(|asset| asset.relative_path == "整理/2026/different-volume-observation.png")
        .unwrap();
    assert!(fixture.original_snapshot.assets.iter().all(|old| {
        old.asset_id != new_identity.asset_id && old.file_identity != new_identity.file_identity
    }));
    assert!(!fixture.base.original.exists());
    assert!(!fixture.peer_path.exists());
}

#[test]
fn combined_incomplete_update_keeps_snapshot_and_equal_byte_replacement_gets_new_identity() {
    let fixture = CombinedFixture::new();
    fixture.scan("combined-initial");
    let before = fixture.base.snapshot();
    let copied_before = before
        .assets
        .iter()
        .find(|asset| {
            asset.scan_id == "combined-initial"
                && asset.relative_path == "from-second/historical.png"
        })
        .unwrap();
    let previous_copy = fixture.base._source.path().join("retained-old-copy.png");
    let modified = fs::metadata(&fixture.copied_path)
        .unwrap()
        .modified()
        .unwrap();
    fs::rename(&fixture.copied_path, &previous_copy).unwrap();
    fs::copy(
        fixture.peer_path.join("historical.png"),
        &fixture.copied_path,
    )
    .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&fixture.copied_path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert_eq!(
        fs::read(&previous_copy).unwrap(),
        fs::read(&fixture.copied_path).unwrap()
    );

    let mut limited_request = request("combined-incomplete", &fixture.base.replacement);
    limited_request.max_items = Some(1);
    let mut stale = false;
    run_scan_with_storage(
        limited_request,
        |event| {
            stale |= matches!(event, ScanEvent::Stale { .. });
            true
        },
        fixture.base.storage.clone(),
    )
    .unwrap();
    assert!(stale);
    let limited = fixture.base.snapshot();
    fixture.assert_old_publications_retained(&limited);
    assert_eq!(limited.assets.len(), before.assets.len());
    assert_eq!(limited.roots.len(), before.roots.len());
    for root in &before.roots {
        let retained = limited
            .roots
            .iter()
            .find(|retained| retained.root_id == root.root_id)
            .unwrap();
        assert_eq!(retained.path, root.path);
        assert_eq!(retained.active_scan_id, root.active_scan_id);
    }
    assert!(limited.assets.iter().any(
        |asset| asset.asset_id == copied_before.asset_id && asset.scan_id == "combined-initial"
    ));

    fixture.scan("combined-complete");
    let complete = fixture.base.snapshot();
    fixture.assert_old_publications_retained(&complete);
    let copied_after = complete
        .assets
        .iter()
        .find(|asset| {
            asset.scan_id == "combined-complete"
                && asset.relative_path == "from-second/historical.png"
        })
        .unwrap();
    assert_ne!(copied_after.file_identity, copied_before.file_identity);
    assert_ne!(copied_after.asset_id, copied_before.asset_id);
    assert_ne!(
        copied_after.source_generation,
        copied_before.source_generation
    );
    assert_eq!(copied_after.file_size, copied_before.file_size);
    assert_eq!(
        copied_after.modified_unix_ms,
        copied_before.modified_unix_ms
    );
    assert_eq!(
        fs::read(&fixture.moved_path).unwrap(),
        fs::read(&fixture.copied_path).unwrap()
    );
}
