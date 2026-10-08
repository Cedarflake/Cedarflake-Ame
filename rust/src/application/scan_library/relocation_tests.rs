use std::fs;
use std::path::{Path, PathBuf};

use image::{Rgb, RgbImage};
use tempfile::{TempDir, tempdir};

use crate::domain::{CatalogSnapshot, GalleryQuery, IncrementalCatalogRoot};

use super::*;

mod combined_publication;
mod discovered_roots;
mod identity_admission;
mod identity_audit;

struct Fixture {
    _source: TempDir,
    _derived: TempDir,
    original: PathBuf,
    replacement: PathBuf,
    storage: StoragePaths,
    root: IncrementalCatalogRoot,
}

impl Fixture {
    fn new() -> Self {
        let source = tempdir().expect("source fixture");
        let derived = tempdir().expect("derived fixture");
        let original = source.path().join("原始图库");
        let replacement = source.path().join("新位置");
        fs::create_dir(&original).expect("original directory");
        RgbImage::from_pixel(8, 6, Rgb([10, 90, 170]))
            .save(original.join("historical.png"))
            .expect("source image");
        let storage = StoragePaths {
            catalog_path: derived.path().join("catalog.sqlite3"),
            preview_root: derived.path().join("previews"),
            preview_budget_bytes: 1024 * 1024,
            settings_path: derived.path().join("settings.sqlite3"),
        };
        run_scan_with_storage(request("initial", &original), |_| true, storage.clone())
            .expect("initial scan");
        let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        let root = catalog
            .load_incremental_catalog_roots()
            .expect("roots")
            .remove(0);
        drop(catalog);
        Self {
            _source: source,
            _derived: derived,
            original,
            replacement,
            storage,
            root,
        }
    }

    fn relocate(
        &self,
        scan_id: &str,
        publish: impl FnMut(ScanEvent) -> bool,
    ) -> Result<(), ScanError> {
        run_scan_with_storage_selection(
            request(scan_id, &self.replacement),
            publish,
            || Ok(self.storage.clone()),
            FullScanReason::ExplicitUserRequest,
            ScanRootSelection::Relocate {
                root_id: self.root.root_id.clone(),
                expected_path: self.root.root_path.clone(),
            },
        )
    }

    fn snapshot(&self) -> CatalogSnapshot {
        SqliteCatalog::open(self.storage.catalog_path.clone())
            .expect("reopen catalog")
            .load_snapshot(
                100,
                &GalleryQuery::default(),
                "relocation",
                None,
                None,
                None,
            )
            .expect("snapshot")
    }

    fn current_root(&self) -> IncrementalCatalogRoot {
        SqliteCatalog::open(self.storage.catalog_path.clone())
            .expect("catalog")
            .load_incremental_catalog_root(&self.root.root_id)
            .expect("root")
            .expect("retained root")
    }
}

fn request(scan_id: &str, path: &Path) -> ScanRequest {
    ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: path.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    }
}

#[test]
fn renamed_root_preserves_identity_and_later_ordinary_updates_keep_one_root() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    fs::rename(&fixture.original, &fixture.replacement).expect("rename generated root");
    fixture.relocate("relocated", |_| true).expect("relocation");
    let relocated = fixture.snapshot();
    assert_eq!(relocated.roots.len(), 1);
    assert_eq!(relocated.roots[0].root_id, fixture.root.root_id);
    assert_eq!(relocated.assets[0].asset_id, before.assets[0].asset_id);
    assert_eq!(
        relocated.assets[0].location_id,
        before.assets[0].location_id
    );
    assert!(
        Path::new(&relocated.assets[0].absolute_path)
            .starts_with(fixture.replacement.canonicalize().unwrap())
    );
    let root = fixture.current_root();
    assert!(root.root_generation > fixture.root.root_generation);
    run_scan_with_storage(
        request("ordinary", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("ordinary update");
    assert_eq!(fixture.snapshot().roots.len(), 1);
    assert_eq!(fixture.current_root().root_id, fixture.root.root_id);
    assert_eq!(fixture.current_root().root_generation, root.root_generation);
}

#[test]
fn replacement_files_do_not_inherit_unproven_asset_or_source_identity() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    fs::create_dir(&fixture.replacement).expect("new directory");
    fs::copy(
        fixture.original.join("historical.png"),
        fixture.replacement.join("historical.png"),
    )
    .expect("copy generated media");
    fixture.relocate("copied", |_| true).expect("relocation");
    let after = fixture.snapshot();
    assert_eq!(after.roots.len(), 1);
    assert_eq!(after.roots[0].root_id, fixture.root.root_id);
    assert_ne!(
        after.assets[0].file_identity,
        before.assets[0].file_identity
    );
    assert_ne!(after.assets[0].asset_id, before.assets[0].asset_id);
    assert_ne!(
        after.assets[0].source_generation,
        before.assets[0].source_generation
    );
    assert_eq!(
        fs::read(fixture.original.join("historical.png")).unwrap(),
        fs::read(fixture.replacement.join("historical.png")).unwrap()
    );
}

