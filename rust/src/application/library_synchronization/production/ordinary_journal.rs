use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rusqlite::Connection;

use super::{
    poll_production_library_synchronization,
    reserve_production_library_synchronization_start_ticket,
    reserve_production_library_synchronization_stop_fence,
    start_production_library_synchronization, stop_production_library_synchronization,
};
use crate::adapters::{FileDiscovery, SqliteCatalog};
use crate::application::scan_library::{run_scan_with_storage, stable_id};
use crate::application::storage::{StoragePaths, storage_paths};
use crate::domain::{LibraryRootGeneration, ScanEvent, ScanRequest};

const WORKER: &str =
    "application::library_synchronization::production::ordinary_journal::ordinary_client_worker";
const PHASE_LIMIT: Duration = Duration::from_secs(60);

fn write_png(path: &Path, color: [u8; 3]) {
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
    image::RgbImage::from_pixel(32, 24, image::Rgb(color))
        .save(path)
        .expect("generated PNG");
}

fn active_locations(catalog: &Path) -> BTreeMap<String, String> {
    let connection = Connection::open(catalog).expect("isolated catalog");
    let mut statement = connection
        .prepare(
            "SELECT location.relative_path, location.asset_id
             FROM asset_locations AS location JOIN library_roots AS root
               ON root.id = location.root_id AND root.active_scan_id = location.scan_id
             ORDER BY location.relative_path",
        )
        .expect("active membership query");
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("active membership")
        .collect::<Result<_, _>>()
        .expect("valid active membership")
}

fn run_worker(directory: &Path, phase: &str) {
    let log_path = directory.join(format!("{phase}.log"));
    let stdout = fs::File::create(&log_path).expect("owned worker output");
    let stderr = stdout.try_clone().expect("owned worker error output");
    let mut child = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            WORKER,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(
            "CEDARFLAKE_AME_TEST_STORAGE_ROOT",
            directory.join("storage"),
        )
        .env(
            "CEDARFLAKE_AME_ORDINARY_JOURNAL_SOURCE",
            directory.join("source"),
        )
        .env("CEDARFLAKE_AME_ORDINARY_JOURNAL_PHASE", phase)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .expect("ordinary-client subprocess");
    let deadline = Instant::now() + PHASE_LIMIT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("owned subprocess status") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("terminate owned expired subprocess");
            child.wait().expect("reap owned subprocess");
            panic!("ordinary journal phase {phase} exceeded {PHASE_LIMIT:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "ordinary journal phase {phase} failed:\n{}",
        fs::read_to_string(log_path).expect("owned worker diagnostics")
    );
}

#[test]
fn real_journal_converges_across_process_exit_and_unchanged_restart() {
    let directory = tempfile::tempdir_in(std::env::current_dir().expect("working directory"))
        .expect("owned NTFS fixture");
    let source = directory.path().join("source");
    for (name, color) in [
        ("delete.png", [180, 20, 20]),
        ("replace.png", [20, 180, 20]),
        ("old/child.png", [20, 20, 180]),
        ("links/one.png", [160, 90, 30]),
    ] {
        write_png(&source.join(name), color);
    }
    fs::hard_link(source.join("links/one.png"), source.join("links/two.png"))
        .expect("generated hardlink");
    write_png(
        &directory.path().join("outside/incoming/arrive.png"),
        [80, 120, 180],
    );
    run_worker(directory.path(), "import");
    let catalog = directory.path().join("storage/catalog/ame.sqlite3");
    let before = active_locations(&catalog);
    assert_eq!(before.len(), 5);
    fs::remove_file(source.join("delete.png")).expect("remove generated fixture");
    fs::remove_file(source.join("replace.png")).expect("replace generated fixture");
    write_png(&source.join("replace.png"), [20, 200, 200]);
    fs::rename(source.join("old"), source.join("renamed")).expect("rename generated directory");
    fs::rename(
        directory.path().join("outside/incoming"),
        source.join("imported"),
    )
    .expect("move generated directory into root");
    fs::hard_link(source.join("links/one.png"), source.join("added-alias.png"))
        .expect("new generated alias");
    fs::remove_file(source.join("links/two.png")).expect("remove generated alias");
    write_png(&source.join("new.png"), [200, 20, 200]);
    let expected_bytes: BTreeMap<_, _> = expected_paths("catch-up")
        .into_iter()
        .map(|path| {
            (
                path,
                fs::read(source.join(path)).expect("final generated bytes"),
            )
        })
        .collect();
    run_worker(directory.path(), "catch-up");
    let after = active_locations(&catalog);
    assert_eq!(after.get("renamed/child.png"), before.get("old/child.png"));
    assert_eq!(after.get("added-alias.png"), before.get("links/one.png"));
    assert_ne!(after.get("replace.png"), before.get("replace.png"));
    run_worker(directory.path(), "unchanged");
    assert_eq!(active_locations(&catalog), after);
    for (path, bytes) in expected_bytes {
        assert_eq!(
            fs::read(source.join(path)).expect("source after client"),
            bytes
        );
    }
}

