#![cfg(windows)]

use std::collections::BTreeMap;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use image::{Rgb, RgbImage};
use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};
use tempfile::tempdir;

use crate::adapters::{LocalMetadataInventory, SqliteCatalog};
use crate::application::metadata_inventory::run_metadata_inventory_with_source_for_test;
use crate::application::scan_library::{run_scan_with_storage, stable_id};
use crate::application::storage::{
    StoragePaths, resolved_path_is_within, resolved_paths_overlap, resolved_paths_same,
};
use crate::domain::{
    AssetLocationView, CatalogFreshnessState, GalleryQuery, IncrementalCatalogRoot,
    LibraryChangeQueuePolicy, LibraryChangeSourceHealth, LibrarySynchronizationSnapshot,
    MetadataInventoryPage, MetadataInventoryRunRequest, MetadataInventoryScope, ScanError,
    ScanRequest,
};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository, MetadataInventorySource};

use super::production::ProductionSynchronizationTestHarness;

const CONSENT_TOKEN: &str = "CEDARFLAKE_AME_R2C_REPLACEMENT_ACCEPTANCE_V1";
const EVENT_CYCLE_COUNT: usize = 5;
const STORM_PATH_COUNT: usize = 96;
const MAX_SOURCE_ENTRIES_PER_ROOT: usize = 250_000;
const EVENT_P95_LIMIT_MS: u64 = 1_000;
const METADATA_ROOT_LIMIT_MS: u64 = 45_000;
const CACHED_GALLERY_LIMIT_MS: u64 = 1_000;
const INITIAL_GALLERY_ITEMS: u32 = 500;
const INITIAL_MANIFEST_ITEMS: u32 = 4_096;
const PRODUCTION_POLL_INTERVAL: Duration = Duration::from_millis(250);
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
const CONTROLLED_PROCESS_WORKER_TEST: &str = "application::library_synchronization::replacement_reliability_acceptance::r2c_m_controlled_process_worker";
const TARGET_MEASUREMENT_WORKER_TEST: &str = "application::library_synchronization::replacement_reliability_acceptance::r2c_m_target_measurement_worker";

