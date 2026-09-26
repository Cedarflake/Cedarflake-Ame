use image::{Rgba, RgbaImage};
use rusqlite::Connection;

use super::*;

#[test]
fn explicit_first_import_resume_revisits_completed_directories_and_changed_files() {
    for suspend in [false, true] {
        let source = tempfile::tempdir().expect("source");
        let derived = tempfile::tempdir().expect("derived storage");
        for name in ["A", "B"] {
            let directory = source.path().join(name);
            std::fs::create_dir(&directory).expect("source directory");
            RgbaImage::from_pixel(4, 4, Rgba([40, 60, 80, 255]))
                .save(directory.join("original.png"))
                .expect("image");
        }
        let request = ScanRequest {
            scan_id: format!("resume-rebuild-{suspend}"),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        let storage = StoragePaths {
            catalog_path: derived.path().join("catalog.sqlite3"),
            preview_root: derived.path().join("previews"),
            preview_budget_bytes: 1024 * 1024,
            settings_path: derived.path().join("settings.sqlite3"),
        };
        let mut first_asset: Option<AssetLocationView> = None;
        let mut interrupted = false;
        run_scan_with_storage(
            request.clone(),
            |event| {
                if let ScanEvent::AssetDiscovered { asset, .. } = event {
                    if first_asset.is_some() {
                        interrupted = if suspend {
                            suspend_scan(&request.scan_id)
                        } else {
                            pause_scan(&request.scan_id)
                        };
                    } else {
                        first_asset = Some(*asset);
                    }
                }
                true
            },
            storage.clone(),
        )
        .expect("interrupt initial import");
        assert!(interrupted);
        let connection = Connection::open(&storage.catalog_path).expect("interrupted staging");
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM assets", [], |row| row
                    .get::<_, i64>(0))
                .expect("owned staging assets"),
            2
        );
        connection
            .execute(
                "INSERT INTO assets(id, created_unix_ms) VALUES ('unrelated-orphan-sentinel', 1)",
                [],
            )
            .expect("unrelated derived sentinel");
        drop(connection);
        let first_asset = first_asset.expect("already enumerated first directory");
        let first_path = Path::new(&first_asset.absolute_path);
        let completed_directory = first_path.parent().expect("parent");
        RgbaImage::from_pixel(7, 3, Rgba([80, 60, 40, 255]))
            .save(first_path)
            .expect("same path different bytes");
        RgbaImage::from_pixel(2, 5, Rgba([10, 20, 30, 255]))
            .save(completed_directory.join("added-while-detached.png"))
            .expect("new file in already visited directory");
        let mut events = Vec::new();
        resume_scan_with_storage(
            request.clone(),
            |event| {
                if matches!(event, ScanEvent::Progress { visited_entries: 0, accepted_items: 0, .. }) {
                    let connection = Connection::open(&storage.catalog_path).expect("before any rediscovered asset");
                    let staged: (i64, i64, i64) = connection.query_row("SELECT (SELECT COUNT(*) FROM assets), (SELECT COUNT(*) FROM asset_locations), (SELECT COUNT(*) FROM assets WHERE id = 'unrelated-orphan-sentinel')", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).expect("scoped resume reclamation");
                    assert_eq!(staged, (1, 0, 1), "resume releases only its old staging assets before publication can clean anything");
                }
                events.push(event);
                true
            },
            storage.clone(),
        )
        .expect("explicit rebuild");
        assert!(matches!(events.first(), Some(ScanEvent::Started { .. })));
        assert!(
            matches!(
                events.get(1),
                Some(ScanEvent::Progress {
                    visited_entries: 0,
                    accepted_items: 0,
                    issue_count: 0,
                    ..
                })
            ),
            "real zero progress precedes all discovered assets"
        );
        assert!(matches!(
            events.last(),
            Some(ScanEvent::Completed { asset_count: 3, .. })
        ));
        assert!(events.iter().any(|event| matches!(event, ScanEvent::AssetDiscovered { asset, .. } if asset.absolute_path == first_asset.absolute_path && asset.width == 7 && asset.height == 3)));
        let connection = Connection::open(&storage.catalog_path).expect("evidence");
        let counts: (i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM assets), (SELECT COUNT(*) FROM asset_locations)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("counts");
        assert_eq!(counts, (3, 3));
        SqliteCatalog::open(storage.catalog_path).expect("full reopen after rebuild");
    }
}

#[test]
fn explicitly_resumed_first_import_can_be_cancelled_before_any_asset_is_republished() {
    let source = tempfile::tempdir().expect("source");
    let derived = tempfile::tempdir().expect("derived");
    RgbaImage::from_pixel(4, 4, Rgba([40, 60, 80, 255]))
        .save(source.path().join("image.png"))
        .expect("image");
    let request = ScanRequest {
        scan_id: "resume-then-cancel".to_owned(),
        root_path: source.path().to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let storage = StoragePaths {
        catalog_path: derived.path().join("catalog.sqlite3"),
        preview_root: derived.path().join("previews"),
        preview_budget_bytes: 1024 * 1024,
        settings_path: derived.path().join("settings.sqlite3"),
    };
    run_scan_with_storage(
        request.clone(),
        |event| {
            if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                assert!(pause_scan(&request.scan_id));
            }
            true
        },
        storage.clone(),
    )
    .expect("pause");
    let mut events = Vec::new();
    resume_scan_with_storage(
        request.clone(),
        |event| {
            if matches!(event, ScanEvent::Started { .. }) {
                assert!(cancel_scan(&request.scan_id));
            }
            events.push(event);
            true
        },
        storage.clone(),
    )
    .expect("cancel resumed import");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ScanEvent::AssetDiscovered { .. }))
    );
    assert!(matches!(events.last(), Some(ScanEvent::Cancelled { .. })));
    let mut catalog = SqliteCatalog::open(storage.catalog_path).expect("valid retired catalog");
    assert!(
        catalog
            .load_single_recoverable_foreground_scan()
            .expect("recoverable")
            .is_none()
    );
    assert_eq!(
        catalog
            .load_inactive_first_import_roots()
            .expect("retained root")
            .len(),
        1
    );
}
