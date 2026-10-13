use std::fs;
use std::path::PathBuf;

use image::{Rgb, RgbImage};
use tempfile::{TempDir, tempdir};

use crate::adapters::locate_library_root;
use crate::application::{StoragePaths, run_scan_with_storage};
use crate::domain::{AssetLocationView, GalleryQuery, ScanRequest};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue};

use super::*;

mod inventory;

pub(super) mod cost;

struct Fixture {
    _directory: TempDir,
    original: PathBuf,
    moved: PathBuf,
    storage: StoragePaths,
    root: IncrementalCatalogRoot,
    before: Vec<AssetLocationView>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let original = directory.path().join("original");
        let moved = directory.path().join("renamed");
        fs::create_dir(&original).unwrap();
        fs::create_dir(original.join("历史目录")).unwrap();
        for file in [original.join("one.png"), original.join("历史目录/two.png")] {
            RgbImage::from_pixel(6, 4, Rgb([30, 100, 170]))
                .save(file)
                .unwrap();
        }
        let storage = StoragePaths {
            catalog_path: directory.path().join("derived/catalog.sqlite3"),
            preview_root: directory.path().join("derived/previews"),
            preview_budget_bytes: 1024 * 1024,
            settings_path: directory.path().join("derived/settings.sqlite3"),
        };
        run_scan_with_storage(
            ScanRequest {
                scan_id: "initial".into(),
                root_path: original.to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |_| true,
            storage.clone(),
        )
        .unwrap();
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
        let root = catalog.load_incremental_catalog_roots().unwrap().remove(0);
        let before = assets(&mut catalog);
        drop(catalog);
        fs::rename(&original, &moved).unwrap();
        Self {
            _directory: directory,
            original,
            moved,
            storage,
            root,
            before,
        }
    }

    fn located(&self) -> LocatedLibraryRoot {
        locate_library_root(
            &self.root.root_path,
            self.root.publication_root_identity.as_ref().unwrap(),
        )
        .unwrap()
        .unwrap()
    }

    fn catalog(&self) -> SqliteCatalog {
        SqliteCatalog::open(self.storage.catalog_path.clone()).unwrap()
    }
}

fn assets(catalog: &mut SqliteCatalog) -> Vec<AssetLocationView> {
    catalog
        .load_snapshot(
            100,
            &GalleryQuery::default(),
            "location-recovery",
            None,
            None,
            None,
        )
        .unwrap()
        .assets
}

#[test]
fn root_location_recovery_keeps_assets_and_durable_gap_across_reopen() {
    let fixture = Fixture::new();
    let located = fixture.located();
    let mut catalog = fixture.catalog();
    catalog
        .recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        .unwrap();
    drop(catalog);
    let mut catalog = fixture.catalog();
    let current = catalog
        .load_incremental_catalog_root(&fixture.root.root_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.root_path, located.path());
    assert_eq!(
        current.root_generation.value(),
        fixture.root.root_generation.value() + 1
    );
    assert_eq!(current.active_scan_id, fixture.root.active_scan_id);
    assert_eq!(
        current.publication_root_identity,
        fixture.root.publication_root_identity
    );
    assert_eq!(current.catalog_revision, fixture.root.catalog_revision + 1);
    let after = assets(&mut catalog);
    assert_eq!(after.len(), fixture.before.len());
    for (before, after) in fixture.before.iter().zip(after.iter()) {
        assert_eq!(before.asset_id, after.asset_id);
        assert_eq!(before.location_id, after.location_id);
        assert_eq!(before.file_identity, after.file_identity);
        assert_eq!(before.source_revision, after.source_revision);
        assert_eq!(before.source_generation, after.source_generation);
        assert_eq!(before.preview_path, after.preview_path);
        assert_eq!(before.relative_path, after.relative_path);
        assert_eq!(
            fs::read(&after.absolute_path).unwrap(),
            fs::read(fixture.moved.join(&after.relative_path)).unwrap()
        );
        assert!(!Path::new(&after.absolute_path).starts_with(&fixture.original));
    }
    let metrics = catalog
        .load_library_change_root_queue_metrics(
            &current.root_id,
            current.root_generation,
            unix_time_ms(),
            LibraryChangeQueuePolicy::default(),
        )
        .unwrap();
    assert!(metrics.freshness_unknown_count > 0);
    assert!(!current.has_running_scan);
}

#[test]
fn root_location_recovery_refuses_stale_or_cancelled_transition() {
    let fixture = Fixture::new();
    let located = fixture.located();
    let mut catalog = fixture.catalog();
    let cancelled = catalog
        .recover_located_root(&fixture.root, &located, &AtomicBool::new(true))
        .unwrap_err();
    assert_eq!(cancelled.code, "root_location_recovery_cancelled");
    let mut stale = fixture.root.clone();
    stale.root_generation = stale.root_generation.next().unwrap();
    assert_eq!(
        catalog
            .recover_located_root(&stale, &located, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "catalog_root_relocation_stale"
    );
    assert_eq!(
        catalog
            .load_incremental_catalog_root(&fixture.root.root_id)
            .unwrap()
            .unwrap(),
        fixture.root
    );
}

#[test]
fn root_location_recovery_rolls_back_after_unsafe_location() {
    let fixture = Fixture::new();
    let located = fixture.located();
    let mut catalog = fixture.catalog();
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET relative_path = '../outside.png' WHERE location_id = ?1",
            [&fixture.before[0].location_id],
        )
        .unwrap();
    assert_eq!(
        catalog
            .recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "root_location_recovery_conflict"
    );
    assert_eq!(
        catalog
            .load_incremental_catalog_root(&fixture.root.root_id)
            .unwrap()
            .unwrap(),
        fixture.root
    );
    let metrics = catalog
        .load_library_change_root_queue_metrics(
            &fixture.root.root_id,
            fixture.root.root_generation,
            unix_time_ms(),
            LibraryChangeQueuePolicy::default(),
        )
        .unwrap();
    assert_eq!(metrics.freshness_unknown_count, 0);
}

#[test]
fn root_location_recovery_cannot_bind_an_assembled_directory() {
    let fixture = Fixture::new();
    let combined = fixture._directory.path().join("combined");
    fs::create_dir(&combined).unwrap();
    fs::rename(fixture.moved.join("one.png"), combined.join("one.png")).unwrap();
    let identity = crate::adapters::file_identity_evidence(&combined)
        .unwrap()
        .unwrap();
    let candidate = locate_library_root(&combined.to_string_lossy(), &identity)
        .unwrap()
        .unwrap();
    let mut catalog = fixture.catalog();
    assert!(
        catalog
            .recover_located_root(&fixture.root, &candidate, &AtomicBool::new(false))
            .is_err()
    );
    assert_eq!(
        catalog
            .load_incremental_catalog_root(&fixture.root.root_id)
            .unwrap()
            .unwrap(),
        fixture.root
    );
}