#[test]
#[ignore = "requires the serial Windows R2c-M replacement reliability wrapper"]
fn r2c_m_controlled_replacement_reliability_acceptance() {
    assert_eq!(
        required_environment("CEDARFLAKE_AME_R2C_M_CONSENT")
            .expect("R2c-M controlled authorization"),
        CONSENT_TOKEN,
        "the exact current R2c-M authorization token is required"
    );
    let report_path = report_path().expect("R2c-M report path");
    let fixture = ControlledFixture::new();
    let scan_rows_before =
        scan_row_count(&fixture.storage.catalog_path).expect("initial scan row count");
    let mut synchronization = ProductionSynchronizationTestHarness::new(fixture.storage.clone());

    let startup_started = Instant::now();
    wait_for(Duration::from_secs(15), PRODUCTION_POLL_INTERVAL, || {
        synchronization
            .poll()
            .is_ok_and(|snapshot| synchronization_is_current(&snapshot))
    });
    let startup_ms = elapsed_millis(startup_started.elapsed());

    let mut event_visible_micros = Vec::with_capacity(EVENT_CYCLE_COUNT * 5);
    let mut maximum_poll_micros = 0_u64;
    for index in 0..EVENT_CYCLE_COUNT {
        let source_relative = format!("live/source-{index:02}.png");
        let source_path = fixture.source_root.join(&source_relative);
        fs::create_dir_all(source_path.parent().expect("source parent"))
            .expect("create source parent");
        event_visible_micros.push(measure_visible_change(
            &mut synchronization,
            &fixture,
            &mut maximum_poll_micros,
            || write_png(&source_path, 2, 2, index as u8),
            |catalog| location(catalog, &fixture.root_id, &source_relative).is_some(),
        ));

        let previous = fixture
            .location(&source_relative)
            .expect("created source location");
        event_visible_micros.push(measure_visible_change(
            &mut synchronization,
            &fixture,
            &mut maximum_poll_micros,
            || write_png(&source_path, 7, 5, index.wrapping_add(31) as u8),
            |catalog| {
                location(catalog, &fixture.root_id, &source_relative)
                    .is_some_and(|current| file_state_changed(&previous, &current))
            },
        ));

        let destination_relative = if index % 2 == 0 {
            format!("live/renamed-{index:02}.png")
        } else {
            format!("moved/cycle-{index:02}/item.png")
        };
        let destination_path = fixture.source_root.join(&destination_relative);
        fs::create_dir_all(destination_path.parent().expect("destination parent"))
            .expect("create destination parent");
        event_visible_micros.push(measure_visible_change(
            &mut synchronization,
            &fixture,
            &mut maximum_poll_micros,
            || fs::rename(&source_path, &destination_path).expect("rename controlled source"),
            |catalog| {
                location(catalog, &fixture.root_id, &source_relative).is_none()
                    && location(catalog, &fixture.root_id, &destination_relative).is_some()
            },
        ));

        let before_replacement = fixture
            .location(&destination_relative)
            .expect("renamed destination location");
        event_visible_micros.push(measure_visible_change(
            &mut synchronization,
            &fixture,
            &mut maximum_poll_micros,
            || {
                fs::remove_file(&destination_path).expect("remove replacement predecessor");
                write_png(&destination_path, 13, 11, index.wrapping_add(97) as u8);
            },
            |catalog| {
                location(catalog, &fixture.root_id, &destination_relative)
                    .is_some_and(|current| file_state_changed(&before_replacement, &current))
            },
        ));

        event_visible_micros.push(measure_visible_change(
            &mut synchronization,
            &fixture,
            &mut maximum_poll_micros,
            || fs::remove_file(&destination_path).expect("delete controlled source"),
            |catalog| location(catalog, &fixture.root_id, &destination_relative).is_none(),
        ));
    }

    let storm_baseline = root_queue_evidence(&fixture.storage.catalog_path, &fixture.root_id)
        .expect("pre-storm queue evidence");
    for index in 0..STORM_PATH_COUNT {
        let path = fixture
            .source_root
            .join(format!("storm/item-{index:04}.png"));
        fs::create_dir_all(path.parent().expect("storm parent")).expect("create storm parent");
        write_png(&path, 2, 2, index as u8);
        write_png(&path, 3, 3, index.wrapping_add(1) as u8);
        write_png(&path, 4, 4, index.wrapping_add(2) as u8);
    }
    let mut last_queue_delta = (0_u64, 0_u64);
    let mut stable_since = Instant::now();
    wait_for(Duration::from_secs(30), PRODUCTION_POLL_INTERVAL, || {
        let poll_started = Instant::now();
        let snapshot = synchronization.poll().expect("storm synchronization poll");
        maximum_poll_micros = maximum_poll_micros.max(elapsed_micros(poll_started.elapsed()));
        let cumulative = root_queue_evidence(&fixture.storage.catalog_path, &fixture.root_id)
            .expect("storm queue evidence");
        let delta = queue_evidence_delta(storm_baseline, cumulative);
        if delta != last_queue_delta {
            last_queue_delta = delta;
            stable_since = Instant::now();
        }
        let catalog = fixture.open_catalog();
        let every_storm_path_is_visible = (0..STORM_PATH_COUNT).all(|index| {
            location(
                &catalog,
                &fixture.root_id,
                &format!("storm/item-{index:04}.png"),
            )
            .is_some()
        });
        delta.1 >= STORM_PATH_COUNT as u64
            && delta.1 > delta.0
            && every_storm_path_is_visible
            && synchronization_is_current(&snapshot)
            && stable_since.elapsed() >= Duration::from_millis(500)
    });
    let (storm_queue_rows, storm_coalesced_observations) = last_queue_delta;
    assert!(storm_queue_rows <= STORM_PATH_COUNT as u64);

    let stop_started = Instant::now();
    synchronization
        .stop()
        .expect("stop controlled production synchronization");
    let stop_ms = elapsed_millis(stop_started.elapsed());
    assert!(stop_ms < 5_000, "observer shutdown exceeded five seconds");

    run_controlled_process_worker(&fixture, "interrupt");

    let offline_added = fixture.source_root.join("offline/新增.png");
    fs::create_dir_all(offline_added.parent().expect("offline parent"))
        .expect("create offline parent");
    write_png(&offline_added, 5, 7, 181);
    let offline_moved = fixture.source_root.join("offline/moved.png");
    fs::rename(
        fixture.source_root.join("storm/item-0000.png"),
        &offline_moved,
    )
    .expect("move source while stopped");
    fs::remove_file(fixture.source_root.join("storm/item-0001.png"))
        .expect("remove source while stopped");
    write_png(&fixture.source_root.join("storm/item-0002.png"), 9, 9, 193);

    let restart_started = Instant::now();
    run_controlled_process_worker(&fixture, "recover");
    let restart_ms = elapsed_millis(restart_started.elapsed());

    let recovered_catalog = fixture.open_catalog();
    assert!(controlled_recovery_is_visible(
        &recovered_catalog,
        &fixture.root_id
    ));

    let scan_rows_after =
        scan_row_count(&fixture.storage.catalog_path).expect("final scan row count");
    assert_eq!(
        scan_rows_before, scan_rows_after,
        "continuity started a full scan"
    );
    let catalog_bytes = sqlite_family_bytes(&fixture.storage.catalog_path);
    let event_p50_ms = percentile_millis(&mut event_visible_micros, 50);
    let event_p95_ms = percentile_millis(&mut event_visible_micros, 95);
    assert!(
        event_p95_ms <= EVENT_P95_LIMIT_MS,
        "event visibility P95 was {event_p95_ms} ms and exceeded one second"
    );
    append_report(
        &report_path,
        &format!(
            "AME_R2C_M_CONTROLLED status=passed startup_ms={startup_ms} event_samples={} event_p50_ms={event_p50_ms} event_p95_ms={event_p95_ms} storm_paths={STORM_PATH_COUNT} storm_queue_rows={storm_queue_rows} storm_coalesced_observations={storm_coalesced_observations} max_poll_ms={} restart_ms={restart_ms} stop_ms={stop_ms} scan_rows_unchanged=true catalog_bytes={catalog_bytes}",
            event_visible_micros.len(),
            maximum_poll_micros.div_ceil(1_000),
        ),
    )
    .expect("write controlled replacement report");
}

