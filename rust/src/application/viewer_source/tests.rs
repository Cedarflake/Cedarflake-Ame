use super::*;

thread_local! {
    static AFTER_SOURCE_GUARD: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}

pub(super) fn after_source_guard() {
    let hook = AFTER_SOURCE_GUARD.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[test]
fn source_lease_admission_is_bounded_and_released_on_drop() {
    static ACTIVE: AtomicUsize = AtomicUsize::new(0);
    let first = SourceLeaseAdmission::acquire(&ACTIVE).expect("first source read");
    let second = SourceLeaseAdmission::acquire(&ACTIVE).expect("second source read");
    assert_eq!(
        SourceLeaseAdmission::acquire(&ACTIVE)
            .err()
            .expect("bounded admission")
            .code,
        "viewer_source_busy"
    );
    drop(first);
    let replacement = SourceLeaseAdmission::acquire(&ACTIVE).expect("released slot");
    drop((second, replacement));
    assert_eq!(ACTIVE.load(Ordering::Acquire), 0);
}

#[test]
fn source_request_rejects_unbounded_or_unproven_identifiers() {
    let mut request = ViewerSourceRequest {
        location_id: "location".to_owned(),
        expected_root_id: "root".to_owned(),
        expected_scan_id: "scan".to_owned(),
        expected_source_revision: None,
        expected_source_generation: 1,
    };
    validate_request(&request).expect("bounded request");
    request.expected_source_generation = 0;
    assert!(validate_request(&request).is_err());
    request.expected_source_generation = 1;
    request.location_id = "x".repeat(513);
    assert!(validate_request(&request).is_err());
    request.location_id = "location".to_owned();
    request.expected_source_revision = Some(SourceRevisionEvidence {
        scheme: "forged".to_owned(),
        value: "x".repeat(1024),
    });
    assert!(validate_request(&request).is_err());
}

#[cfg(windows)]
mod windows {
    use std::fs::{self, File, FileTimes, OpenOptions};
    use std::path::PathBuf;
    use std::time::UNIX_EPOCH;

    use image::{Rgb, RgbImage};
    use tempfile::{TempDir, tempdir};

    use crate::adapters::{
        FileDiscovery, SqliteCatalog, canonical_source_root_path, file_identity_evidence,
        open_preview_source,
    };
    use crate::domain::{ExpectedFileState, PreviewStatus, ScanRequest};
    use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

    use super::*;

    #[test]
    fn viewer_read_pins_source_and_every_nested_parent_until_release() {
        static ACTIVE: AtomicUsize = AtomicUsize::new(0);
        let fixture = Fixture::new();
        let lease = acquire_with_catalog(
            &fixture.request,
            &fixture.catalog_path,
            SourceLeaseAdmission::acquire(&ACTIVE).expect("admit"),
        )
        .expect("guarded viewer source");
        assert_eq!(
            fs::read(lease.path()).expect("normal Flutter-equivalent source reopen"),
            fs::read(&fixture.path).expect("source bytes")
        );
        assert!(OpenOptions::new().write(true).open(&fixture.path).is_err());
        assert!(fs::remove_file(&fixture.path).is_err());
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_WRITE_ATTRIBUTES, FILE_WRITE_DATA,
        };
        for parent in [
            fixture.path.parent().expect("parent"),
            fixture.root.as_path(),
        ] {
            assert!(
                fs::rename(parent, parent.with_extension("moved")).is_err(),
                "every mutable parent must remain pinned"
            );
        }
        assert!(
            OpenOptions::new()
                .access_mode(FILE_WRITE_DATA)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
                .open(fixture.path.parent().expect("nested parent"))
                .is_err(),
            "the guarded descendant must reject data-write access"
        );
        drop(
            OpenOptions::new()
                .access_mode(FILE_WRITE_ATTRIBUTES)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
                .open(fixture.path.parent().expect("nested parent"))
                .expect("Windows attribute-only access is outside share-mode exclusion"),
        );
        drop(lease);
        assert_eq!(ACTIVE.load(Ordering::Acquire), 0);
        let nested = fixture.path.parent().expect("parent");
        fs::rename(nested, nested.with_extension("moved")).expect("parent rename after release");
    }

    #[test]
    fn viewer_read_rejects_same_size_same_timestamp_source_replacement() {
        static ACTIVE: AtomicUsize = AtomicUsize::new(0);
        let fixture = Fixture::new();
        let metadata = fixture.path.metadata().expect("source state");
        let mut bytes = fs::read(&fixture.path).expect("source");
        bytes[20] ^= 1;
        fs::write(&fixture.path, bytes).expect("in-place fixture replacement");
        File::options()
            .write(true)
            .open(&fixture.path)
            .expect("timestamp handle")
            .set_times(FileTimes::new().set_modified(metadata.modified().expect("modified")))
            .expect("restore timestamp");
        assert_eq!(
            fixture.path.metadata().expect("new state").len(),
            metadata.len()
        );
        assert_eq!(
            acquire_with_catalog(
                &fixture.request,
                &fixture.catalog_path,
                SourceLeaseAdmission::acquire(&ACTIVE).expect("admit")
            )
            .err()
            .expect("changed source rejected")
            .code,
            "viewer_source_unavailable"
        );
        assert_eq!(ACTIVE.load(Ordering::Acquire), 0);
    }

    #[test]
    fn viewer_read_rejects_removed_root_and_stale_generation_without_a_lease() {
        static ACTIVE: AtomicUsize = AtomicUsize::new(0);
        let fixture = Fixture::new();
        let mut stale = fixture.request.clone();
        stale.expected_source_generation += 1;
        assert_eq!(
            acquire_with_catalog(
                &stale,
                &fixture.catalog_path,
                SourceLeaseAdmission::acquire(&ACTIVE).expect("admit")
            )
            .err()
            .expect("stale generation")
            .code,
            "viewer_source_superseded"
        );
        let mut catalog = SqliteCatalog::open(fixture.catalog_path.clone()).expect("catalog");
        catalog
            .unregister_root(&fixture.request.expected_root_id)
            .expect("remove fixture root");
        drop(catalog);
        assert_eq!(
            acquire_with_catalog(
                &fixture.request,
                &fixture.catalog_path,
                SourceLeaseAdmission::acquire(&ACTIVE).expect("admit")
            )
            .err()
            .expect("removed root")
            .code,
            "viewer_source_superseded"
        );
        assert_eq!(ACTIVE.load(Ordering::Acquire), 0);
    }

    #[test]
    fn viewer_read_requires_durable_root_identity_and_enforces_encoded_bounds() {
        let fixture = Fixture::new();
        let catalog = SqliteCatalog::open(fixture.catalog_path.clone()).expect("catalog");
        let mut location = catalog
            .load_active_location(&fixture.request.location_id)
            .expect("location query")
            .expect("location");
        let mut root = catalog
            .load_incremental_catalog_root(&fixture.request.expected_root_id)
            .expect("root query")
            .expect("root");
        root.publication_root_identity = None;
        assert_eq!(
            acquire_guard(&location, &root)
                .err()
                .expect("missing root identity")
                .code,
            "viewer_source_identity_unproven"
        );
        location.file_size = MAX_ENCODED_SOURCE_BYTES;
        validate_context(&fixture.request, Some(location.clone()), Some(root.clone()))
            .expect("exact encoded limit");
        location.file_size += 1;
        assert_eq!(
            validate_context(&fixture.request, Some(location.clone()), Some(root.clone()))
                .expect_err("encoded limit")
                .code,
            "viewer_source_size_exceeded"
        );
        location.file_size = 1;
        location.absolute_path = format!(
            "C:\\{}\\source.png",
            "nested\\".repeat(MAX_SOURCE_PATH_COMPONENTS)
        );
        assert_eq!(
            validate_context(&fixture.request, Some(location), Some(root))
                .expect_err("namespace bound")
                .code,
            "viewer_source_path_exceeded"
        );
    }

    #[test]
    fn viewer_read_rejects_root_unregistered_after_guard_before_catalog_recheck() {
        static ACTIVE: AtomicUsize = AtomicUsize::new(0);
        assert_guarded_catalog_change(CatalogChange::UnregisterRoot, &ACTIVE);
    }

    #[test]
    fn viewer_read_rejects_generation_changed_after_guard_before_catalog_recheck() {
        static ACTIVE: AtomicUsize = AtomicUsize::new(0);
        assert_guarded_catalog_change(CatalogChange::SourceGeneration, &ACTIVE);
    }

    enum CatalogChange {
        UnregisterRoot,
        SourceGeneration,
    }

    fn assert_guarded_catalog_change(change: CatalogChange, active: &'static AtomicUsize) {
        let fixture = Fixture::new();
        let source_bytes = fs::read(&fixture.path).expect("original fixture bytes");
        let catalog_path = fixture.catalog_path.clone();
        let root_id = fixture.request.expected_root_id.clone();
        let location_id = fixture.request.location_id.clone();
        let guarded_path = fixture.path.clone();
        let guarded_root = fixture.root.clone();
        let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reached = ran.clone();
        AFTER_SOURCE_GUARD.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(Box::new(move || {
                reached.store(true, Ordering::Release);
                assert!(OpenOptions::new().write(true).open(&guarded_path).is_err());
                for parent in [guarded_path.parent().expect("nested parent"), guarded_root.as_path()] {
                    assert!(fs::rename(parent, parent.with_extension("moved")).is_err());
                }
                match change {
                    CatalogChange::UnregisterRoot => {
                        SqliteCatalog::open(catalog_path).expect("concurrent catalog")
                            .unregister_root(&root_id).expect("unregister while source guard is held");
                    }
                    CatalogChange::SourceGeneration => {
                        let mut connection = rusqlite::Connection::open(catalog_path).expect("concurrent catalog");
                        let transaction = connection.transaction().expect("source-generation transaction");
                        assert_eq!(transaction.execute(
                            "UPDATE asset_locations SET source_generation = source_generation + 1 WHERE location_id = ?1",
                            [&location_id],
                        ).expect("advance fixture source generation"), 1);
                        transaction.execute("UPDATE catalog_state SET next_source_generation = next_source_generation + 1", [])
                            .expect("advance generation allocation boundary");
                        transaction.commit().expect("commit new generation");
                    }
                }
            }));
        });
        struct HookReset;
        impl Drop for HookReset {
            fn drop(&mut self) {
                AFTER_SOURCE_GUARD.with(|slot| {
                    slot.borrow_mut().take();
                });
            }
        }
        let _hook_reset = HookReset;
        let error = acquire_with_catalog(
            &fixture.request,
            &fixture.catalog_path,
            SourceLeaseAdmission::acquire(active).expect("admit guarded read"),
        )
        .err()
        .expect("catalog recheck must reject stale guarded source");
        assert!(
            ran.load(Ordering::Acquire),
            "the mutation must happen after native protection"
        );
        assert_eq!(error.code, "viewer_source_superseded");
        assert_eq!(
            active.load(Ordering::Acquire),
            0,
            "rejected admission must return its capacity"
        );
        drop(
            OpenOptions::new()
                .write(true)
                .open(&fixture.path)
                .expect("source handle released"),
        );
        let parent = fixture.path.parent().expect("nested parent");
        let moved = parent.with_extension("moved");
        fs::rename(parent, &moved).expect("nested guard released");
        fs::rename(&moved, parent).expect("restore generated fixture parent");
        let moved_root = fixture.root.with_extension("moved");
        fs::rename(&fixture.root, &moved_root).expect("configured-root guard released");
        fs::rename(&moved_root, &fixture.root).expect("restore generated root");
        assert_eq!(
            fs::read(&fixture.path).expect("source after rejection"),
            source_bytes
        );
    }

    struct Fixture {
        _directory: TempDir,
        root: PathBuf,
        path: PathBuf,
        catalog_path: PathBuf,
        request: ViewerSourceRequest,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempdir().expect("fixture storage");
            let root = directory.path().join("source");
            fs::create_dir_all(root.join("nested")).expect("nested source root");
            let root = canonical_source_root_path(&root).expect("canonical source root");
            let path = root.join("nested").join("图片.png");
            RgbImage::from_pixel(16, 12, Rgb([12, 64, 128]))
                .save(&path)
                .expect("source PNG");
            let metadata = path.metadata().expect("source metadata");
            let modified_unix_ms = i64::try_from(
                metadata
                    .modified()
                    .expect("modified")
                    .duration_since(UNIX_EPOCH)
                    .expect("epoch")
                    .as_millis(),
            )
            .expect("timestamp");
            let identity = FileDiscovery::new(root.to_str().expect("UTF8 root"))
                .expect("discovery")
                .metadata_inventory_root_identity()
                .expect("root identity query")
                .expect("root identity");
            let expected = ExpectedFileState {
                absolute_path: path.to_string_lossy().into_owned(),
                file_size: metadata.len(),
                modified_unix_ms,
                file_identity: file_identity_evidence(&path).expect("file identity"),
                source_revision: None,
            };
            let source_revision = open_preview_source(&expected, &root, Some(&identity))
                .expect("source evidence")
                .source_revision;
            let suffix = directory
                .path()
                .file_name()
                .expect("unique fixture")
                .to_string_lossy();
            let root_id = format!("viewer-root-{suffix}");
            let scan_id = format!("viewer-scan-{suffix}");
            let location_id = format!("viewer-location-{suffix}");
            let catalog_path = directory.path().join("catalog.sqlite3");
            let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
            let scan = ScanRequest {
                scan_id: scan_id.clone(),
                root_path: root.to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 256,
            };
            catalog
                .begin_scan_with_publication_namespace(&scan, &root_id, &scan.root_path, &identity)
                .expect("begin source fixture");
            catalog
                .prove_live_only_first_import_handoff_for_test(&scan_id)
                .expect("first-import handoff");
            catalog
                .stage_location(
                    &scan_id,
                    &root_id,
                    &AssetLocationView {
                        asset_id: format!("viewer-asset-{suffix}"),
                        location_id: location_id.clone(),
                        root_id: root_id.clone(),
                        scan_id: scan_id.clone(),
                        absolute_path: expected.absolute_path,
                        display_path: "nested/图片.png".to_owned(),
                        relative_path: "nested/图片.png".to_owned(),
                        preview_path: String::new(),
                        file_size: expected.file_size,
                        created_unix_ms: None,
                        modified_unix_ms,
                        file_identity: expected.file_identity,
                        source_revision: source_revision.clone(),
                        source_generation: 0,
                        width: 16,
                        height: 12,
                        preview_status: PreviewStatus::Pending,
                        preview_issue_code: None,
                        preview_issue_message: None,
                        metadata_engine_id: "fixture".to_owned(),
                        metadata_engine_version: "1".to_owned(),
                        capture_time: None,
                    },
                )
                .expect("stage source fixture");
            catalog
                .publish_scan(&scan_id, &root_id, 1, 0)
                .expect("publish source fixture");
            let source_generation = catalog
                .load_active_location(&location_id)
                .expect("location query")
                .expect("published location")
                .source_generation;
            drop(catalog);
            Self {
                _directory: directory,
                root,
                path,
                catalog_path,
                request: ViewerSourceRequest {
                    location_id,
                    expected_root_id: root_id,
                    expected_scan_id: scan_id,
                    expected_source_revision: source_revision,
                    expected_source_generation: source_generation,
                },
            }
        }
    }
}
