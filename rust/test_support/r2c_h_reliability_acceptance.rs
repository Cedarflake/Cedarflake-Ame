#![cfg(windows)]

use std::collections::{BTreeMap, BinaryHeap};
use std::fs::{self, Metadata};
use std::io::{Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use blake3::Hasher;
use image::{Rgb, RgbImage};
use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};
use tempfile::tempdir;

use crate::adapters::{
    SqliteCatalog, production_library_change_catch_up_source,
    production_library_change_source_factory,
};
use crate::domain::{
    CatalogFreshnessState, LibraryChangeQueuePolicy, LibraryChangeSourceHealth,
    LibraryRootAvailability, ScanRequest,
};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue};

use super::LibrarySynchronizationRuntime;
use crate::application::library_change_catch_up::{
    LibraryChangeCatchUpExecution, process_library_change_catch_up,
};

const CONSENT_TOKEN: &str = "CEDARFLAKE_AME_R2C_RELIABILITY_ACCEPTANCE_V1";
const EVENT_SAMPLE_COUNT: usize = 24;
const STORM_PATH_COUNT: usize = 96;
const IDLE_SAMPLE_COUNT: usize = 128;
const MAX_SOURCE_ENTRIES_PER_ROOT: usize = 250_000;
const HASH_SAMPLE_COUNT: usize = 32;
const HASH_SAMPLE_FILE_LIMIT: u64 = 64 * 1024 * 1024;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

