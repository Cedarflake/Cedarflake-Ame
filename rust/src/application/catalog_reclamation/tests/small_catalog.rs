use super::*;
use crate::application::scan_library::run_scan_with_storage;
use crate::application::storage::StoragePaths;
use crate::domain::{GalleryQuery, ScanRequest};
use crate::ports::CatalogRepository;
use crate::ports::organization::OrganizationRepository;

fn add_free_pages(path: &Path) {
    Connection::open(path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE reclaimable_fixture(payload BLOB NOT NULL);
             INSERT INTO reclaimable_fixture VALUES (zeroblob(25165824));
             DROP TABLE reclaimable_fixture;",
        )
        .unwrap();
}

fn space(path: &Path) -> CatalogSpaceUsage {
    match SqliteCatalogSpaceMaintenance::new(path.to_path_buf())
        .inspect_catalog_space()
        .unwrap()
    {
        CatalogMaintenanceAttempt::Completed(usage) => usage,
        _ => panic!("isolated catalog must be idle"),
    }
}

#[test]
fn smaller_incremental_catalog_reclaims_space_and_preserves_search_and_decisions() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("source");
    fs::create_dir(&source).unwrap();
    let image_path = source.join("retained.png");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([40, 80, 120, 255]))
        .save(&image_path)
        .unwrap();
    let original = fs::read(&image_path).unwrap();
    let path = directory.path().join("catalog.sqlite3");
    run_scan_with_storage(
        ScanRequest {
            scan_id: "small-catalog".into(),
            root_path: source.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        StoragePaths {
            catalog_path: path.clone(),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings.sqlite3"),
        },
    )
    .unwrap();
    let query = GalleryQuery {
        search_text: "retained".into(),
        ..GalleryQuery::default()
    };
    let mut catalog = SqliteCatalog::open(path.clone()).unwrap();
    let revision = catalog
        .load_organization_albums(None)
        .unwrap()
        .organization_revision;
    catalog
        .create_organization_album("album", revision, "Retained")
        .unwrap();
    let history = catalog.load_organization_history().unwrap();
    let before_assets = catalog
        .load_snapshot(10, &query, "search", None, None, None)
        .unwrap()
        .assets;
    assert_eq!(before_assets.len(), 1);
    drop(catalog);
    add_free_pages(&path);
    let before = space(&path);
    assert!(before.catalog_file_bytes() < 64 * 1024 * 1024);
    assert!(before.reclaimable_bytes() > 16 * 1024 * 1024);
    let operation = CatalogReclamationOperation::new("small-catalog-reclamation".into());
    operation
        .finish(run_reclamation_inner(&path, &operation))
        .unwrap();
    assert_eq!(
        operation.snapshot().unwrap().phase,
        CatalogReclamationPhase::Completed
    );
    let after = space(&path);
    assert!(after.catalog_file_bytes() < before.catalog_file_bytes() - 16 * 1024 * 1024);
    assert!(after.reclaimable_bytes() <= 8 * 1024 * 1024);
    let mut reopened = SqliteCatalog::open(path).unwrap();
    assert_eq!(
        reopened
            .load_snapshot(10, &query, "search", None, None, None)
            .unwrap()
            .assets,
        before_assets
    );
    assert_eq!(reopened.load_organization_history().unwrap(), history);
    assert_eq!(
        reopened.load_organization_albums(None).unwrap().albums[0].name,
        "Retained"
    );
    assert_eq!(fs::read(image_path).unwrap(), original);
}

#[test]
fn smaller_legacy_catalog_does_not_trigger_a_full_rewrite() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(path.clone()).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = NONE; VACUUM;")
        .unwrap();
    add_free_pages(&path);
    let before = space(&path);
    let operation = CatalogReclamationOperation::new("small-legacy-catalog".into());
    operation
        .finish(run_reclamation_inner(&path, &operation))
        .unwrap();
    assert_eq!(space(&path), before);
    assert_eq!(
        operation.snapshot().unwrap().phase,
        CatalogReclamationPhase::Completed
    );
}