#[test]
#[ignore = "launched as an isolated child process by the R2c-M controlled acceptance"]
fn r2c_m_controlled_process_worker() {
    let Ok(phase) = std::env::var("CEDARFLAKE_AME_R2C_M_CONTROLLED_WORKER_PHASE") else {
        return;
    };
    let storage = StoragePaths {
        catalog_path: absolute_environment("CEDARFLAKE_AME_R2C_M_CONTROLLED_CATALOG")
            .expect("controlled worker catalog path"),
        preview_root: absolute_environment("CEDARFLAKE_AME_R2C_M_CONTROLLED_PREVIEWS")
            .expect("controlled worker preview path"),
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path: absolute_environment("CEDARFLAKE_AME_R2C_M_CONTROLLED_SETTINGS")
            .expect("controlled worker settings path"),
    };
    let root_id = required_environment("CEDARFLAKE_AME_R2C_M_CONTROLLED_ROOT_ID")
        .expect("controlled worker root id");
    let mut synchronization = ProductionSynchronizationTestHarness::new(storage.clone());
    match phase.as_str() {
        "interrupt" => {
            wait_for(Duration::from_secs(15), PRODUCTION_POLL_INTERVAL, || {
                synchronization
                    .poll()
                    .is_ok_and(|snapshot| synchronization_is_current(&snapshot))
            });
            std::mem::forget(synchronization);
        }
        "recover" => {
            wait_for(Duration::from_secs(60), PRODUCTION_POLL_INTERVAL, || {
                let snapshot = synchronization.poll().expect("process recovery poll");
                let catalog = SqliteCatalog::open(storage.catalog_path.clone())
                    .expect("process recovery catalog");
                controlled_recovery_is_visible(&catalog, &root_id)
                    && synchronization_is_current(&snapshot)
            });
            synchronization
                .stop()
                .expect("stop process recovery synchronization");
        }
        _ => panic!("unsupported controlled worker phase"),
    }
}

#[test]
#[ignore = "requires current explicit authorization for both retained read-only roots"]
fn r2c_m_user_authorized_metadata_inventory_reliability_acceptance() {
    let configuration = acceptance_configuration().expect("R2c-M acceptance authorization");
    let report_path = report_path().expect("R2c-M report path");
    run_metadata_inventory_acceptance(&configuration, &report_path);
}

#[test]
#[ignore = "launched as a fresh process after the retained catalog backup is prepared"]
fn r2c_m_target_measurement_worker() {
    let Ok(isolated_catalog) = std::env::var("CEDARFLAKE_AME_R2C_M_TARGET_PREPARED_CATALOG") else {
        return;
    };
    let isolated_catalog = PathBuf::from(isolated_catalog);
    let roots = [
        (
            "local-primary",
            absolute_directory_environment("CEDARFLAKE_AME_R2C_M_TARGET_LOCAL_ROOT")
                .expect("target worker local root"),
        ),
        (
            "cloud-primary",
            absolute_directory_environment("CEDARFLAKE_AME_R2C_M_TARGET_CLOUD_ROOT")
                .expect("target worker cloud root"),
        ),
    ];
    let report_path =
        absolute_environment("CEDARFLAKE_AME_R2C_M_TARGET_REPORT").expect("target worker report");
    let backup_ms = required_environment("CEDARFLAKE_AME_R2C_M_TARGET_BACKUP_MS")
        .and_then(|value| {
            value
                .parse::<u64>()
                .map_err(|_| "target worker backup duration is invalid".to_owned())
        })
        .expect("target worker backup duration");
    run_prepared_metadata_inventory_acceptance(&isolated_catalog, &roots, &report_path, backup_ms);
}

#[test]
fn r2c_m_small_metadata_inventory_reliability_fixture() {
    let fixture = tempdir().expect("small replacement fixture");
    let local_root = fixture.path().join("local");
    let cloud_root = fixture.path().join("cloud");
    let unrelated_root = fixture.path().join("unrelated");
    let storage_root = fixture.path().join("acceptance");
    let source_catalog = fixture.path().join("retained.sqlite3");
    let preview_root = fixture.path().join("previews");
    let settings_path = fixture.path().join("settings.sqlite3");
    fs::create_dir_all(&local_root).expect("small local root");
    fs::create_dir_all(&cloud_root).expect("small cloud root");
    fs::create_dir_all(&unrelated_root).expect("small unrelated root");
    fs::create_dir_all(&storage_root).expect("small acceptance storage");
    for root in [&local_root, &cloud_root] {
        write_png(&root.join("keep.png"), 2, 2, 11);
        write_png(&root.join("remove.png"), 3, 3, 23);
    }
    write_png(&unrelated_root.join("outside-scope.png"), 2, 2, 71);
    let storage = StoragePaths {
        catalog_path: source_catalog.clone(),
        preview_root,
        preview_budget_bytes: 64 * 1024 * 1024,
        settings_path,
    };
    for root in [&local_root, &cloud_root, &unrelated_root] {
        let root_path = root.canonicalize().expect("canonical small root");
        run_scan_with_storage(
            ScanRequest {
                scan_id: stable_id("r2c-m-small-scan-v1", &root_path.to_string_lossy()),
                root_path: root_path.to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |_| true,
            storage.clone(),
        )
        .expect("initial small retained scan");
    }
    write_png(&local_root.join("keep.png"), 8, 8, 41);
    fs::remove_file(local_root.join("remove.png")).expect("small removal");
    write_png(&local_root.join("new.png"), 4, 5, 53);
    fs::rename(
        cloud_root.join("remove.png"),
        cloud_root.join("renamed.png"),
    )
    .expect("small rename");
    let configuration = AcceptanceConfiguration {
        source_catalog,
        storage_root: storage_root.clone(),
        local_root: local_root.canonicalize().expect("canonical local root"),
        cloud_root: cloud_root.canonicalize().expect("canonical cloud root"),
    };
    let report_path = storage_root.join("small-report.log");

    run_metadata_inventory_acceptance(&configuration, &report_path);

    let report = fs::read_to_string(report_path).expect("small replacement report");
    assert!(report.contains("AME_R2C_M_REAL status=passed"));
    assert!(report.contains("repeated_inventory_source_metadata_unchanged=true"));
    assert!(report.contains("cold_inventory_source_snapshot=not_measured"));
    assert!(report.contains("full_scan_rows_unchanged=true"));

    let isolated_catalog = storage_root.join("replacement-catalog").join("ame.sqlite3");
    let unrelated_path = unrelated_root
        .canonicalize()
        .expect("canonical unrelated root")
        .to_string_lossy()
        .into_owned();
    let connection = Connection::open(isolated_catalog).expect("isolated replacement catalog");
    let unrelated_root_id: String = connection
        .query_row(
            "SELECT id FROM library_roots WHERE path = ?1",
            [&unrelated_path],
            |row| row.get(0),
        )
        .expect("unrelated retained root");
    let unrelated_inventory_runs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_runs WHERE root_id = ?1",
            [&unrelated_root_id],
            |row| row.get(0),
        )
        .expect("unrelated inventory run count");
    assert_eq!(
        unrelated_inventory_runs, 0,
        "an unapproved retained root entered the target measurement"
    );
}

