use super::*;
use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};
use crate::domain::file_operation_exports::FileOrganizationRule;
use crate::domain::file_operation_selection::{
    DuplicateCleanupGroup, DuplicateCleanupKind, FileOperationCommand, FileOperationSelectionScope,
};
use crate::domain::file_operation_task::FileOperationTaskPhase;
use crate::ports::exact_duplicate_review::ExactDuplicateReviewRepository;

#[test]
#[ignore = "requires guest-local Windows Sandbox fixtures"]
fn file_operation_sandbox_cleanup_preserves_keeps_and_revalidates_before_deletion() {
    assert!(
        std::env::var("USERNAME").is_ok_and(|name| name.eq_ignore_ascii_case("WDAGUtilityAccount"))
    );
    let base = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("AmeR8");
    fs::create_dir_all(&base).unwrap();
    let fixture = tempfile::Builder::new()
        .prefix("cleanup-")
        .tempdir_in(base)
        .unwrap();
    let source = fixture.path().join("source");
    fs::create_dir(&source).unwrap();
    for (suffix, color) in [
        ("a", [23, 86, 137]),
        ("b", [128, 76, 35]),
        ("c", [75, 32, 196]),
        ("d", [162, 127, 52]),
    ] {
        for role in ["keep", "drop"] {
            RgbImage::from_pixel(16, 12, Rgb(color))
                .save(source.join(format!("{role}-{suffix}.png")))
                .unwrap();
        }
    }
    let originals: Vec<_> = ["a", "b", "c", "d"]
        .into_iter()
        .map(|suffix| fs::read(source.join(format!("keep-{suffix}.png"))).unwrap())
        .collect();
    let storage = StoragePaths {
        catalog_path: fixture.path().join("catalog.sqlite3"),
        preview_root: fixture.path().join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: fixture.path().join("settings.sqlite3"),
    };
    scan_library::run_scan_with_storage(
        ScanRequest {
            scan_id: "cleanup-fixture".into(),
            root_path: source.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |_| true,
        storage.clone(),
    )
    .unwrap();
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    let root = SqliteFileOperationJournal::open(storage.catalog_path.clone())
        .unwrap()
        .publication_roots()
        .unwrap()
        .remove(0);
    let mut keys = Vec::new();
    for (suffix, bytes) in ["a", "b", "c", "d"].into_iter().zip(&originals) {
        let key = ExactDuplicateReviewKey {
            digest: blake3::hash(bytes).to_hex().to_string(),
            byte_length: bytes.len() as u64,
        };
        for role in ["keep", "drop"] {
            let location = catalog
                .load_incremental_location_by_relative_path(
                    &root.root_id,
                    &format!("{role}-{suffix}.png"),
                )
                .unwrap()
                .unwrap();
            super::recovery::publish_fingerprint(&mut catalog, &root, &location, bytes);
            if role == "keep" && ["a", "b"].contains(&suffix) {
                catalog
                    .set_duplicate_preference(
                        &key,
                        Some(&ExactDuplicatePreference {
                            asset_id: location.asset_id,
                            location_id: location.location_id,
                            source_generation: location.source_generation,
                        }),
                    )
                    .unwrap();
            }
        }
        keys.push(key);
    }
    catalog.set_duplicate_ignored(&keys[2], true).unwrap();
    let automatic = catalog
        .load_review_groups(None, false)
        .unwrap()
        .groups
        .into_iter()
        .find(|group| group.group.digest == keys[3].digest)
        .unwrap()
        .representative;
    let automatic_keep = source.join(
        std::path::Path::new(&automatic.absolute_path)
            .file_name()
            .unwrap(),
    );
    let automatic_drop = [source.join("keep-d.png"), source.join("drop-d.png")]
        .into_iter()
        .find(|path| path != &automatic_keep)
        .unwrap();
    let selected = FileOperationSelectionSession::for_catalog(
        storage.catalog_path.clone(),
        FileOperationSelectionScope::DuplicateCleanup {
            kind: DuplicateCleanupKind::Exact,
            all_groups: false,
        },
    )
    .unwrap();
    selected
        .append_groups(vec![DuplicateCleanupGroup::Exact {
            key: keys.remove(0),
        }])
        .unwrap();
    assert_eq!(selected.prepare().unwrap().file_count, 1);
    let command = |action| FileOperationCommand {
        action,
        destination_directory: None,
        new_name: None,
        organization_rule: FileOrganizationRule::Direct,
    };
    let progress = super::exports::finish(
        selected
            .execute(
                "selected-cleanup".into(),
                command(FileAction::DeletePermanently),
                None,
            )
            .unwrap(),
    );
    selected.close().unwrap();
    assert_eq!(progress.phase, FileOperationTaskPhase::Finished);
    assert_eq!(progress.published_items, 1);
    assert_eq!(progress.failed_items, 0);
    assert!(!source.join("drop-a.png").exists());
    assert!(source.join("drop-b.png").exists());
    let all = FileOperationSelectionSession::for_catalog(
        storage.catalog_path.clone(),
        FileOperationSelectionScope::DuplicateCleanup {
            kind: DuplicateCleanupKind::Exact,
            all_groups: true,
        },
    )
    .unwrap();
    let summary = all.prepare().unwrap();
    assert_eq!(summary.file_count, 2);
    assert_eq!(summary.cleanup.unwrap().skipped_groups, 0);
    let progress = super::exports::finish(
        all.execute("all-cleanup".into(), command(FileAction::Recycle), None)
            .unwrap(),
    );
    all.close().unwrap();
    assert_eq!(progress.phase, FileOperationTaskPhase::Finished);
    assert_eq!(progress.published_items, 2);
    assert_eq!(progress.failed_items, 0);
    assert!(!source.join("drop-b.png").exists());
    assert!(!automatic_drop.exists());
    assert_eq!(fs::read(&automatic_keep).unwrap(), originals[3]);
    assert!(source.join("drop-c.png").exists());
    catalog.set_duplicate_ignored(&keys[1], false).unwrap();
    let kept = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "keep-c.png")
        .unwrap()
        .unwrap();
    catalog
        .set_duplicate_preference(
            &keys[1],
            Some(&ExactDuplicatePreference {
                asset_id: kept.asset_id,
                location_id: kept.location_id,
                source_generation: kept.source_generation,
            }),
        )
        .unwrap();
    let changed = FileOperationSelectionSession::for_catalog(
        storage.catalog_path.clone(),
        FileOperationSelectionScope::DuplicateCleanup {
            kind: DuplicateCleanupKind::Exact,
            all_groups: true,
        },
    )
    .unwrap();
    assert_eq!(changed.prepare().unwrap().file_count, 1);
    let keep = source.join("keep-c.png");
    let held = fixture.path().join("held.png");
    fs::rename(&keep, &held).unwrap();
    let error = match changed.execute(
        "missing-keep".into(),
        command(FileAction::DeletePermanently),
        None,
    ) {
        Err(error) => error,
        Ok(_) => panic!("Cleanup must not start when its retained file is missing"),
    };
    assert_eq!(error.code, "file_operation_keep_unavailable");
    assert_eq!(fs::read(source.join("drop-c.png")).unwrap(), originals[2]);
    fs::rename(&held, &keep).unwrap();
    changed.close().unwrap();
    assert_eq!(fs::read(&keep).unwrap(), originals[2]);
    for (suffix, bytes) in ["a", "b"].into_iter().zip(&originals) {
        assert_eq!(
            fs::read(source.join(format!("keep-{suffix}.png"))).unwrap(),
            *bytes
        );
        assert!(
            catalog
                .load_incremental_location_by_relative_path(
                    &root.root_id,
                    &format!("drop-{suffix}.png")
                )
                .unwrap()
                .is_none()
        );
    }
    println!(
        "selected/all cleanup, automatic representative retained, missing keep blocks mutation, permanent/system recycle, retained bytes and catalog publication: PASS"
    );
}
