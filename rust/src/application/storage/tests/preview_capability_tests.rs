use super::super::preview_activation::activate_configured_preview_root_with_preparation;
use super::*;
use crate::adapters::{
    PreviewRootPreparation, prepare_preview_root, prepare_preview_root_with_namespace_probe,
};

fn unavailable_namespace(path: &Path) -> Result<PreviewRootPreparation, ScanError> {
    prepare_preview_root_with_namespace_probe(path, |_| {
        Err(ScanError::new(
            "injected_namespace_unavailable",
            "unsupported storage capability",
        ))
    })
}

#[test]
fn preview_capability_refusal_preserves_configured_catalog_and_settings_loading() {
    let directory = tempdir().expect("owned storage");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let settings_path = directory.path().join("settings.sqlite3");
    let cache = directory.path().join("existing-cache");
    publish_storage_fixture(&catalog_path, &directory.path().join("source"), &cache);
    let configured = StorageConfiguration {
        catalog_path: catalog_path.to_string_lossy().into_owned(),
        preview_root: cache.to_string_lossy().into_owned(),
        preview_budget_bytes: MIN_PREVIEW_BUDGET_BYTES,
    };
    SqliteStorageSettings::open(settings_path.clone())
        .expect("settings")
        .load_or_initialize(&configured)
        .expect("saved configuration");
    let before = load_fixture_snapshot(&catalog_path);
    let active_root = activate_configured_preview_root_with_preparation(
        &settings_path,
        &catalog_path,
        &configured,
        |path, _| unavailable_namespace(path),
    )
    .expect("catalog startup remains available");
    assert_eq!(active_root, cache);
    assert_preserved_catalog(&before, &catalog_path, &cache);
    let active = StoragePaths {
        catalog_path,
        preview_root: active_root,
        preview_budget_bytes: configured.preview_budget_bytes,
        settings_path: settings_path.clone(),
    };
    let status = storage_status(&active, &configured).expect("settings remain accessible");
    assert!(!status.requires_restart);
    assert_eq!(status.configured_preview_root, configured.preview_root);
    let mut settings = SqliteStorageSettings::open(settings_path).expect("reopened settings");
    assert_eq!(
        settings
            .load_or_initialize(&configured)
            .expect("configuration")
            .preview_root,
        configured.preview_root
    );
    assert!(
        settings
            .load_pending_preview_roots()
            .expect("pending")
            .is_empty()
    );
    assert!(
        settings
            .load_retired_preview_roots()
            .expect("retired")
            .is_empty()
    );
}

#[test]
fn preview_capability_refusal_keeps_previous_root_without_completing_pending_migration() {
    for previous_writable in [false, true] {
        let directory = tempdir().expect("owned storage");
        let catalog_path = directory.path().join("catalog.sqlite3");
        let settings_path = directory.path().join("settings.sqlite3");
        let old = directory.path().join("old-cache");
        let target = directory.path().join("missing-target");
        publish_storage_fixture(&catalog_path, &directory.path().join("source"), &old);
        let configured = save_pending_preview_target(&settings_path, &catalog_path, &old, &target);
        let before = load_fixture_snapshot(&catalog_path);
        let active = activate_configured_preview_root_with_preparation(
            &settings_path,
            &catalog_path,
            &configured,
            |path, _| {
                if previous_writable && path == old {
                    prepare_preview_root(path)
                } else {
                    unavailable_namespace(path)
                }
            },
        )
        .expect("retain usable previous catalog view");
        assert_eq!(active, old);
        assert_preserved_catalog(&before, &catalog_path, &old);
        assert!(!target.exists(), "no unsafe namespace creation fallback");
        let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
        assert_eq!(
            settings.load_pending_preview_roots().expect("pending"),
            vec![old.to_string_lossy().into_owned()]
        );
        assert!(
            settings
                .load_retired_preview_roots()
                .expect("retired")
                .is_empty()
        );
        assert_eq!(
            settings
                .load_or_initialize(&configured)
                .expect("configuration")
                .preview_root,
            configured.preview_root
        );
    }
}

#[test]
fn preview_capability_read_only_fallback_never_initializes_a_source_overlap() {
    let directory = tempdir().expect("owned storage");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let settings_path = directory.path().join("settings.sqlite3");
    let source = directory.path().join("source");
    let old = source.join("former-cache");
    let target = directory.path().join("missing-target");
    let cache = directory.path().join("actual-cache");
    publish_storage_fixture(&catalog_path, &source, &cache);
    let configured = save_pending_preview_target(&settings_path, &catalog_path, &old, &target);
    let before = load_fixture_snapshot(&catalog_path);
    let mut probed = Vec::new();
    let active = activate_configured_preview_root_with_preparation(
        &settings_path,
        &catalog_path,
        &configured,
        |path, _| {
            probed.push(path.to_path_buf());
            unavailable_namespace(path)
        },
    )
    .expect("safe target remains a read-only configuration");
    assert_eq!(active, target);
    assert_eq!(probed, vec![target.clone()]);
    assert!(!old.exists());
    assert!(!target.exists());
    assert_preserved_catalog(&before, &catalog_path, &cache);
}

#[test]
fn preview_capability_preparation_does_not_hide_an_inventory_failure() {
    let directory = tempdir().expect("owned storage");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let settings_path = directory.path().join("settings.sqlite3");
    let configured = StorageConfiguration {
        catalog_path: catalog_path.to_string_lossy().into_owned(),
        preview_root: directory
            .path()
            .join("cache")
            .to_string_lossy()
            .into_owned(),
        preview_budget_bytes: MIN_PREVIEW_BUDGET_BYTES,
    };
    SqliteStorageSettings::open(settings_path.clone())
        .expect("settings")
        .load_or_initialize(&configured)
        .expect("configuration");
    let error = activate_configured_preview_root_with_preparation(
        &settings_path,
        &catalog_path,
        &configured,
        |_, _| {
            Err(ScanError::new(
                "preview_cache_usage_unavailable",
                "injected inventory failure",
            ))
        },
    )
    .expect_err("only capability refusal has the read-only result");
    assert_eq!(error.code, "preview_cache_usage_unavailable");
}

fn load_fixture_snapshot(catalog_path: &Path) -> crate::domain::CatalogSnapshot {
    SqliteCatalog::open(catalog_path.to_path_buf())
        .expect("readable catalog")
        .load_snapshot(
            10,
            &GalleryQuery::default(),
            "preview-capability",
            None,
            None,
            None,
        )
        .expect("last trustworthy catalog")
}

fn assert_preserved_catalog(
    before: &crate::domain::CatalogSnapshot,
    catalog_path: &Path,
    cache: &Path,
) {
    let after = load_fixture_snapshot(catalog_path);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.assets.len(), 1);
    assert_eq!(
        format!("{:?}", after.assets),
        format!("{:?}", before.assets)
    );
    assert_eq!(format!("{:?}", after.roots), format!("{:?}", before.roots));
    assert_eq!(
        fs::read(cache.join("preview.jpg")).expect("untouched cached preview"),
        b"preview"
    );
    assert_eq!(fs::read_dir(cache).expect("cache entries").count(), 1);
}