fn run_metadata_inventory_acceptance(configuration: &AcceptanceConfiguration, report_path: &Path) {
    let isolated_root = configuration.storage_root.join("replacement-catalog");
    ensure_fresh_directory(&isolated_root).expect("fresh isolated R2c-M storage");
    let isolated_catalog = isolated_root.join("ame.sqlite3");
    let backup_started = Instant::now();
    backup_catalog(&configuration.source_catalog, &isolated_catalog)
        .expect("read-only retained catalog backup");
    let backup_ms = elapsed_millis(backup_started.elapsed());

    let status = Command::new(std::env::current_exe().expect("current R2c-M test executable"))
        .args([
            "--exact",
            TARGET_MEASUREMENT_WORKER_TEST,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(
            "CEDARFLAKE_AME_R2C_M_TARGET_PREPARED_CATALOG",
            &isolated_catalog,
        )
        .env(
            "CEDARFLAKE_AME_R2C_M_TARGET_LOCAL_ROOT",
            &configuration.local_root,
        )
        .env(
            "CEDARFLAKE_AME_R2C_M_TARGET_CLOUD_ROOT",
            &configuration.cloud_root,
        )
        .env("CEDARFLAKE_AME_R2C_M_TARGET_REPORT", report_path)
        .env(
            "CEDARFLAKE_AME_R2C_M_TARGET_BACKUP_MS",
            backup_ms.to_string(),
        )
        .status()
        .expect("launch fresh target measurement process");
    assert!(status.success(), "target measurement process failed");
}

fn run_prepared_metadata_inventory_acceptance(
    isolated_catalog: &Path,
    roots: &[(&'static str, PathBuf); 2],
    report_path: &Path,
    backup_ms: u64,
) {
    let cached_gallery_started = Instant::now();
    let mut catalog =
        SqliteCatalog::open(isolated_catalog.to_path_buf()).expect("isolated catalog");
    let cached_gallery = catalog
        .load_snapshot(
            INITIAL_GALLERY_ITEMS,
            &GalleryQuery::default(),
            "r2c-m-cached",
            None,
            None,
            None,
        )
        .expect("load cached gallery");
    let manifest = catalog
        .load_gallery_layout_manifest_chunk(
            INITIAL_MANIFEST_ITEMS,
            &GalleryQuery::default(),
            "r2c-m-cached",
            None,
        )
        .expect("load initial cached gallery manifest");
    let cached_gallery_ms = elapsed_millis(cached_gallery_started.elapsed());
    assert!(
        !cached_gallery.assets.is_empty(),
        "retained gallery is empty"
    );
    assert!(
        !manifest.location_ids.is_empty(),
        "retained gallery manifest is empty"
    );
    assert!(
        cached_gallery_ms <= CACHED_GALLERY_LIMIT_MS,
        "cached gallery did not become available within one second"
    );

    let catalog_roots = catalog
        .load_incremental_catalog_roots()
        .expect("load retained catalog roots");
    assert_catalog_roots(&catalog_roots, roots);
    let scan_rows_before = scan_row_count(isolated_catalog).expect("pre-inventory scan rows");
    let queue_before = queue_totals(isolated_catalog).expect("pre-inventory queue totals");
    let catalog_bytes_before = sqlite_family_bytes(isolated_catalog);

    let root_reports =
        run_inventory_pass(&mut catalog, &catalog_roots, roots, true, Some(report_path));

    let safety_before = snapshot_roots_from_directory_entries(roots)
        .expect("pre-safety-pass directory-entry metadata snapshot");
    let placeholder_count = safety_before
        .iter()
        .filter(|entry| is_cloud_placeholder(entry.attributes))
        .count();
    run_inventory_pass(&mut catalog, &catalog_roots, roots, false, None);
    let safety_after = snapshot_roots_from_directory_entries(roots)
        .expect("post-safety-pass directory-entry metadata snapshot");
    assert_eq!(
        safety_before, safety_after,
        "source metadata changed during the repeated inventory safety pass"
    );

    let scan_rows_after = scan_row_count(isolated_catalog).expect("post-inventory scan rows");
    assert_eq!(
        scan_rows_before, scan_rows_after,
        "inventory started a full scan"
    );
    let queue_after = queue_totals(isolated_catalog).expect("post-inventory queue totals");
    let catalog_bytes_after = sqlite_family_bytes(isolated_catalog);

    let local = root_reports.get("local-primary").expect("local report");
    let cloud = root_reports.get("cloud-primary").expect("cloud report");
    append_report(
        report_path,
        &format!(
            "AME_R2C_M_REAL status=passed roots=2 cached_gallery_ms={cached_gallery_ms} cached_gallery_items={} initial_manifest_items={} manifest_total_items={} local_inventory_ms={} local_entries={} local_candidates={} local_unchanged={} cloud_inventory_ms={} cloud_entries={} cloud_candidates={} cloud_unchanged={} placeholder_entries={placeholder_count} metadata_boundary=local_metadata_inventory safety_snapshot_boundary=windows_directory_entries cold_inventory_source_snapshot=not_measured queue_rows_before={} queue_rows_after={} queue_growth={} catalog_bytes_before={catalog_bytes_before} catalog_bytes_after={catalog_bytes_after} catalog_growth={} backup_ms={backup_ms} repeated_inventory_source_metadata_unchanged=true repeated_inventory_placeholder_state_unchanged=true full_scan_rows_unchanged=true",
            cached_gallery.assets.len(),
            manifest.location_ids.len(),
            manifest.total_items,
            local.0,
            local.1,
            local.2,
            local.3,
            cloud.0,
            cloud.1,
            cloud.2,
            cloud.3,
            queue_before.0,
            queue_after.0,
            queue_after.0.saturating_sub(queue_before.0),
            catalog_bytes_after.saturating_sub(catalog_bytes_before),
        ),
    )
    .expect("write retained replacement report");
}

fn assert_catalog_roots(
    catalog_roots: &[IncrementalCatalogRoot],
    roots: &[(&'static str, PathBuf); 2],
) {
    for (_, expected_path) in roots {
        let matching_roots = catalog_roots
            .iter()
            .filter(|root| paths_same(&PathBuf::from(&root.root_path), expected_path))
            .count();
        assert_eq!(
            matching_roots, 1,
            "retained catalog must contain exactly one matching authorized logical root"
        );
    }
}

fn run_inventory_pass(
    catalog: &mut SqliteCatalog,
    catalog_roots: &[IncrementalCatalogRoot],
    roots: &[(&'static str, PathBuf); 2],
    enforce_latency: bool,
    report_path: Option<&Path>,
) -> BTreeMap<&'static str, (u64, u64, u64, u64)> {
    let mut reports = BTreeMap::new();
    for (logical_root, root_path) in roots {
        let root = catalog_roots
            .iter()
            .find(|root| paths_same(&PathBuf::from(&root.root_path), root_path))
            .expect("catalog root for authorized path");
        let started_unix_ms = current_unix_millis();
        let epoch = next_inventory_epoch(catalog.catalog_path(), &root.root_id)
            .expect("next metadata inventory epoch");
        let request = MetadataInventoryRunRequest {
            run_id: format!("r2c-m-{logical_root}-{epoch}"),
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            epoch,
            scope: MetadataInventoryScope::Root,
            started_unix_ms,
        };
        let mut source = TimedMetadataInventorySource::new(
            LocalMetadataInventory::new(&root.root_path, &MetadataInventoryScope::Root)
                .expect("retained-root inventory source"),
        );
        let inventory_started = Instant::now();
        let inventory = run_metadata_inventory_with_source_for_test(
            catalog,
            &mut source,
            &request,
            started_unix_ms,
            4_096,
            LibraryChangeQueuePolicy::default(),
            &AtomicBool::new(false),
        )
        .expect("metadata-only retained-root inventory");
        let inventory_ms = elapsed_millis(inventory_started.elapsed());
        let source_ms = elapsed_millis(source.next_page_elapsed);
        let repository_ms = inventory_ms.saturating_sub(source_ms);
        if let Some(report_path) = report_path {
            append_report(
                report_path,
                &format!(
                    "AME_R2C_M_ROOT logical_root={logical_root} inventory_ms={inventory_ms} source_page_ms={source_ms} repository_ms={repository_ms} entries={} candidates={} unchanged={} complete={} cancelled={} backpressured={}",
                    inventory.staged_entry_count,
                    inventory.candidate_count,
                    inventory.unchanged_count,
                    inventory.is_complete,
                    inventory.is_cancelled,
                    inventory.is_backpressured,
                ),
            )
            .expect("write retained-root timing report");
        }
        assert!(inventory.is_complete, "metadata inventory did not complete");
        assert!(!inventory.is_cancelled, "metadata inventory was cancelled");
        assert!(
            !inventory.is_backpressured,
            "metadata inventory was backpressured"
        );
        if enforce_latency {
            assert!(
                inventory_ms <= METADATA_ROOT_LIMIT_MS,
                "initial metadata inventory for {logical_root} exceeded 45 seconds: total={inventory_ms}ms source={source_ms}ms repository={repository_ms}ms"
            );
        }
        reports.insert(
            *logical_root,
            (
                inventory_ms,
                inventory.staged_entry_count,
                inventory.candidate_count,
                inventory.unchanged_count,
            ),
        );
    }
    reports
}

struct TimedMetadataInventorySource {
    inner: LocalMetadataInventory,
    next_page_elapsed: Duration,
}

impl TimedMetadataInventorySource {
    const fn new(inner: LocalMetadataInventory) -> Self {
        Self {
            inner,
            next_page_elapsed: Duration::ZERO,
        }
    }
}

impl MetadataInventorySource for TimedMetadataInventorySource {
    fn next_page(
        &mut self,
        max_entries: u32,
        cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        let started = Instant::now();
        let result = self.inner.next_page(max_entries, cancelled);
        self.next_page_elapsed += started.elapsed();
        result
    }
}

struct ControlledFixture {
    _storage: tempfile::TempDir,
    storage: StoragePaths,
    source_root: PathBuf,
    root_id: String,
}

impl ControlledFixture {
    fn new() -> Self {
        let storage = tempdir().expect("controlled replacement storage");
        let source_root = storage.path().join("source");
        fs::create_dir_all(&source_root).expect("controlled source root");
        let mut catalog =
            SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
        let request = ScanRequest {
            scan_id: "r2c-m-controlled-scan".to_owned(),
            root_path: source_root.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        let root_id = "r2c-m-controlled-root";
        let checkpoint = catalog
            .begin_scan(&request, root_id, &request.root_path)
            .expect("begin controlled scan");
        catalog
            .publish_scan(
                &request.scan_id,
                root_id,
                checkpoint.accepted_items,
                checkpoint.issue_count,
            )
            .expect("publish controlled root");
        let storage_paths = StoragePaths {
            catalog_path: catalog.catalog_path().to_path_buf(),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        drop(catalog);
        Self {
            _storage: storage,
            storage: storage_paths,
            source_root,
            root_id: root_id.to_owned(),
        }
    }

    fn open_catalog(&self) -> SqliteCatalog {
        SqliteCatalog::open(self.storage.catalog_path.clone()).expect("controlled catalog")
    }

    fn location(&self, relative_path: &str) -> Option<AssetLocationView> {
        location(&self.open_catalog(), &self.root_id, relative_path)
    }
}

struct AcceptanceConfiguration {
    source_catalog: PathBuf,
    storage_root: PathBuf,
    local_root: PathBuf,
    cloud_root: PathBuf,
}

fn acceptance_configuration() -> Result<AcceptanceConfiguration, String> {
    if required_environment("CEDARFLAKE_AME_R2C_M_CONSENT")? != CONSENT_TOKEN {
        return Err("the exact current R2c-M authorization token is required".to_owned());
    }
    if required_environment("CEDARFLAKE_AME_R2C_M_CLOUD_READ_ONLY_ACK")? != "true" {
        return Err("the cloud read-only acknowledgement is required".to_owned());
    }
    let source_catalog = absolute_file_environment("CEDARFLAKE_AME_R2C_M_SOURCE_CATALOG")?;
    let storage_root = absolute_directory_environment("CEDARFLAKE_AME_R2C_M_STORAGE_ROOT")?;
    let local_root = absolute_directory_environment("CEDARFLAKE_AME_R2C_M_LOCAL_ROOT")?;
    let cloud_root = absolute_directory_environment("CEDARFLAKE_AME_R2C_M_CLOUD_ROOT")?;
    if resolved_paths_overlap(&local_root, &cloud_root).map_err(storage_error_message)?
        || resolved_paths_overlap(&local_root, &storage_root).map_err(storage_error_message)?
        || resolved_paths_overlap(&cloud_root, &storage_root).map_err(storage_error_message)?
        || resolved_paths_overlap(&source_catalog, &storage_root).map_err(storage_error_message)?
        || resolved_path_is_within(&source_catalog, &local_root).map_err(storage_error_message)?
        || resolved_path_is_within(&source_catalog, &cloud_root).map_err(storage_error_message)?
    {
        return Err("acceptance roots, catalog, and isolated storage must not overlap".to_owned());
    }
    Ok(AcceptanceConfiguration {
        source_catalog,
        storage_root,
        local_root,
        cloud_root,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SourceSafetyEntry {
    logical_root: &'static str,
    relative_path: Vec<u16>,
    entry_kind: u8,
    length: u64,
    last_write_time: u64,
    attributes: u32,
}

fn snapshot_roots_from_directory_entries(
    roots: &[(&'static str, PathBuf); 2],
) -> Result<Vec<SourceSafetyEntry>, String> {
    let mut entries = Vec::new();
    for (logical_root, root_path) in roots {
        let mut pending = vec![root_path.clone()];
        let mut count = 0_usize;
        while let Some(directory) = pending.pop() {
            for child in fs::read_dir(directory)
                .map_err(|_| format!("{logical_root} could not be enumerated read-only"))?
            {
                let child =
                    child.map_err(|_| format!("{logical_root} returned an unreadable entry"))?;
                let path = child.path();
                // Pinned Rust returns cached WIN32_FIND_DATAW here, so no child handle is opened.
                let metadata = child
                    .metadata()
                    .map_err(|_| format!("{logical_root} directory metadata was unavailable"))?;
                let attributes = metadata.file_attributes();
                entries.push(SourceSafetyEntry {
                    logical_root,
                    relative_path: path
                        .strip_prefix(root_path)
                        .map_err(|_| "an enumerated path escaped its authorized root".to_owned())?
                        .as_os_str()
                        .encode_wide()
                        .collect(),
                    entry_kind: if metadata.is_dir() {
                        1
                    } else if metadata.is_file() {
                        2
                    } else {
                        3
                    },
                    length: metadata.len(),
                    last_write_time: metadata.last_write_time(),
                    attributes,
                });
                count = count.saturating_add(1);
                if count > MAX_SOURCE_ENTRIES_PER_ROOT {
                    return Err(format!("{logical_root} exceeded the bounded entry limit"));
                }
                if metadata.is_dir() && attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
                    pending.push(path);
                }
            }
        }
    }
    entries.sort();
    Ok(entries)
}

fn measure_visible_change(
    synchronization: &mut ProductionSynchronizationTestHarness,
    fixture: &ControlledFixture,
    maximum_poll_micros: &mut u64,
    mutate: impl FnOnce(),
    mut is_visible: impl FnMut(&SqliteCatalog) -> bool,
) -> u64 {
    let started = Instant::now();
    mutate();
    wait_for(Duration::from_secs(10), PRODUCTION_POLL_INTERVAL, || {
        let poll_started = Instant::now();
        let snapshot = synchronization.poll().expect("event synchronization poll");
        *maximum_poll_micros = (*maximum_poll_micros).max(elapsed_micros(poll_started.elapsed()));
        let mut catalog = fixture.open_catalog();
        if !is_visible(&catalog) {
            return false;
        }
        let gallery = catalog
            .load_snapshot(
                INITIAL_GALLERY_ITEMS,
                &GalleryQuery::default(),
                "r2c-m-live",
                None,
                None,
                None,
            )
            .expect("load refreshed controlled gallery");
        assert!(
            gallery.revision >= snapshot.catalog_revision,
            "visible catalog mutation was not available to the gallery query"
        );
        true
    });
    elapsed_micros(started.elapsed())
}

fn synchronization_is_current(snapshot: &LibrarySynchronizationSnapshot) -> bool {
    snapshot.is_running
        && !snapshot.roots.is_empty()
        && snapshot.roots.iter().all(|root| {
            root.source_health == LibraryChangeSourceHealth::Healthy
                && root.freshness == CatalogFreshnessState::Synchronized
        })
}

fn controlled_recovery_is_visible(catalog: &SqliteCatalog, root_id: &str) -> bool {
    location(catalog, root_id, "offline/新增.png").is_some()
        && location(catalog, root_id, "offline/moved.png").is_some()
        && location(catalog, root_id, "storm/item-0000.png").is_none()
        && location(catalog, root_id, "storm/item-0001.png").is_none()
        && location(catalog, root_id, "storm/item-0002.png")
            .is_some_and(|entry| entry.file_size > 100)
}

fn run_controlled_process_worker(fixture: &ControlledFixture, phase: &str) {
    let status = Command::new(std::env::current_exe().expect("current R2c-M test executable"))
        .args([
            "--exact",
            CONTROLLED_PROCESS_WORKER_TEST,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("CEDARFLAKE_AME_R2C_M_CONTROLLED_WORKER_PHASE", phase)
        .env(
            "CEDARFLAKE_AME_R2C_M_CONTROLLED_CATALOG",
            &fixture.storage.catalog_path,
        )
        .env(
            "CEDARFLAKE_AME_R2C_M_CONTROLLED_PREVIEWS",
            &fixture.storage.preview_root,
        )
        .env(
            "CEDARFLAKE_AME_R2C_M_CONTROLLED_SETTINGS",
            &fixture.storage.settings_path,
        )
        .env("CEDARFLAKE_AME_R2C_M_CONTROLLED_ROOT_ID", &fixture.root_id)
        .status()
        .expect("launch controlled synchronization process");
    assert!(
        status.success(),
        "controlled synchronization process failed"
    );
}

fn location(
    catalog: &SqliteCatalog,
    root_id: &str,
    relative_path: &str,
) -> Option<AssetLocationView> {
    catalog
        .load_incremental_location_by_relative_path(root_id, relative_path)
        .expect("load controlled location")
}

fn file_state_changed(previous: &AssetLocationView, current: &AssetLocationView) -> bool {
    previous.file_size != current.file_size
        || previous.modified_unix_ms != current.modified_unix_ms
        || previous.file_identity != current.file_identity
}

fn root_queue_evidence(catalog_path: &Path, root_id: &str) -> Result<(u64, u64), String> {
    let connection = Connection::open(catalog_path).map_err(database_message)?;
    let pair = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(coalesced_observation_count), 0)
             FROM library_change_queue
             WHERE root_id = ?1",
            [root_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(database_message)?;
    nonnegative_pair(pair)
}

fn queue_evidence_delta(baseline: (u64, u64), current: (u64, u64)) -> (u64, u64) {
    (
        current.0.saturating_sub(baseline.0),
        current.1.saturating_sub(baseline.1),
    )
}

fn next_inventory_epoch(catalog_path: &Path, root_id: &str) -> Result<u64, String> {
    let connection = Connection::open(catalog_path).map_err(database_message)?;
    let latest = connection
        .query_row(
            "SELECT COALESCE(MAX(epoch), 0)
             FROM library_metadata_inventory_runs
             WHERE root_id = ?1",
            [root_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_message)?;
    let latest =
        u64::try_from(latest).map_err(|_| "metadata inventory epoch was negative".to_owned())?;
    latest
        .checked_add(1)
        .ok_or_else(|| "metadata inventory epoch overflowed".to_owned())
}

fn queue_totals(catalog_path: &Path) -> Result<(u64, u64), String> {
    let connection = Connection::open(catalog_path).map_err(database_message)?;
    let pair = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(coalesced_observation_count), 0)
             FROM library_change_queue",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(database_message)?;
    nonnegative_pair(pair)
}

fn nonnegative_pair(pair: (i64, i64)) -> Result<(u64, u64), String> {
    Ok((
        u64::try_from(pair.0).map_err(|_| "catalog count was negative".to_owned())?,
        u64::try_from(pair.1).map_err(|_| "catalog count was negative".to_owned())?,
    ))
}

fn scan_row_count(catalog_path: &Path) -> Result<u64, String> {
    let connection = Connection::open(catalog_path).map_err(database_message)?;
    let count = connection
        .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(database_message)?;
    u64::try_from(count).map_err(|_| "scan row count was negative".to_owned())
}

fn backup_catalog(source_path: &Path, destination_path: &Path) -> Result<(), String> {
    let source = Connection::open_with_flags(
        source_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(database_message)?;
    let mut destination = Connection::open(destination_path).map_err(database_message)?;
    let backup = Backup::new(&source, &mut destination).map_err(database_message)?;
    backup
        .run_to_completion(128, Duration::from_millis(10), None)
        .map_err(database_message)
}

fn ensure_fresh_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        if fs::read_dir(path)
            .map_err(|_| "isolated storage is unreadable".to_owned())?
            .next()
            .is_some()
        {
            return Err("isolated R2c-M storage must be empty".to_owned());
        }
    } else {
        fs::create_dir_all(path).map_err(|_| "isolated storage could not be created".to_owned())?;
    }
    Ok(())
}

fn append_report(path: &Path, line: &str) -> Result<(), String> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| "the R2c-M report could not be opened".to_owned())?;
    writeln!(file, "{line}").map_err(|_| "the R2c-M report could not be written".to_owned())
}

fn report_path() -> Result<PathBuf, String> {
    let report = absolute_environment("CEDARFLAKE_AME_R2C_M_REPORT")?;
    let storage = absolute_environment("CEDARFLAKE_AME_R2C_M_STORAGE_ROOT")?;
    let parent = report
        .parent()
        .ok_or_else(|| "the R2c-M report must have a parent directory".to_owned())?;
    if !resolved_paths_same(parent, &storage).map_err(storage_error_message)? {
        return Err("the R2c-M report must remain directly inside isolated storage".to_owned());
    }
    Ok(report)
}

fn required_environment(name: &str) -> Result<String, String> {
    let value = std::env::var(name).map_err(|_| format!("{name} is required"))?;
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(value)
}

fn absolute_environment(name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(required_environment(name)?);
    if !path.is_absolute() {
        return Err(format!("{name} must be absolute"));
    }
    Ok(path)
}

fn absolute_directory_environment(name: &str) -> Result<PathBuf, String> {
    let path = absolute_environment(name)?;
    path.canonicalize()
        .map_err(|_| format!("{name} must name an available directory"))
        .and_then(|resolved| {
            if resolved.is_dir() {
                Ok(resolved)
            } else {
                Err(format!("{name} must name a directory"))
            }
        })
}

fn absolute_file_environment(name: &str) -> Result<PathBuf, String> {
    let path = absolute_environment(name)?;
    path.canonicalize()
        .map_err(|_| format!("{name} must name an available file"))
        .and_then(|resolved| {
            if resolved.is_file() {
                Ok(resolved)
            } else {
                Err(format!("{name} must name a file"))
            }
        })
}

fn paths_same(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn is_cloud_placeholder(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

fn write_png(path: &Path, width: u32, height: u32, seed: u8) {
    RgbImage::from_pixel(
        width,
        height,
        Rgb([seed, seed.wrapping_add(31), seed.wrapping_add(63)]),
    )
    .save(path)
    .expect("write controlled PNG");
}

fn wait_for(timeout: Duration, interval: Duration, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while !predicate() {
        assert!(
            Instant::now() < deadline,
            "bounded reliability wait timed out"
        );
        thread::sleep(interval);
    }
}

fn percentile_millis(samples: &mut [u64], percentile: usize) -> u64 {
    samples.sort_unstable();
    let index = samples
        .len()
        .saturating_mul(percentile)
        .div_ceil(100)
        .saturating_sub(1)
        .min(samples.len().saturating_sub(1));
    samples[index].div_ceil(1_000)
}

fn elapsed_micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn elapsed_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn current_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(i64::MAX)
}

fn sqlite_family_bytes(path: &Path) -> u64 {
    [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.to_string_lossy())),
        PathBuf::from(format!("{}-shm", path.to_string_lossy())),
    ]
    .iter()
    .filter_map(|candidate| fs::metadata(candidate).ok())
    .map(|metadata| metadata.len())
    .sum()
}

fn database_message(error: rusqlite::Error) -> String {
    format!("isolated catalog database operation failed: {error}")
}

fn storage_error_message(error: crate::domain::ScanError) -> String {
    format!("{}: {}", error.code, error.message)
}
