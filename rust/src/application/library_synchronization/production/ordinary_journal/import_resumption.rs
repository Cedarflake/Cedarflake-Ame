use super::*;
use crate::application::scan_library::{pause_scan, resume_scan_with_storage};

const SCAN_ID: &str = "ordinary-paused-first-import";
const EXPECTED: &[&str] = &[
    "renamed/added.png",
    "renamed/one.png",
    "renamed/two.png",
    "z-pending/keep-one.png",
    "z-pending/keep-two.png",
];

#[test]
fn paused_import_retains_its_checkpoint_and_catches_up_after_process_exit() {
    let directory = fixture_capability::journal_capable_fixture();
    let source = directory.path().join("source");
    write_png(&source.join("a-visited/one.png"), [160, 80, 40]);
    fs::hard_link(
        source.join("a-visited/one.png"),
        source.join("a-visited/two.png"),
    )
    .expect("generated alias");
    for (name, color) in [
        ("z-pending/gone.png", [60, 80, 100]),
        ("z-pending/keep-one.png", [80, 100, 120]),
        ("z-pending/keep-two.png", [100, 120, 140]),
    ] {
        write_png(&source.join(name), color);
    }
    super::run_worker(directory.path(), "paused-import");
    let catalog = directory.path().join("storage/catalog/ame.sqlite3");
    let original_asset: String = Connection::open(&catalog)
        .expect("paused catalog")
        .query_row(
            "SELECT asset_id FROM asset_locations WHERE relative_path = 'a-visited/one.png'",
            [],
            |row| row.get(0),
        )
        .expect("retained asset");
    write_png(&source.join("a-visited/one.png"), [20, 180, 220]);
    fs::rename(source.join("a-visited"), source.join("renamed")).expect("rename generated folder");
    write_png(&source.join("renamed/added.png"), [200, 60, 140]);
    fs::remove_file(source.join("z-pending/gone.png")).expect("remove generated fixture");
    let source_bytes: BTreeMap<_, _> = EXPECTED
        .iter()
        .map(|path| {
            (
                *path,
                fs::read(source.join(path)).expect("expected source bytes"),
            )
        })
        .collect();
    super::run_worker(directory.path(), "continued-import");
    let locations = active_locations(&catalog);
    assert_eq!(locations.get("renamed/one.png"), Some(&original_asset));
    assert_eq!(locations.get("renamed/two.png"), Some(&original_asset));
    for (path, bytes) in source_bytes {
        assert_eq!(
            fs::read(source.join(path)).expect("source after resume"),
            bytes
        );
    }
    SqliteCatalog::open(catalog).expect("full catalog validation after changed hardlink resume");
}

pub(super) fn is_phase(phase: &str) -> bool {
    matches!(phase, "paused-import" | "continued-import")
}

pub(super) fn run_worker(source: &Path, phase: &str) {
    let storage = storage_paths().expect("isolated storage");
    let root_path = FileDiscovery::new(&source.to_string_lossy())
        .expect("discovery")
        .canonical_root()
        .expect("root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &root_path);
    let request = ScanRequest {
        scan_id: SCAN_ID.to_owned(),
        root_path,
        max_items: None,
        max_entries: None,
        preview_edge: 96,
    };
    let ticket = reserve_production_library_synchronization_start_ticket().expect("ticket");
    start_production_library_synchronization(ticket).expect("ordinary startup");
    let deadline = Instant::now() + Duration::from_secs(45);
    if phase == "paused-import" {
        let mut inspected = 0;
        run_scan_with_storage(
            request,
            |event| {
                if matches!(event, ScanEvent::Started { .. }) {
                    loop {
                        poll_production_library_synchronization(ticket).expect("capture poll");
                        let catalog = SqliteCatalog::open(storage.catalog_path.clone())
                            .expect("capture catalog");
                        if catalog
                            .first_import_change_capture_is_ready(
                                SCAN_ID,
                                &root_id,
                                LibraryRootGeneration::initial(),
                            )
                            .expect("boundary")
                        {
                            break;
                        }
                        assert!(Instant::now() < deadline, "first-import capture expired");
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
                if matches!(event, ScanEvent::AssetDiscovered { .. }) {
                    inspected += 1;
                    if inspected == 2 {
                        assert!(pause_scan(SCAN_ID));
                    }
                }
                true
            },
            storage.clone(),
        )
        .expect("pause after two inspections");
        let connection = Connection::open(&storage.catalog_path).expect("paused state");
        let retained: (String, i64) = connection
            .query_row(
                "SELECT status, accepted_items FROM scan_runs WHERE id = ?1",
                [SCAN_ID],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("durable checkpoint");
        assert_eq!(retained, ("paused".to_owned(), 2));
    } else {
        let mut first_progress = None;
        let mut rediscovered = Vec::new();
        resume_scan_with_storage(
            request,
            |event| {
                if let ScanEvent::Progress { accepted_items, .. } = event {
                    first_progress.get_or_insert(accepted_items);
                }
                if let ScanEvent::AssetDiscovered { asset, .. } = event {
                    rediscovered.push(asset.relative_path);
                }
                true
            },
            storage.clone(),
        )
        .expect("continue covered checkpoint");
        assert_eq!(first_progress, Some(2), "Continue retains accepted work");
        assert!(
            rediscovered
                .iter()
                .all(|path| path.starts_with("z-pending/")),
            "completed directories must not be scanned again: {rediscovered:?}"
        );
        loop {
            poll_production_library_synchronization(ticket).expect("resume catch-up");
            if is_current(&storage, EXPECTED) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "resumed import did not converge: {}",
                convergence_state(&storage)
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let fence = reserve_production_library_synchronization_stop_fence().expect("stop fence");
    stop_production_library_synchronization(fence).expect("normal worker shutdown");
}
