use super::*;

mod legacy;

fn paused_rewritten_hard_links() -> PausedHardLinks {
    let source = tempfile::tempdir().expect("generated hard links");
    let derived = tempfile::tempdir().expect("isolated catalog");
    for directory in ["A", "B"] {
        std::fs::create_dir(source.path().join(directory)).expect("source directory");
    }
    let first = source.path().join("A/image.png");
    let second = source.path().join("B/image.png");
    RgbaImage::from_pixel(4, 4, Rgba([40, 60, 80, 255]))
        .save(&first)
        .expect("original generated image");
    std::fs::hard_link(&first, &second).expect("same physical file in another directory");
    let request = ScanRequest {
        scan_id: "hard-link-resume".to_owned(),
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
    let mut initial = Vec::new();
    run_scan_with_storage(
        request.clone(),
        |event| {
            if let ScanEvent::AssetDiscovered { asset, .. } = event {
                initial.push(*asset);
                if initial.len() == 2 {
                    assert!(pause_scan(&request.scan_id));
                }
            }
            true
        },
        storage.clone(),
    )
    .expect("pause after both aliases were inspected");
    assert_eq!(initial.len(), 2);
    assert!(initial[0].file_identity.is_some());
    assert_eq!(initial[0].file_identity, initial[1].file_identity);
    assert_eq!(initial[0].asset_id, initial[1].asset_id);
    drop(SqliteCatalog::open(storage.catalog_path.clone()).expect("first paused reopen"));
    let connection = Connection::open(&storage.catalog_path).expect("persisted original revision");
    let mut original = initial[0].clone();
    let (generation, revision): (i64, String) = connection
        .query_row(
            "SELECT source_generation, source_revision_token FROM asset_locations
             WHERE scan_id = ?1 AND location_id = ?2",
            [&request.scan_id, &original.location_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("original durable source state");
    original.source_generation = u64::try_from(generation).expect("positive source generation");
    let evidence = original
        .source_revision
        .as_ref()
        .expect("original revision evidence");
    assert_eq!(revision, format!("{}:{}", evidence.scheme, evidence.value));
    drop(connection);

    RgbaImage::from_pixel(7, 3, Rgba([80, 60, 40, 255]))
        .save(&first)
        .expect("rewrite the generated file in place");
    let current_bytes = std::fs::read(&first).expect("current source bytes");
    assert_eq!(
        std::fs::read(&second).expect("hard-link bytes"),
        current_bytes
    );
    let mut revisited = Vec::new();
    resume_scan_with_storage(
        request.clone(),
        |event| {
            if let ScanEvent::AssetDiscovered { asset, .. } = event {
                assert_eq!(asset.file_identity, initial[0].file_identity);
                assert_eq!((asset.width, asset.height), (7, 3));
                revisited.push(asset.relative_path);
                assert!(pause_scan(&request.scan_id));
            }
            true
        },
        storage.clone(),
    )
    .expect("pause after only one alias was revisited");
    assert_eq!(revisited, ["A/image.png"]);
    drop(SqliteCatalog::open(storage.catalog_path.clone()).expect("second paused full reopen"));

    let connection = Connection::open(&storage.catalog_path).expect("paused membership");
    let membership: (i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM scan_resume_pending_locations),
                (SELECT COUNT(*) FROM library_roots WHERE active_scan_id IS NOT NULL)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("unvisited aliases remain unpublished");
    assert_eq!(membership, (1, 0));
    drop(connection);
    PausedHardLinks {
        _source: source,
        _derived: derived,
        request,
        storage,
        paths: [first, second],
        current_bytes,
        original,
    }
}

struct PausedHardLinks {
    _source: tempfile::TempDir,
    _derived: tempfile::TempDir,
    request: ScanRequest,
    storage: StoragePaths,
    paths: [std::path::PathBuf; 2],
    current_bytes: Vec<u8>,
    original: AssetLocationView,
}

#[test]
fn rewritten_hard_link_resume_can_pause_reopen_and_finish() {
    let fixture = paused_rewritten_hard_links();
    let request = fixture.request;
    let storage = fixture.storage;
    let current_bytes = fixture.current_bytes;
    let [first, second] = fixture.paths;
    let mut completed = false;
    resume_scan_with_storage(
        request,
        |event| {
            completed |= matches!(event, ScanEvent::Completed { .. });
            true
        },
        storage.clone(),
    )
    .expect("complete the retained hard-link import");
    assert!(completed);
    let connection = Connection::open(&storage.catalog_path).expect("closed membership");
    let state: (i64, i64, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT COUNT(*), COUNT(DISTINCT asset_id), COUNT(DISTINCT source_generation),
                    COUNT(DISTINCT source_revision_token), MIN(width), MIN(height)
             FROM asset_locations AS location
             JOIN library_roots AS root ON root.active_scan_id = location.scan_id",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("consistent published aliases");
    assert_eq!(state, (2, 1, 1, 1, 7, 3));
    drop(connection);
    drop(SqliteCatalog::open(storage.catalog_path).expect("completed full reopen"));
    for path in [&first, &second] {
        assert_eq!(
            std::fs::read(path).expect("preserved source"),
            current_bytes
        );
    }
}

#[test]
fn rewritten_hard_link_resume_can_be_cancelled_after_reopen() {
    let fixture = paused_rewritten_hard_links();
    let mut cancelled = false;
    resume_scan_with_storage(
        fixture.request.clone(),
        |event| {
            if matches!(event, ScanEvent::Started { .. }) {
                assert!(cancel_scan(&fixture.request.scan_id));
            }
            assert!(!matches!(event, ScanEvent::AssetDiscovered { .. }));
            cancelled |= matches!(event, ScanEvent::Cancelled { .. });
            true
        },
        fixture.storage.clone(),
    )
    .expect("cancel the reopened import");
    assert!(cancelled);
    let mut catalog =
        SqliteCatalog::open(fixture.storage.catalog_path).expect("cancelled full reopen");
    assert!(
        catalog
            .load_single_recoverable_foreground_scan()
            .expect("recovery state")
            .is_none()
    );
    for path in fixture.paths {
        assert_eq!(
            std::fs::read(path).expect("preserved source"),
            fixture.current_bytes
        );
    }
}

#[test]
fn rewritten_retained_hard_link_still_requires_current_namespace_membership() {
    let fixture = paused_rewritten_hard_links();
    std::fs::remove_file(&fixture.paths[1]).expect("remove the generated unvisited alias");
    let mut completed = false;
    resume_scan_with_storage(
        fixture.request,
        |event| {
            completed |= matches!(event, ScanEvent::Completed { .. });
            true
        },
        fixture.storage.clone(),
    )
    .expect("reconcile the removed generated alias");
    assert!(completed);
    let connection = Connection::open(&fixture.storage.catalog_path).expect("closed membership");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations AS location
         JOIN library_roots AS root ON root.active_scan_id = location.scan_id",
            [],
            |row| row.get(0),
        )
        .expect("published surviving alias");
    assert_eq!(count, 1);
    drop(connection);
    drop(SqliteCatalog::open(fixture.storage.catalog_path).expect("completed full reopen"));
    assert_eq!(
        std::fs::read(&fixture.paths[0]).expect("surviving source bytes"),
        fixture.current_bytes
    );
    assert!(!fixture.paths[1].exists());
}