#[test]
#[ignore = "requires the serial Windows R2c-H reliability acceptance wrapper"]
fn r2c_h_controlled_watcher_reliability_acceptance() {
    acceptance_configuration().expect("R2c-H acceptance authorization");
    let report_path = report_path().expect("R2c-H report path");
    let fixture = ControlledFixture::new();
    let mut catalog = fixture.catalog;
    let mut runtime =
        LibrarySynchronizationRuntime::new_erased(production_library_change_source_factory());
    runtime.queue_policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };

    let started_at = current_unix_millis();
    wait_for(Duration::from_secs(10), || {
        let snapshot = runtime
            .poll(&mut catalog, current_unix_millis(), |_| {
                LibraryRootAvailability::Available
            })
            .expect("initial synchronization poll");
        snapshot.roots.first().is_some_and(|root| {
            root.source_health == LibraryChangeSourceHealth::Healthy
                && root.freshness == CatalogFreshnessState::Synchronized
        })
    });
    let startup_elapsed_ms = current_unix_millis().saturating_sub(started_at);

    let mut idle_micros = Vec::with_capacity(IDLE_SAMPLE_COUNT);
    for _ in 0..IDLE_SAMPLE_COUNT {
        let started = Instant::now();
        runtime
            .poll(&mut catalog, current_unix_millis(), |_| {
                LibraryRootAvailability::Available
            })
            .expect("idle synchronization poll");
        idle_micros.push(elapsed_micros(started.elapsed()));
    }

    let mut event_visible_micros = Vec::with_capacity(EVENT_SAMPLE_COUNT);
    let mut maximum_poll_micros = 0_u64;
    for index in 0..EVENT_SAMPLE_COUNT {
        let relative_path = format!("latency/event-{index:03}.png");
        let absolute_path = fixture.source_root.join(&relative_path);
        fs::create_dir_all(absolute_path.parent().expect("latency parent"))
            .expect("create latency parent");
        let started = Instant::now();
        write_png(&absolute_path, index as u8);
        wait_for(Duration::from_secs(10), || {
            let poll_started = Instant::now();
            runtime
                .poll(&mut catalog, current_unix_millis(), |_| {
                    LibraryRootAvailability::Available
                })
                .expect("event synchronization poll");
            maximum_poll_micros = maximum_poll_micros.max(elapsed_micros(poll_started.elapsed()));
            catalog
                .load_incremental_location_by_relative_path(
                    &fixture.root_id,
                    &relative_path.replace('\\', "/"),
                )
                .expect("load latency location")
                .is_some()
        });
        event_visible_micros.push(elapsed_micros(started.elapsed()));
    }

    runtime.queue_policy.debounce_millis = LibraryChangeQueuePolicy::MAX_DEBOUNCE_MILLIS;
    for index in 0..STORM_PATH_COUNT {
        let path = fixture
            .source_root
            .join(format!("storm/item-{index:04}.png"));
        fs::create_dir_all(path.parent().expect("storm parent")).expect("create storm parent");
        write_png(&path, index as u8);
        write_png(&path, index.wrapping_add(1) as u8);
        write_png(&path, index.wrapping_add(2) as u8);
    }

    let mut last_queue_evidence = (0_u64, 0_u64);
    let mut stable_since = Instant::now();
    wait_for(Duration::from_secs(15), || {
        let poll_started = Instant::now();
        runtime
            .poll(&mut catalog, current_unix_millis(), |_| {
                LibraryRootAvailability::Available
            })
            .expect("storm synchronization poll");
        maximum_poll_micros = maximum_poll_micros.max(elapsed_micros(poll_started.elapsed()));
        let evidence = pending_queue_evidence(catalog.catalog_path(), &fixture.root_id)
            .expect("storm queue evidence");
        if evidence != last_queue_evidence {
            last_queue_evidence = evidence;
            stable_since = Instant::now();
        }
        evidence.0 > 0
            && evidence.1 > evidence.0
            && stable_since.elapsed() >= Duration::from_millis(400)
    });
    let (storm_queue_rows, storm_coalesced_observations) = last_queue_evidence;
    assert!(
        storm_queue_rows <= u64::try_from(STORM_PATH_COUNT).expect("storm count"),
        "the normalized storm queue exceeded the number of affected paths"
    );

    let stop_started = Instant::now();
    runtime.stop().expect("stop controlled observer");
    let stop_micros = elapsed_micros(stop_started.elapsed());
    assert!(
        stop_micros < 5_000_000,
        "observer shutdown exceeded five seconds"
    );

    let recovery_started = Instant::now();
    let mut recovered_runtime =
        LibrarySynchronizationRuntime::new_erased(production_library_change_source_factory());
    recovered_runtime.queue_policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    let recovery_now = current_unix_millis().saturating_add(
        i64::try_from(LibraryChangeQueuePolicy::MAX_DEBOUNCE_MILLIS)
            .expect("debounce duration")
            .saturating_add(1_000),
    );
    wait_for(Duration::from_secs(30), || {
        let poll_started = Instant::now();
        recovered_runtime
            .poll(&mut catalog, recovery_now, |_| {
                LibraryRootAvailability::Available
            })
            .expect("restart recovery poll");
        maximum_poll_micros = maximum_poll_micros.max(elapsed_micros(poll_started.elapsed()));
        (0..STORM_PATH_COUNT).all(|index| {
            catalog
                .load_incremental_location_by_relative_path(
                    &fixture.root_id,
                    &format!("storm/item-{index:04}.png"),
                )
                .expect("load recovered storm location")
                .is_some()
        })
    });
    let recovery_micros = elapsed_micros(recovery_started.elapsed());
    recovered_runtime
        .stop()
        .expect("stop recovered controlled observer");

    let catalog_bytes = sqlite_family_bytes(catalog.catalog_path());
    let idle_p50_ms = percentile_millis(&mut idle_micros, 50);
    let idle_p95_ms = percentile_millis(&mut idle_micros, 95);
    let event_p50_ms = percentile_millis(&mut event_visible_micros, 50);
    let event_p95_ms = percentile_millis(&mut event_visible_micros, 95);
    assert!(
        event_p95_ms < 5_000,
        "event visibility P95 exceeded five seconds"
    );
    assert!(
        catalog_bytes < 64 * 1024 * 1024,
        "controlled catalog exceeded 64 MiB"
    );
    append_report(
        &report_path,
        &format!(
            "AME_R2C_H_CONTROLLED status=passed startup_ms={startup_elapsed_ms} idle_samples={} idle_p50_ms={idle_p50_ms} idle_p95_ms={idle_p95_ms} event_samples={} event_p50_ms={event_p50_ms} event_p95_ms={event_p95_ms} storm_paths={STORM_PATH_COUNT} storm_queue_rows={storm_queue_rows} storm_coalesced_observations={storm_coalesced_observations} max_poll_ms={} restart_recovery_ms={} stop_ms={} catalog_bytes={catalog_bytes}",
            idle_micros.len(),
            event_visible_micros.len(),
            maximum_poll_micros.div_ceil(1_000),
            recovery_micros.div_ceil(1_000),
            stop_micros.div_ceil(1_000),
        ),
    )
    .expect("write controlled acceptance report");
}

