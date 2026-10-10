use std::os::windows::fs::{FileTimesExt, MetadataExt};
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use super::*;

fn fixture_root() -> PathBuf {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let root = PathBuf::from(std::env::var("AME_GENERATED_NATIVE_ROOT").unwrap());
    let build = repository.join("build");
    assert_eq!(root.parent().unwrap(), build);
    for parent in build.ancestors() {
        assert_eq!(
            fs::symlink_metadata(parent).unwrap().file_attributes() & 0x400,
            0
        );
    }
    let name = root.file_name().unwrap().to_str().unwrap();
    let nonce = name.strip_prefix("integration-storage-").unwrap();
    assert_eq!(nonce.len(), 32);
    assert!(nonce.bytes().all(|byte| byte.is_ascii_hexdigit()));
    root
}

#[test]
#[ignore = "prepares one explicitly selected generated fixture for native recovery verification"]
fn prepare_large_inventory_native_fixture() {
    let root = fixture_root();
    fs::create_dir(&root).unwrap();
    let original = root.join("sources/original");
    let replacement = root.join("sources/recovered");
    let derived = root.join("derived");
    fs::create_dir_all(&original).unwrap();
    let dimensions = [
        (1200, 800),
        (800, 1200),
        (4096, 2048),
        (1024, 4096),
        (32, 32),
        (6000, 4000),
    ];
    let mut source_bytes = std::collections::BTreeMap::new();
    for (index, (width, height)) in dimensions.into_iter().enumerate() {
        let file_name = format!("sample-{index}.png");
        let path = original.join(&file_name);
        let color = [
            40 + index as u8 * 30,
            170 - index as u8 * 20,
            50 + index as u8 * 25,
        ];
        RgbImage::from_pixel(width, height, Rgb(color))
            .save(&path)
            .unwrap();
        let modified = UNIX_EPOCH + Duration::from_secs(1_262_304_000 + index as u64 * 94_608_000);
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(
                fs::FileTimes::new()
                    .set_created(modified)
                    .set_modified(modified),
            )
            .unwrap();
        source_bytes.insert(file_name, fs::read(path).unwrap());
    }
    let storage = StoragePaths {
        catalog_path: derived.join("catalog/ame.sqlite3"),
        preview_root: derived
            .join("cache/previews")
            .join(crate::adapters::PREVIEW_CACHE_VERSION),
        preview_budget_bytes: 4 * 1024 * 1024 * 1024,
        settings_path: derived.join("settings/storage.sqlite3"),
    };
    run_scan_with_storage(
        ScanRequest {
            scan_id: "native-large-inventory-initial".into(),
            root_path: original.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        },
        |_| true,
        storage.clone(),
    )
    .unwrap();
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    let published_roots = catalog.load_incremental_catalog_roots().unwrap();
    assert_eq!(published_roots.len(), 1);
    assert_eq!(assets(&mut catalog).len(), 6);
    let published = &published_roots[0];
    seed_inventory(&mut catalog, published, 3_073);
    seed_raw_spool(&catalog, 3_073);
    seed_candidates(&mut catalog, published, 2_305);
    drop(catalog);
    let catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_entries"),
        3_073
    );
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_candidate_owners"),
        2_305
    );
    drop(catalog);
    fs::rename(&original, &replacement).unwrap();
    let catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    assert_eq!(
        catalog.load_incremental_catalog_roots().unwrap()[0].root_path,
        published.root_path
    );
    drop(catalog);
    for (file_name, bytes) in source_bytes {
        assert_eq!(fs::read(replacement.join(file_name)).unwrap(), bytes);
    }
    fs::write(
        root.join("fixture-ready.txt"),
        "AME_LARGE_INVENTORY_FIXTURE images=6 logical_entries=3073 raw_entries=3073 candidate_owners=2305 full_reopen=passed",
    )
    .unwrap();
    println!(
        "AME_LARGE_INVENTORY_FIXTURE images=6 logical_entries=3073 raw_entries=3073 candidate_owners=2305 full_reopen=passed"
    );
}

#[test]
#[ignore = "verifies the closed catalog after the explicitly selected generated native lifetime"]
fn verify_large_inventory_native_fixture() {
    let root = fixture_root();
    assert!(root.join("fixture-ready.txt").is_file());
    assert!(!root.join("sources/original").exists());
    let mut catalog = SqliteCatalog::open(root.join("derived/catalog/ame.sqlite3")).unwrap();
    let roots = catalog.load_incremental_catalog_roots().unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(
        Path::new(&roots[0].root_path).canonicalize().unwrap(),
        root.join("sources/recovered").canonicalize().unwrap()
    );
    assert_eq!(assets(&mut catalog).len(), 6);
    assert!(
        catalog
            .load_metadata_inventory_run("old-inventory")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_entries"),
        0
    );
    assert_eq!(
        payload_count(&catalog, "library_metadata_inventory_candidate_owners"),
        0
    );
    let live_authorities: i64 = catalog.connection.query_row(
        "SELECT COUNT(*) FROM library_recovery_authorities WHERE run_id='old-inventory' AND retired_unix_ms IS NULL",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(live_authorities, 0);
    println!(
        "AME_LARGE_INVENTORY_NATIVE full_reopen=passed old_logical_payload=retired old_authority=retired images=6"
    );
}
