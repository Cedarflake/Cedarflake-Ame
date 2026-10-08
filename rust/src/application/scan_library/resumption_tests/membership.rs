use super::*;

#[test]
fn repeated_resume_reconciles_added_changed_deleted_and_unvisited_files() {
    let source = tempfile::tempdir().expect("generated source");
    let derived = tempfile::tempdir().expect("isolated catalog");
    for directory in ["A", "B"] {
        std::fs::create_dir(source.path().join(directory)).expect("source directory");
    }
    for name in [
        "A/changed.png",
        "A/deleted.png",
        "A/unchanged.png",
        "B/unvisited.png",
    ] {
        RgbaImage::from_pixel(4, 4, Rgba([40, 60, 80, 255]))
            .save(source.path().join(name))
            .expect("generated image");
    }
    let request = ScanRequest {
        scan_id: "retained-membership".to_owned(),
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
    let mut unchanged_id = None;
    run_scan_with_storage(
        request.clone(),
        |event| {
            if let ScanEvent::AssetDiscovered { asset, .. } = event
                && asset.relative_path == "A/unchanged.png"
            {
                unchanged_id = Some(asset.asset_id);
                assert!(pause_scan(&request.scan_id));
            }
            true
        },
        storage.clone(),
    )
    .expect("pause after the first directory");
    let unchanged_id = unchanged_id.expect("retained unchanged image");
    RgbaImage::from_pixel(7, 3, Rgba([80, 60, 40, 255]))
        .save(source.path().join("A/changed.png"))
        .expect("changed generated image");
    std::fs::remove_file(source.path().join("A/deleted.png")).expect("deleted generated image");
    std::fs::create_dir(source.path().join("C")).expect("new generated directory");
    for name in ["A/added.png", "C/added.png"] {
        RgbaImage::from_pixel(2, 5, Rgba([10, 20, 30, 255]))
            .save(source.path().join(name))
            .expect("new generated image");
    }
    let names = [
        "A/added.png",
        "A/changed.png",
        "A/unchanged.png",
        "B/unvisited.png",
        "C/added.png",
    ];
    let bytes_before: Vec<_> = names
        .iter()
        .map(|name| std::fs::read(source.path().join(name)).expect("source bytes"))
        .collect();
    let mut second_pause = false;
    resume_scan_with_storage(
        request.clone(),
        |event| {
            if let ScanEvent::AssetDiscovered { asset, .. } = event
                && asset.relative_path == "A/changed.png"
            {
                second_pause = pause_scan(&request.scan_id);
            }
            true
        },
        storage.clone(),
    )
    .expect("interrupt namespace reconciliation");
    assert!(second_pause);
    drop(SqliteCatalog::open(storage.catalog_path.clone()).expect("full interrupted reopen"));
    let mut events = Vec::new();
    resume_scan_with_storage(
        request,
        |event| {
            events.push(event);
            true
        },
        storage.clone(),
    )
    .expect("complete retained import");
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Completed { asset_count: 5, .. })
    ));
    let connection = Connection::open(&storage.catalog_path).expect("closed membership");
    let mut statement = connection.prepare(
        "SELECT location.relative_path, location.asset_id, location.width, location.height
         FROM asset_locations AS location JOIN library_roots AS root ON root.active_scan_id = location.scan_id
         ORDER BY location.relative_path",
    ).expect("active membership query");
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .expect("active rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("valid rows");
    assert_eq!(
        rows.iter().map(|row| row.0.as_str()).collect::<Vec<_>>(),
        names
    );
    assert_eq!((rows[1].2, rows[1].3), (7, 3));
    assert_eq!(rows[2].1, unchanged_id);
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM scan_resume_pending_locations",
                [],
                |row| row.get::<_, i64>(0)
            )
            .expect("retired resume roster"),
        0
    );
    for (name, before) in names.iter().zip(bytes_before) {
        assert_eq!(
            std::fs::read(source.path().join(name)).expect("preserved source"),
            before
        );
    }
    drop(statement);
    drop(connection);
    drop(SqliteCatalog::open(storage.catalog_path).expect("full completed reopen"));
}