fn expected_paths(phase: &str) -> Vec<&'static str> {
    if phase == "import" {
        vec![
            "delete.png",
            "links/one.png",
            "links/two.png",
            "old/child.png",
            "replace.png",
        ]
    } else {
        vec![
            "added-alias.png",
            "imported/arrive.png",
            "links/one.png",
            "new.png",
            "renamed/child.png",
            "replace.png",
        ]
    }
}

fn is_current(storage: &StoragePaths, expected: &[&str]) -> bool {
    let connection = Connection::open(&storage.catalog_path).expect("isolated current-state query");
    let current: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_persistent_journal_root_state
               WHERE capability_state = 'supported' AND continuity_state = 'current')
             AND NOT EXISTS(SELECT 1 FROM library_change_queue
               WHERE status NOT IN ('completed', 'superseded'))
             AND NOT EXISTS(SELECT 1 FROM library_persistent_journal_baselines
               WHERE phase <> 'completed')",
            [],
            |row| row.get(0),
        )
        .expect("current continuity and retired work");
    current
        && active_locations(&storage.catalog_path)
            .keys()
            .map(String::as_str)
            .eq(expected.iter().copied())
}

#[test]
#[ignore = "Executed by the real-journal parent in three isolated processes"]
fn ordinary_client_worker() {
    let source = PathBuf::from(
        std::env::var_os("CEDARFLAKE_AME_ORDINARY_JOURNAL_SOURCE").expect("owned source argument"),
    );
    let phase = std::env::var("CEDARFLAKE_AME_ORDINARY_JOURNAL_PHASE").expect("worker phase");
    assert!(matches!(
        phase.as_str(),
        "import" | "catch-up" | "unchanged"
    ));
    let storage = storage_paths().expect("isolated worker storage");
    let root_path = FileDiscovery::new(&source.to_string_lossy())
        .expect("NTFS root discovery")
        .canonical_root()
        .expect("canonical owned root")
        .to_string_lossy()
        .into_owned();
    let root_id = stable_id("library-root-v1", &root_path);
    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
    crate::adapters::reset_source_content_open_instrumentation(&root_path);
    let ticket = reserve_production_library_synchronization_start_ticket().expect("start ticket");
    start_production_library_synchronization(ticket).expect("ordinary production startup");
    let deadline = Instant::now() + Duration::from_secs(45);
    if phase == "import" {
        run_scan_with_storage(
            ScanRequest {
                scan_id: "ordinary-native-first-import".to_owned(),
                root_path: root_path.clone(),
                max_items: Some(8),
                max_entries: Some(32),
                preview_edge: 64,
            },
            |event| {
                if let ScanEvent::Started { .. } = event {
                    loop {
                        poll_production_library_synchronization(ticket)
                            .expect("native first-import capture");
                        let catalog = SqliteCatalog::open(storage.catalog_path.clone())
                            .expect("capture catalog");
                        if catalog
                            .first_import_change_capture_is_ready(
                                "ordinary-native-first-import",
                                &root_id,
                                LibraryRootGeneration::initial(),
                            )
                            .expect("real first-import boundary")
                        {
                            break;
                        }
                        assert!(
                            Instant::now() < deadline,
                            "native first-import capture expired"
                        );
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
                true
            },
            storage.clone(),
        )
        .expect("scan sharing its real first-import capture");
    }
    let expected = expected_paths(&phase);
    loop {
        poll_production_library_synchronization(ticket).expect("ordinary production catch-up");
        if is_current(&storage, &expected) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "phase {phase} failed to converge through the real journal"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let fence = reserve_production_library_synchronization_stop_fence().expect("stop fence");
    stop_production_library_synchronization(fence).expect("ordinary process shutdown");
    let connection = Connection::open(&storage.catalog_path).expect("closed catalog");
    let scans: u64 = connection
        .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
        .expect("scan count");
    let inventories: u64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_runs",
            [],
            |row| row.get(0),
        )
        .expect("inventory count");
    assert_eq!(scans, 1, "restart cannot create another scan");
    assert_eq!(
        inventories, 0,
        "precise journal work cannot become root inventory"
    );
    let entries = crate::adapters::source_entry_read_count(&root_path);
    let content = crate::adapters::source_content_open_count(&root_path);
    if phase == "unchanged" {
        assert_eq!(entries, 0, "unchanged restart must not enumerate sources");
        assert_eq!(content, 0, "unchanged restart must not open media");
    }
    println!(
        "ordinary phase={phase} scans={scans} inventories={inventories} entries={entries} content={content}"
    );
}