#[test]
#[ignore = "requires current explicit authorization for both named read-only roots"]
fn r2c_h_user_authorized_real_library_reliability_acceptance() {
    let configuration = acceptance_configuration().expect("R2c-H acceptance authorization");
    let report_path = report_path().expect("R2c-H report path");
    run_real_library_acceptance(&configuration, &report_path);
}

#[test]
fn r2c_h_small_read_only_reliability_fixture() {
    let fixture = tempdir().expect("small reliability fixture");
    let local_root = fixture.path().join("local");
    let cloud_root = fixture.path().join("cloud");
    let storage_root = fixture.path().join("acceptance");
    fs::create_dir_all(&local_root).expect("small local root");
    fs::create_dir_all(&cloud_root).expect("small cloud root");
    fs::create_dir_all(&storage_root).expect("small acceptance storage");
    write_png(&local_root.join("local.png"), 17);
    write_png(&cloud_root.join("cloud.png"), 29);
    let source_catalog = fixture.path().join("retained.sqlite3");
    let mut catalog = SqliteCatalog::open(source_catalog.clone()).expect("small source catalog");
    for (scan_id, root_id, root_path) in [
        ("small-local-scan", "small-local-root", &local_root),
        ("small-cloud-scan", "small-cloud-root", &cloud_root),
    ] {
        let request = ScanRequest {
            scan_id: scan_id.to_owned(),
            root_path: root_path.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let checkpoint = catalog
            .begin_scan(&request, root_id, &request.root_path)
            .expect("begin small retained scan");
        catalog
            .publish_scan(
                scan_id,
                root_id,
                checkpoint.accepted_items,
                checkpoint.issue_count,
            )
            .expect("publish small retained scan");
    }
    drop(catalog);
    let configuration = AcceptanceConfiguration {
        source_catalog,
        storage_root: storage_root.clone(),
        local_root,
        cloud_root,
    };
    let report_path = storage_root.join("small-report.log");

    run_real_library_acceptance(&configuration, &report_path);

    let report = fs::read_to_string(report_path).expect("small reliability report");
    assert!(report.contains("AME_R2C_H_REAL status=passed"));
    assert!(report.contains("source_unchanged=true"));
}

fn run_real_library_acceptance(configuration: &AcceptanceConfiguration, report_path: &Path) {
    let isolated_root = configuration.storage_root.join("real-catalog");
    ensure_fresh_directory(&isolated_root).expect("fresh isolated R2c-H storage");
    let isolated_catalog = isolated_root.join("ame.sqlite3");
    let backup_started = Instant::now();
    backup_catalog(&configuration.source_catalog, &isolated_catalog)
        .expect("read-only retained catalog backup");
    let backup_micros = elapsed_micros(backup_started.elapsed());

    let roots = [
        ("local-primary", configuration.local_root.clone()),
        ("cloud-primary", configuration.cloud_root.clone()),
    ];
    let mut catalog = SqliteCatalog::open(isolated_catalog.clone()).expect("isolated catalog");
    let catalog_roots = catalog
        .load_incremental_catalog_roots()
        .expect("load retained catalog roots");
    assert_eq!(
        catalog_roots.len(),
        roots.len(),
        "retained root count changed"
    );
    for (_, expected_path) in &roots {
        assert!(
            catalog_roots
                .iter()
                .any(|root| paths_same(&PathBuf::from(&root.root_path), expected_path)),
            "retained catalog does not match an authorized logical root"
        );
    }

    let snapshot_started = Instant::now();
    let before = snapshot_roots(&roots).expect("pre-catch-up source snapshot");
    let snapshot_before_micros = elapsed_micros(snapshot_started.elapsed());
    let hash_candidates = choose_hash_candidates(&configuration.local_root, HASH_SAMPLE_COUNT)
        .expect("select local source-byte samples");
    assert!(
        !hash_candidates.is_empty(),
        "no safe local hash sample was available"
    );
    let hashes_before = hash_files(&hash_candidates).expect("pre-catch-up source hashes");

    let queue_before = queue_totals(&isolated_catalog).expect("pre-catch-up queue totals");
    let catalog_bytes_before = sqlite_family_bytes(&isolated_catalog);
    let catch_up_started = Instant::now();
    let catch_up = process_library_change_catch_up(
        &production_library_change_catch_up_source(),
        &mut catalog,
        &catalog_roots,
        LibraryChangeCatchUpExecution::at(
            current_unix_millis(),
            LibraryChangeQueuePolicy::default(),
        ),
        &AtomicBool::new(false),
    )
    .expect("read-only root catch-up against isolated catalog");
    let catch_up_micros = elapsed_micros(catch_up_started.elapsed());
    assert_eq!(
        catch_up.completed_roots.len(),
        roots.len(),
        "catch-up did not resolve every authorized logical root"
    );

    let queue_after = queue_totals(&isolated_catalog).expect("post-catch-up queue totals");
    let catalog_bytes_after = sqlite_family_bytes(&isolated_catalog);
    let mut idle_metric_micros = Vec::with_capacity(IDLE_SAMPLE_COUNT);
    for root in &catalog_roots {
        for _ in 0..(IDLE_SAMPLE_COUNT / roots.len()) {
            let started = Instant::now();
            catalog
                .load_library_change_root_queue_metrics(
                    &root.root_id,
                    root.root_generation,
                    current_unix_millis(),
                    LibraryChangeQueuePolicy::default(),
                )
                .expect("retained queue metric query");
            idle_metric_micros.push(elapsed_micros(started.elapsed()));
        }
    }

    let snapshot_after_started = Instant::now();
    let after = snapshot_roots(&roots).expect("post-catch-up source snapshot");
    let snapshot_after_micros = elapsed_micros(snapshot_after_started.elapsed());
    assert_eq!(
        before, after,
        "source entries or metadata changed during R2c-H acceptance"
    );
    let hashes_after = hash_files(&hash_candidates).expect("post-catch-up source hashes");
    assert_eq!(hashes_before, hashes_after, "sampled source bytes changed");
    let placeholder_count = before
        .iter()
        .filter(|entry| is_cloud_placeholder(entry.attributes))
        .count();
    let local_counts = source_entry_counts(&before, "local-primary");
    let cloud_counts = source_entry_counts(&before, "cloud-primary");
    let local_entries = local_counts.total();
    let cloud_entries = cloud_counts.total();
    let fallback_count = catch_up.fallback_count;
    let mut fallback_codes = BTreeMap::<&str, usize>::new();
    for code in catch_up
        .completed_roots
        .iter()
        .filter_map(|root| root.fallback_code.as_deref())
    {
        *fallback_codes.entry(code).or_default() += 1;
    }
    let fallback_codes = if fallback_codes.is_empty() {
        "none".to_owned()
    } else {
        fallback_codes
            .into_iter()
            .map(|(code, count)| format!("{code}:{count}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let direct_root_count = catch_up
        .completed_roots
        .len()
        .saturating_sub(usize::try_from(fallback_count).unwrap_or(usize::MAX));
    let idle_p50_ms = percentile_millis(&mut idle_metric_micros, 50);
    let idle_p95_ms = percentile_millis(&mut idle_metric_micros, 95);
    append_report(
        report_path,
        &format!(
            "AME_R2C_H_REAL status=passed roots={} local_entries={local_entries} local_files={} local_directories={} local_other={} cloud_entries={cloud_entries} cloud_files={} cloud_directories={} cloud_other={} placeholder_entries={placeholder_count} hash_samples={} snapshot_before_ms={} snapshot_after_ms={} catch_up_ms={} catch_up_direct_roots={direct_root_count} catch_up_fallback_roots={fallback_count} catch_up_fallback_codes={fallback_codes} observations={} checkpoints={} queue_rows_before={} queue_rows_after={} queue_growth={} catalog_bytes_before={catalog_bytes_before} catalog_bytes_after={catalog_bytes_after} catalog_growth={} metric_samples={} metric_p50_ms={idle_p50_ms} metric_p95_ms={idle_p95_ms} backup_ms={} source_unchanged=true",
            roots.len(),
            local_counts.files,
            local_counts.directories,
            local_counts.other,
            cloud_counts.files,
            cloud_counts.directories,
            cloud_counts.other,
            hash_candidates.len(),
            snapshot_before_micros.div_ceil(1_000),
            snapshot_after_micros.div_ceil(1_000),
            catch_up_micros.div_ceil(1_000),
            catch_up.observation_count,
            catch_up.checkpoint_count,
            queue_before.0,
            queue_after.0,
            queue_after.0.saturating_sub(queue_before.0),
            catalog_bytes_after.saturating_sub(catalog_bytes_before),
            idle_metric_micros.len(),
            backup_micros.div_ceil(1_000),
        ),
    )
    .expect("write real-library acceptance report");
}

struct ControlledFixture {
    _storage: tempfile::TempDir,
    catalog: SqliteCatalog,
    source_root: PathBuf,
    root_id: String,
}

impl ControlledFixture {
    fn new() -> Self {
        let storage = tempdir().expect("controlled reliability storage");
        let source_root = storage.path().join("source");
        fs::create_dir_all(&source_root).expect("controlled source root");
        let mut catalog =
            SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
        let request = ScanRequest {
            scan_id: "r2c-h-controlled-scan".to_owned(),
            root_path: source_root.to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        let checkpoint = catalog
            .begin_scan(&request, "r2c-h-controlled-root", &request.root_path)
            .expect("begin controlled scan");
        catalog
            .publish_scan(
                &request.scan_id,
                "r2c-h-controlled-root",
                checkpoint.accepted_items,
                checkpoint.issue_count,
            )
            .expect("publish controlled root");
        Self {
            _storage: storage,
            catalog,
            source_root,
            root_id: "r2c-h-controlled-root".to_owned(),
        }
    }
}

struct AcceptanceConfiguration {
    source_catalog: PathBuf,
    storage_root: PathBuf,
    local_root: PathBuf,
    cloud_root: PathBuf,
}

fn acceptance_configuration() -> Result<AcceptanceConfiguration, String> {
    if required_environment("CEDARFLAKE_AME_R2C_H_CONSENT")? != CONSENT_TOKEN {
        return Err("the exact current R2c-H authorization token is required".to_owned());
    }
    if required_environment("CEDARFLAKE_AME_R2C_H_CLOUD_READ_ONLY_ACK")? != "true" {
        return Err("the cloud read-only acknowledgement is required".to_owned());
    }
    let source_catalog = absolute_file_environment("CEDARFLAKE_AME_R2C_H_SOURCE_CATALOG")?;
    let storage_root = absolute_environment("CEDARFLAKE_AME_R2C_H_STORAGE_ROOT")?;
    let local_root = absolute_directory_environment("CEDARFLAKE_AME_R2C_H_LOCAL_ROOT")?;
    let cloud_root = absolute_directory_environment("CEDARFLAKE_AME_R2C_H_CLOUD_ROOT")?;
    if paths_overlap(&local_root, &cloud_root)
        || paths_overlap(&local_root, &storage_root)
        || paths_overlap(&cloud_root, &storage_root)
        || paths_overlap(&source_catalog, &storage_root)
        || paths_overlap(&source_catalog, &local_root)
        || paths_overlap(&source_catalog, &cloud_root)
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
struct SourceEntry {
    logical_root: &'static str,
    relative_path: Vec<u16>,
    entry_kind: u8,
    length: u64,
    modified_nanos: u128,
    attributes: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SourceEntryCounts {
    files: usize,
    directories: usize,
    other: usize,
}

impl SourceEntryCounts {
    const fn total(self) -> usize {
        self.files
            .saturating_add(self.directories)
            .saturating_add(self.other)
    }
}

fn source_entry_counts(entries: &[SourceEntry], logical_root: &str) -> SourceEntryCounts {
    let mut counts = SourceEntryCounts::default();
    for entry in entries
        .iter()
        .filter(|entry| entry.logical_root == logical_root)
    {
        match entry.entry_kind {
            1 => counts.directories = counts.directories.saturating_add(1),
            2 => counts.files = counts.files.saturating_add(1),
            _ => counts.other = counts.other.saturating_add(1),
        }
    }
    counts
}

fn snapshot_roots(roots: &[(&'static str, PathBuf); 2]) -> Result<Vec<SourceEntry>, String> {
    let mut entries = Vec::new();
    for (logical_root, root_path) in roots {
        snapshot_directory(logical_root, root_path, &mut entries)?;
    }
    entries.sort();
    Ok(entries)
}

fn snapshot_directory(
    logical_root: &'static str,
    root: &Path,
    entries: &mut Vec<SourceEntry>,
) -> Result<(), String> {
    let mut pending_directories = vec![root.to_path_buf()];
    let mut root_entry_count = 0_usize;
    while let Some(directory) = pending_directories.pop() {
        let children = fs::read_dir(directory)
            .map_err(|_| format!("{logical_root} could not be enumerated read-only"))?;
        for child in children {
            let child =
                child.map_err(|_| format!("{logical_root} returned an unreadable entry"))?;
            let path = child.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| format!("{logical_root} metadata could not be inspected"))?;
            let relative_path = path
                .strip_prefix(root)
                .map_err(|_| "an enumerated path escaped its authorized root".to_owned())?
                .as_os_str()
                .encode_wide()
                .collect();
            let attributes = metadata.file_attributes();
            let entry_kind = if metadata.is_dir() {
                1
            } else if metadata.is_file() {
                2
            } else {
                3
            };
            entries.push(SourceEntry {
                logical_root,
                relative_path,
                entry_kind,
                length: metadata.len(),
                modified_nanos: modified_nanos(&metadata),
                attributes,
            });
            root_entry_count = root_entry_count.saturating_add(1);
            if root_entry_count > MAX_SOURCE_ENTRIES_PER_ROOT {
                return Err(format!("{logical_root} exceeded the bounded entry limit"));
            }
            if metadata.is_dir() && attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
                pending_directories.push(path);
            }
        }
    }
    Ok(())
}

fn choose_hash_candidates(root: &Path, limit: usize) -> Result<Vec<PathBuf>, String> {
    let mut heap = BinaryHeap::<([u8; 32], PathBuf)>::new();
    let mut pending_directories = vec![root.to_path_buf()];
    while let Some(directory) = pending_directories.pop() {
        for child in
            fs::read_dir(directory).map_err(|_| "local-primary could not be sampled".to_owned())?
        {
            let child = child
                .map_err(|_| "local-primary returned an unreadable sample entry".to_owned())?;
            let path = child.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| "local-primary sample metadata could not be inspected".to_owned())?;
            let attributes = metadata.file_attributes();
            if metadata.is_dir() && attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
                pending_directories.push(path);
            } else if metadata.is_file()
                && metadata.len() <= HASH_SAMPLE_FILE_LIMIT
                && !is_cloud_placeholder(attributes)
                && attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0
            {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| "a hash candidate escaped local-primary".to_owned())?;
                let score = hash_windows_relative_path(relative);
                heap.push((score, path));
                if heap.len() > limit {
                    heap.pop();
                }
            }
        }
    }
    let mut selected = heap.into_iter().map(|(_, path)| path).collect::<Vec<_>>();
    selected.sort();
    Ok(selected)
}

fn hash_files(paths: &[PathBuf]) -> Result<Vec<[u8; 32]>, String> {
    paths
        .iter()
        .map(|path| {
            let mut file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                .open(path)
                .map_err(|_| "a source hash sample could not be opened".to_owned())?;
            let metadata = file
                .metadata()
                .map_err(|_| "a source hash sample became unavailable".to_owned())?;
            let attributes = metadata.file_attributes();
            if !metadata.is_file()
                || is_cloud_placeholder(attributes)
                || attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
            {
                return Err("a source hash sample became unsafe to read".to_owned());
            }
            let mut hasher = Hasher::new();
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = file
                    .read(&mut buffer)
                    .map_err(|_| "a source hash sample could not be read".to_owned())?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Ok(*hasher.finalize().as_bytes())
        })
        .collect()
}

fn hash_windows_relative_path(path: &Path) -> [u8; 32] {
    let mut hasher = Hasher::new();
    for code_unit in path.as_os_str().encode_wide() {
        hasher.update(&code_unit.to_le_bytes());
    }
    *hasher.finalize().as_bytes()
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

fn pending_queue_evidence(catalog_path: &Path, root_id: &str) -> Result<(u64, u64), String> {
    let connection = Connection::open(catalog_path).map_err(database_message)?;
    let (rows, observations) = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(coalesced_observation_count), 0)
             FROM library_change_queue
             WHERE root_id = ?1 AND status IN ('pending', 'leased', 'retry_wait')",
            [root_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(database_message)?;
    nonnegative_pair(rows, observations)
}

fn queue_totals(catalog_path: &Path) -> Result<(u64, u64), String> {
    let connection = Connection::open(catalog_path).map_err(database_message)?;
    let (rows, observations) = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(coalesced_observation_count), 0)
             FROM library_change_queue",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(database_message)?;
    nonnegative_pair(rows, observations)
}

fn nonnegative_pair(first: i64, second: i64) -> Result<(u64, u64), String> {
    Ok((
        u64::try_from(first).map_err(|_| "catalog count was negative".to_owned())?,
        u64::try_from(second).map_err(|_| "catalog count was negative".to_owned())?,
    ))
}

fn ensure_fresh_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        let mut entries =
            fs::read_dir(path).map_err(|_| "isolated storage is unreadable".to_owned())?;
        if entries.next().is_some() {
            return Err("isolated R2c-H storage must be empty".to_owned());
        }
    } else {
        fs::create_dir_all(path).map_err(|_| "isolated storage could not be created".to_owned())?;
    }
    Ok(())
}

fn append_report(path: &Path, line: &str) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| "the R2c-H report could not be opened".to_owned())?;
    writeln!(file, "{line}").map_err(|_| "the R2c-H report could not be written".to_owned())
}

fn report_path() -> Result<PathBuf, String> {
    let report = absolute_environment("CEDARFLAKE_AME_R2C_H_REPORT")?;
    let storage = absolute_environment("CEDARFLAKE_AME_R2C_H_STORAGE_ROOT")?;
    if report
        .parent()
        .is_none_or(|parent| !paths_same(parent, &storage))
    {
        return Err("the R2c-H report must remain directly inside isolated storage".to_owned());
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

fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = left.to_string_lossy().replace('/', "\\").to_lowercase();
    let right = right.to_string_lossy().replace('/', "\\").to_lowercase();
    let left = left.trim_end_matches('\\');
    let right = right.trim_end_matches('\\');
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('\\'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn modified_nanos(metadata: &Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos())
}

fn is_cloud_placeholder(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

fn write_png(path: &Path, seed: u8) {
    RgbImage::from_pixel(
        2,
        2,
        Rgb([seed, seed.wrapping_add(31), seed.wrapping_add(63)]),
    )
    .save(path)
    .expect("write controlled PNG");
}

fn wait_for(timeout: Duration, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while !predicate() {
        assert!(
            Instant::now() < deadline,
            "bounded reliability wait timed out"
        );
        thread::sleep(Duration::from_millis(5));
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