#[test]
fn reusing_the_old_path_registers_a_distinct_root_without_reverting_relocation() {
    let fixture = Fixture::new();
    fs::rename(&fixture.original, &fixture.replacement).expect("rename generated root");
    fixture.relocate("relocated", |_| true).expect("relocation");
    let relocated = fixture.current_root();
    fs::create_dir(&fixture.original).expect("reuse old directory name");
    RgbImage::from_pixel(4, 9, Rgb([230, 60, 30]))
        .save(fixture.original.join("new-library.png"))
        .expect("new source image");
    run_scan_with_storage(
        request("new-library", &fixture.original),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("register new source at old path");
    assert_eq!(fixture.current_root().root_path, relocated.root_path);
    assert_eq!(
        fixture.current_root().active_scan_id,
        relocated.active_scan_id
    );
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.roots.len(), 2);
    assert_eq!(snapshot.assets.len(), 2);
    assert_ne!(snapshot.roots[0].root_id, snapshot.roots[1].root_id);
}

#[test]
fn cancelled_relocation_keeps_published_snapshot_and_retry_uses_new_registered_path() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    fs::rename(&fixture.original, &fixture.replacement).expect("rename generated root");
    let mut cancelled = false;
    fixture
        .relocate("cancelled-relocation", |event| {
            if let ScanEvent::Started { scan_id, .. } = &event {
                assert!(cancel_scan(scan_id));
            }
            cancelled |= matches!(event, ScanEvent::Cancelled { .. });
            true
        })
        .expect("cancelled relocation");
    assert!(cancelled);
    let retained = fixture.snapshot();
    assert_eq!(
        retained.roots[0].active_scan_id,
        before.roots[0].active_scan_id
    );
    assert_eq!(
        retained.assets[0].absolute_path,
        before.assets[0].absolute_path
    );
    assert_eq!(
        fixture.current_root().root_path,
        fixture
            .replacement
            .canonicalize()
            .unwrap()
            .to_string_lossy()
    );
    run_scan_with_storage(
        request("retry-relocation", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("retry after reopen");
    assert_eq!(fixture.snapshot().roots.len(), 1);
    assert_eq!(
        fixture.current_root().active_scan_id.as_deref(),
        Some("retry-relocation")
    );
}

#[test]
fn interrupted_existing_root_relocation_retries_after_catalog_reopen() {
    let fixture = Fixture::new();
    fs::rename(&fixture.original, &fixture.replacement).expect("rename generated root");
    let mut cancelled = false;
    fixture
        .relocate("paused-relocation", |event| {
            if let ScanEvent::Started { scan_id, .. } = &event {
                assert!(pause_scan(scan_id));
            }
            cancelled |= matches!(event, ScanEvent::Cancelled { .. });
            true
        })
        .expect("pause relocation");
    assert!(cancelled);
    assert!(
        load_paused_scan_from_path(&fixture.storage.catalog_path)
            .unwrap()
            .is_none()
    );
    run_scan_with_storage(
        request("continued-relocation", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("restart relocated update");
    assert_eq!(fixture.snapshot().roots.len(), 1);
    assert_eq!(
        fixture.current_root().active_scan_id.as_deref(),
        Some("continued-relocation")
    );
}

#[test]
fn another_registered_destination_is_rejected_without_retiring_either_root() {
    let fixture = Fixture::new();
    fs::create_dir(&fixture.replacement).expect("peer directory");
    run_scan_with_storage(
        request("peer", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("peer scan");
    let before = fixture.current_root();
    let error = fixture
        .relocate("conflict", |_| true)
        .expect_err("registered destination");
    assert_eq!(error.code, "catalog_root_relocation_conflict");
    assert_eq!(fixture.current_root(), before);
    assert_eq!(fixture.snapshot().roots.len(), 2);
}

#[test]
fn stale_generation_and_active_scan_reject_relocation_atomically() {
    let fixture = Fixture::new();
    fs::create_dir(&fixture.replacement).expect("replacement directory");
    let discovery = FileDiscovery::new(&fixture.replacement.to_string_lossy()).unwrap();
    let path = discovery
        .canonical_root()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let identity = discovery
        .metadata_inventory_root_identity()
        .unwrap()
        .unwrap();
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    let mut stale = fixture.root.clone();
    stale.root_generation = stale.root_generation.next().unwrap();
    let error = catalog
        .begin_relocated_scan(
            &request("stale", &fixture.replacement),
            &stale,
            &path,
            &identity,
        )
        .expect_err("stale generation");
    assert_eq!(error.code, "catalog_root_relocation_stale");
    catalog
        .begin_scan_with_publication_namespace(
            &request("busy", &fixture.original),
            &fixture.root.root_id,
            &fixture.root.root_path,
            fixture.root.publication_root_identity.as_ref().unwrap(),
        )
        .expect("active scan");
    let error = catalog
        .begin_relocated_scan(
            &request("conflict", &fixture.replacement),
            &fixture.root,
            &path,
            &identity,
        )
        .expect_err("active scan");
    assert_eq!(error.code, "catalog_root_scan_in_progress");
    assert_eq!(
        catalog
            .load_incremental_catalog_root(&fixture.root.root_id)
            .unwrap()
            .unwrap()
            .root_generation,
        fixture.root.root_generation
    );
    assert_eq!(catalog.load_registered_root_id(&path).unwrap(), None);
}
