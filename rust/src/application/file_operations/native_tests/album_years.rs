use super::*;
use crate::domain::file_operation_exports::FileOrganizationRule;
use crate::domain::file_operation_task::FileOperationTaskPhase;
use crate::domain::organization::OrganizationAction;
use crate::ports::file_operation_exports::{FileExportRepository, NativeFileExports};
use std::os::windows::fs::FileTimesExt;

#[test]
#[ignore = "requires guest-local Windows Sandbox fixtures"]
fn file_operation_sandbox_moves_album_years_and_preserves_membership() {
    move_album_years(false);
}

#[test]
#[ignore = "requires guest-local Windows Sandbox fixtures and disposable R: volume"]
fn file_operation_sandbox_moves_album_years_across_volumes() {
    move_album_years(true);
}

fn move_album_years(cross_volume: bool) {
    assert!(
        std::env::var("USERNAME").is_ok_and(|name| name.eq_ignore_ascii_case("WDAGUtilityAccount"))
    );
    let base = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("AmeR9");
    fs::create_dir_all(&base).unwrap();
    let fixture = tempfile::Builder::new()
        .prefix("album-years-")
        .tempdir_in(base)
        .unwrap();
    let source = fixture.path().join("source");
    let external = cross_volume.then(|| {
        tempfile::Builder::new()
            .prefix("album-years-")
            .tempdir_in("R:\\")
            .unwrap()
    });
    let output_root = external.as_ref().map_or(source.as_path(), |dir| dir.path());
    let output = output_root.join("organized");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(output.join("旅行/2024")).unwrap();
    for (name, color) in [
        ("图片.png", [23, 86, 137]),
        ("较早.png", [128, 76, 35]),
        ("未归属.png", [71, 11, 131]),
    ] {
        RgbImage::from_pixel(16, 12, Rgb(color))
            .save(source.join(name))
            .unwrap();
    }
    // July 15 at 10:00 UTC in 2024 and 2023 keeps filesystem fallback away from timezone edges.
    for (name, unix_seconds) in [("图片.png", 1_721_037_600), ("较早.png", 1_689_415_200)] {
        let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(unix_seconds);
        fs::OpenOptions::new()
            .write(true)
            .open(source.join(name))
            .unwrap()
            .set_times(fs::FileTimes::new().set_created(time).set_modified(time))
            .unwrap();
    }
    let first = fs::read(source.join("图片.png")).unwrap();
    let second = fs::read(source.join("较早.png")).unwrap();
    let untouched = fs::read(source.join("未归属.png")).unwrap();
    let collision = output.join("旅行/2024/图片.png");
    fs::write(&collision, b"existing destination").unwrap();
    let storage = StoragePaths {
        catalog_path: fixture.path().join("catalog.sqlite3"),
        preview_root: fixture.path().join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: fixture.path().join("settings.sqlite3"),
    };
    scan_library::run_scan_with_storage(
        ScanRequest {
            scan_id: "year-fixture".into(),
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
    let destination_root = if cross_volume {
        scan_library::run_scan_with_storage(
            ScanRequest {
                scan_id: "year-target".into(),
                root_path: output_root.to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |_| true,
            storage.clone(),
        )
        .unwrap();
        SqliteFileOperationJournal::open(storage.catalog_path.clone())
            .unwrap()
            .publication_roots()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.root_id != root.root_id)
            .unwrap()
    } else {
        root.clone()
    };
    let locations: Vec<_> = ["图片.png", "较早.png", "未归属.png"]
        .iter()
        .map(|name| {
            catalog
                .load_incremental_location_by_relative_path(&root.root_id, name)
                .unwrap()
                .unwrap()
        })
        .collect();
    let album = catalog
        .create_organization_album("create", 0, "旅行", Default::default())
        .unwrap();
    let selected: Vec<_> = locations.iter().map(selection).collect();
    let prepared = catalog
        .prepare_organization_selection(&selected[..2])
        .unwrap();
    catalog
        .change_organization(
            "assign",
            &prepared,
            &OrganizationAction::AssignAlbum {
                album_id: album.album_id.clone(),
            },
        )
        .unwrap();
    let prepared = catalog
        .prepare_organization_selection(&selected[..2])
        .unwrap();
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let engine = WindowsFileOperations::new(None);
    let admitted = engine.inspect_directory(output.to_str().unwrap()).unwrap();
    assert!(
        engine
            .organization_directory(&admitted, &["must-not-exist", "../escape"])
            .is_err()
    );
    assert!(!output.join("must-not-exist").exists());
    let result = exports::transfer(
        &storage,
        &selected,
        &output,
        "year-move",
        FileOrganizationRule::AlbumYear,
        FileAction::Move,
    );
    assert_eq!(
        result.phase,
        FileOperationTaskPhase::Finished,
        "{:?}",
        result.failure
    );
    assert_eq!((result.published_items, result.skipped_items), (2, 1));
    assert_eq!(
        fs::read(output.join("旅行/2024/图片(1).png")).unwrap(),
        first
    );
    assert_eq!(fs::read(output.join("旅行/2023/较早.png")).unwrap(), second);
    assert_eq!(fs::read(&collision).unwrap(), b"existing destination");
    assert_eq!(fs::read(source.join("未归属.png")).unwrap(), untouched);
    assert!(!source.join("图片.png").exists());
    assert!(!source.join("较早.png").exists());
    let report = fs::read_to_string(exports::report_path(result)).unwrap();
    assert!(report.contains("图片(1).png") && report.contains("未归属.png"));
    drop(catalog);
    let catalog = SqliteCatalog::open(storage.catalog_path.clone()).unwrap();
    let mut moved_selections = Vec::new();
    for (relative, original) in [
        ("organized/旅行/2024/图片(1).png", &locations[0]),
        ("organized/旅行/2023/较早.png", &locations[1]),
    ] {
        let moved = catalog
            .load_incremental_location_by_relative_path(&destination_root.root_id, relative)
            .unwrap()
            .unwrap();
        assert_eq!(moved.asset_id, original.asset_id);
        assert_eq!(moved.created_unix_ms, original.created_unix_ms);
        assert_eq!(moved.modified_unix_ms, original.modified_unix_ms);
        let decisions = catalog
            .inspect_organization_selection(&[selection(&moved)])
            .unwrap();
        assert!(decisions.decisions[0].favorite);
        assert_eq!(
            decisions.decisions[0].album.as_ref().unwrap().album_id,
            album.album_id
        );
        moved_selections.push(selection(&moved));
    }
    let unchanged = exports::transfer(
        &storage,
        &moved_selections,
        &output,
        "already-organized",
        FileOrganizationRule::AlbumYear,
        FileAction::Move,
    );
    assert_eq!((unchanged.published_items, unchanged.skipped_items), (0, 2));
    assert_eq!(
        fs::read(output.join("旅行/2024/图片(1).png")).unwrap(),
        first
    );
    assert_eq!(fs::read(output.join("旅行/2023/较早.png")).unwrap(), second);
    assert!(!output.join("旅行/2024/图片(1)(1).png").exists());
    let journal = SqliteFileOperationJournal::open(storage.catalog_path).unwrap();
    assert_eq!(
        journal
            .load_export("year-move")
            .unwrap()
            .unwrap()
            .batch
            .rule,
        FileOrganizationRule::AlbumYear
    );
    assert_eq!(journal.batch_count("year-move").unwrap(), 2);
    println!(
        "Album/year move (cross_volume={cross_volume}): two calendar years, original file dates, numbered collision, same-folder skips, report, exact bytes and album/favorite membership retained"
    );
}
