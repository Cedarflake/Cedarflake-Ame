use super::*;
use crate::domain::file_operation_exports::{FileExportReport, FileOrganizationRule};
use crate::domain::file_operation_selection::{FileOperationCommand, FileOperationSelectionScope};
use crate::domain::file_operation_task::{
    FileOperationTaskKind, FileOperationTaskPhase, FileOperationTaskProgress,
    FileOperationTaskRequest, FileOperationTaskTarget,
};
use crate::domain::organization::OrganizationAction;
use crate::ports::file_operation_exports::FileExportRepository;

#[test]
#[ignore = "requires guest-local Windows Sandbox fixtures"]
fn file_operation_sandbox_organizes_files_and_retries_only_the_result_report() {
    assert!(
        std::env::var("USERNAME").is_ok_and(|name| name.eq_ignore_ascii_case("WDAGUtilityAccount"))
    );
    let base = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("AmeR8");
    fs::create_dir_all(&base).unwrap();
    let fixture = tempfile::Builder::new()
        .prefix("export-")
        .tempdir_in(base)
        .unwrap();
    let source = fixture.path().join("source");
    let exports = fixture.path().join("exports");
    fs::create_dir(&source).unwrap();
    fs::create_dir_all(exports.join("旅行")).unwrap();
    for (name, color) in [("图片.png", [23, 86, 137]), ("无相册.png", [128, 76, 35])] {
        RgbImage::from_pixel(16, 12, Rgb(color))
            .save(source.join(name))
            .unwrap();
    }
    let original = fs::read(source.join("图片.png")).unwrap();
    let unassigned = fs::read(source.join("无相册.png")).unwrap();
    fs::write(exports.join("旅行/图片.png"), b"existing destination").unwrap();
    let storage = StoragePaths {
        catalog_path: fixture.path().join("catalog.sqlite3"),
        preview_root: fixture.path().join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: fixture.path().join("settings.sqlite3"),
    };
    scan_library::run_scan_with_storage(
        ScanRequest {
            scan_id: "export-fixture".into(),
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
    let assigned = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "图片.png")
        .unwrap()
        .unwrap();
    let other = catalog
        .load_incremental_location_by_relative_path(&root.root_id, "无相册.png")
        .unwrap()
        .unwrap();
    let album = catalog
        .create_organization_album("create-album", 0, "旅行", Default::default())
        .unwrap();
    let prepared = catalog
        .prepare_organization_selection(&[selection(&assigned)])
        .unwrap();
    catalog
        .change_organization(
            "assign",
            &prepared,
            &OrganizationAction::AssignAlbum {
                album_id: album.album_id,
            },
        )
        .unwrap();
    let locations = [selection(&assigned), selection(&other)];
    let result = transfer(
        &storage,
        &locations,
        &exports,
        "album-copy",
        FileOrganizationRule::Album,
        FileAction::Copy,
    );
    assert_eq!((result.published_items, result.skipped_items), (1, 1));
    assert_eq!(
        result.phase,
        FileOperationTaskPhase::Finished,
        "{:?}",
        result.failure
    );
    let report = report_path(result);
    let text = fs::read_to_string(&report).unwrap();
    assert!(text.contains("图片(1).png"));
    assert!(text.contains("无相册.png"));
    assert!(text.contains("已跳过"));
    assert_eq!(
        fs::read(exports.join("旅行/图片(1).png")).unwrap(),
        original
    );
    assert_eq!(
        fs::read(exports.join("旅行/图片.png")).unwrap(),
        b"existing destination"
    );

    let unavailable = fixture.path().join("temporarily-unavailable");
    fs::rename(&exports, &unavailable).unwrap();
    let failure = write_report(&storage, "album-copy");
    assert_eq!(failure.phase, FileOperationTaskPhase::NeedsAttention);
    fs::rename(&unavailable, &exports).unwrap();
    let retry = write_report(&storage, "album-copy");
    assert_eq!(
        retry.phase,
        FileOperationTaskPhase::Finished,
        "{:?}",
        retry.failure
    );
    let second_report = report_path(retry);
    assert_ne!(report, second_report);
    assert_eq!(fs::read_to_string(&report).unwrap(), text);
    assert_eq!(fs::read_dir(exports.join("旅行")).unwrap().count(), 2);
    let journal = SqliteFileOperationJournal::open(storage.catalog_path.clone()).unwrap();
    assert_eq!(journal.batch_count("album-copy").unwrap(), 1);
    assert_eq!(
        journal
            .load_export("album-copy")
            .unwrap()
            .unwrap()
            .skipped_files,
        1
    );

    let dated = fixture.path().join("dated");
    fs::create_dir(&dated).unwrap();
    let dates = Connection::open(&storage.catalog_path).unwrap();
    dates
        .execute(
            "UPDATE asset_locations SET capture_local_time = '2024-06-02T12:00:00',
             capture_time_source = 'exif_original', capture_raw_value = '2024:06:02 12:00:00'",
            [],
        )
        .unwrap();
    let clean = transfer(
        &storage,
        &locations,
        &dated,
        "date-copy",
        FileOrganizationRule::Date,
        FileAction::Copy,
    );
    assert_eq!((clean.published_items, clean.skipped_items), (2, 0));
    assert!(matches!(
        clean.export.unwrap().report,
        FileExportReport::NotNeeded
    ));
    assert_eq!(fs::read_dir(&dated).unwrap().count(), 1);
    assert_eq!(fs::read(dated.join("2024-06/图片.png")).unwrap(), original);
    assert_eq!(
        fs::read(dated.join("2024-06/无相册.png")).unwrap(),
        unassigned
    );
    assert_eq!(fs::read(source.join("图片.png")).unwrap(), original);
    assert_eq!(fs::read(source.join("无相册.png")).unwrap(), unassigned);
    assert_eq!(journal.publication_roots().unwrap().len(), 1);
    assert!(assigned.absolute_path.starts_with(r"\\?\"));
    assert!(!source.to_str().unwrap().starts_with(r"\\?\"));
    let same_folder = transfer(
        &storage,
        &[selection(&assigned)],
        &source,
        "same-folder-move",
        FileOrganizationRule::Direct,
        FileAction::Move,
    );
    assert_eq!(same_folder.phase, FileOperationTaskPhase::Finished);
    assert_eq!(
        (same_folder.published_items, same_folder.skipped_items),
        (0, 1)
    );
    assert_eq!(journal.batch_count("same-folder-move").unwrap(), 0);
    assert!(
        fs::read_to_string(report_path(same_folder))
            .unwrap()
            .contains("文件已在目标位置")
    );
    assert_eq!(fs::read(source.join("图片.png")).unwrap(), original);
    assert!(!source.join("图片(1).png").exists());

    let same_folder_copy = transfer(
        &storage,
        &[selection(&assigned)],
        &source,
        "same-folder-copy",
        FileOrganizationRule::Direct,
        FileAction::Copy,
    );
    assert_eq!(
        (
            same_folder_copy.published_items,
            same_folder_copy.skipped_items
        ),
        (1, 0)
    );
    assert_eq!(fs::read(source.join("图片.png")).unwrap(), original);
    assert_eq!(fs::read(source.join("图片(1).png")).unwrap(), original);

    let selected = FileOperationSelectionSession::for_catalog(
        storage.catalog_path.clone(),
        FileOperationSelectionScope::Locations,
    )
    .unwrap();
    selected
        .append_locations(vec![selection(&assigned)])
        .unwrap();
    let renamed = finish(
        selected
            .execute(
                "case-only-rename".into(),
                FileOperationCommand {
                    action: FileAction::Rename,
                    destination_directory: None,
                    new_name: Some("图片.PNG".into()),
                    organization_rule: FileOrganizationRule::Direct,
                },
                None,
            )
            .unwrap(),
    );
    selected.close().unwrap();
    assert_eq!((renamed.published_items, renamed.skipped_items), (1, 0));
    assert!(
        fs::read_dir(&source)
            .unwrap()
            .any(|entry| entry.unwrap().file_name() == "图片.PNG")
    );
    assert_eq!(fs::read(source.join("图片.PNG")).unwrap(), original);
    println!(
        "album subfolder, collision naming, skipped membership, UTF-8 report, report-only retry, date folder, clean-run omission, same-folder move skip, same-folder copy and case-only rename: PASS"
    );
}

pub(super) fn transfer(
    storage: &StoragePaths,
    locations: &[OrganizationSelection],
    root: &std::path::Path,
    batch: &str,
    rule: FileOrganizationRule,
    action: FileAction,
) -> FileOperationTaskProgress {
    let selected = FileOperationSelectionSession::for_catalog(
        storage.catalog_path.clone(),
        FileOperationSelectionScope::Locations,
    )
    .unwrap();
    selected.append_locations(locations.to_vec()).unwrap();
    let task = selected
        .execute(
            batch.into(),
            FileOperationCommand {
                action,
                destination_directory: Some(root.to_string_lossy().into_owned()),
                new_name: None,
                organization_rule: rule,
            },
            None,
        )
        .unwrap();
    let result = finish(task);
    selected.close().unwrap();
    result
}

fn write_report(storage: &StoragePaths, batch: &str) -> FileOperationTaskProgress {
    finish(
        start_file_operation_task(
            storage.catalog_path.clone(),
            FileOperationTaskRequest {
                kind: FileOperationTaskKind::WriteReport,
                target: FileOperationTaskTarget::Export(batch.into()),
            },
            None,
        )
        .unwrap(),
    )
}

pub(super) fn report_path(progress: FileOperationTaskProgress) -> String {
    match progress.export.unwrap().report {
        FileExportReport::Written(path) => path,
        other => panic!("Expected report, got {other:?}"),
    }
}

pub(super) fn finish(task: FileOperationTask) -> FileOperationTaskProgress {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let progress = task.snapshot();
        if matches!(
            progress.phase,
            FileOperationTaskPhase::Finished
                | FileOperationTaskPhase::Cancelled
                | FileOperationTaskPhase::NeedsAttention
        ) {
            task.close().unwrap();
            return progress;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "export task did not retire"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
