use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use blake3::Hasher;
use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags, params};

use crate::adapters::{LocalPreviewStore, SqliteCatalog, revalidate_file_state};
use crate::domain::{AssetLocationView, ExpectedFileState, PreviewRequest, PreviewStatus};
use crate::ports::CatalogRepository;

use super::preview::materialize_preview_with_store;
use super::preview_reclamation::reclaim_preview_capacity;
use super::storage::{resolved_path_is_within, resolved_paths_overlap, resolved_paths_same};
use super::{PREVIEW_LIFECYCLE_TEST_LOCK, StoragePaths};

const CONSENT_TOKEN: &str = "CEDARFLAKE_AME_PREVIEW_PERFORMANCE_ACCEPTANCE_V1";
const MIN_CACHE_BUDGET_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CACHE_BUDGET_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ACCEPTANCE_ITEMS: usize = 512;
const MAX_HASH_SAMPLES: usize = 16;
const CANDIDATE_MULTIPLIER: usize = 4;
const PRESSURE_THRESHOLD_NUMERATOR: u64 = 17;
const PRESSURE_THRESHOLD_DENOMINATOR: u64 = 20;

struct AcceptanceConfiguration {
    source_catalog: PathBuf,
    source_root: PathBuf,
    storage_root: PathBuf,
    report_path: PathBuf,
    logical_root: String,
    max_items: usize,
    min_successful_items: usize,
    cache_budget_bytes: u64,
    max_source_file_bytes: u64,
}

#[derive(Clone)]
struct Candidate {
    location_id: String,
    expected: ExpectedFileState,
}

#[derive(Default)]
struct BucketMetrics {
    cold_micros: Vec<u64>,
    warm_micros: Vec<u64>,
    reuse_checks: u64,
    reuse_passes: u64,
}

struct MaterializationResult {
    location: AssetLocationView,
    elapsed_micros: u64,
    cache_bytes_before: u64,
    cache_bytes_after: u64,
}

#[test]
#[ignore = "requires explicit authorization and a bounded real-library workload"]
fn user_authorized_preview_performance_acceptance() {
    let configuration = AcceptanceConfiguration::from_environment()
        .unwrap_or_else(|message| panic!("preview acceptance configuration rejected: {message}"));
    ensure_fresh_storage(&configuration.storage_root)
        .unwrap_or_else(|message| panic!("preview acceptance storage rejected: {message}"));
    let mut report = AcceptanceReport::new(configuration.report_path.clone());
    report.line(format!(
        "AME_PREVIEW_ACCEPTANCE_BEGIN logical_root={} max_items={} min_successful_items={} cache_budget_bytes={} max_source_file_bytes={}",
        configuration.logical_root,
        configuration.max_items,
        configuration.min_successful_items,
        configuration.cache_budget_bytes,
        configuration.max_source_file_bytes,
    ));
    match run_acceptance(&configuration, &mut report) {
        Ok(()) => report.line("AME_PREVIEW_ACCEPTANCE status=passed"),
        Err(message) => {
            report.line(format!(
                "AME_PREVIEW_ACCEPTANCE status=failed reason={message:?}"
            ));
            panic!("preview performance acceptance failed: {message}");
        }
    }
}

impl AcceptanceConfiguration {
    fn from_environment() -> Result<Self, String> {
        let consent = required_environment("CEDARFLAKE_AME_PREVIEW_ACCEPTANCE_CONSENT")?;
        if consent != CONSENT_TOKEN {
            return Err("the exact current authorization token is required".to_owned());
        }
        let source_catalog = absolute_environment_path("CEDARFLAKE_AME_PREVIEW_SOURCE_CATALOG")?;
        let source_root = absolute_environment_path("CEDARFLAKE_AME_PREVIEW_SOURCE_ROOT")?;
        let storage_root = absolute_environment_path("CEDARFLAKE_AME_PREVIEW_STORAGE_ROOT")?;
        let report_path = absolute_environment_path("CEDARFLAKE_AME_PREVIEW_REPORT")?;
        let logical_root = required_environment("CEDARFLAKE_AME_PREVIEW_LOGICAL_ROOT")?;
        if logical_root != "local-primary" {
            return Err("only the local-primary logical root is admitted".to_owned());
        }
        require_path_type(&source_catalog, false, "source catalog")?;
        require_path_type(&source_root, true, "source root")?;
        if resolved_paths_overlap(&source_root, &storage_root).map_err(scan_error_message)? {
            return Err("acceptance storage overlaps the source root".to_owned());
        }
        if resolved_path_is_within(&source_catalog, &source_root).map_err(scan_error_message)? {
            return Err("the source catalog is inside the source root".to_owned());
        }
        if resolved_paths_overlap(&source_catalog, &storage_root).map_err(scan_error_message)? {
            return Err("acceptance storage overlaps the source catalog".to_owned());
        }
        if !resolved_path_is_within(&report_path, &storage_root).map_err(scan_error_message)? {
            return Err("the report path must remain inside acceptance storage".to_owned());
        }
        let max_items =
            bounded_environment_usize("CEDARFLAKE_AME_PREVIEW_MAX_ITEMS", 1, MAX_ACCEPTANCE_ITEMS)?;
        let min_successful_items =
            bounded_environment_usize("CEDARFLAKE_AME_PREVIEW_MIN_SUCCESSFUL_ITEMS", 1, max_items)?;
        let cache_budget_bytes = bounded_environment_u64(
            "CEDARFLAKE_AME_PREVIEW_CACHE_BUDGET_BYTES",
            MIN_CACHE_BUDGET_BYTES,
            MAX_CACHE_BUDGET_BYTES,
        )?;
        let max_source_file_bytes = bounded_environment_u64(
            "CEDARFLAKE_AME_PREVIEW_MAX_SOURCE_FILE_BYTES",
            1024 * 1024,
            256 * 1024 * 1024,
        )?;
        Ok(Self {
            source_catalog,
            source_root,
            storage_root,
            report_path,
            logical_root,
            max_items,
            min_successful_items,
            cache_budget_bytes,
            max_source_file_bytes,
        })
    }
}

fn run_acceptance(
    configuration: &AcceptanceConfiguration,
    report: &mut AcceptanceReport,
) -> Result<(), String> {
    let _test_lock = PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .map_err(|_| "the preview lifecycle test lock is poisoned".to_owned())?;
    let storage = acceptance_storage(configuration);
    let backup_started = Instant::now();
    backup_catalog(&configuration.source_catalog, &storage.catalog_path)?;
    report.line(format!(
        "AME_PREVIEW_CATALOG_SNAPSHOT elapsed_ms={}",
        elapsed_millis(backup_started.elapsed()),
    ));

    let root_id = resolve_catalog_root(
        &storage.catalog_path,
        &configuration.source_root,
        &configuration.logical_root,
    )?;
    let mut catalog =
        SqliteCatalog::open(storage.catalog_path.clone()).map_err(scan_error_message)?;
    let reset_locations = catalog
        .reset_all_previews_for_cleanup()
        .map_err(scan_error_message)?;
    drop(catalog);
    let preview_store = LocalPreviewStore::new(
        storage.preview_root.clone(),
        configuration.cache_budget_bytes,
    )
    .map_err(|issue| format!("{}: {}", issue.code, issue.message))?;
    if preview_store.used_bytes() != 0 {
        return Err("the isolated preview cache was not empty".to_owned());
    }

    let (active_locations, candidates) = load_candidates(
        &storage.catalog_path,
        &root_id,
        &configuration.source_root,
        configuration.max_items,
        configuration.max_source_file_bytes,
    )?;
    if candidates.len() < configuration.min_successful_items {
        return Err(format!(
            "only {} eligible locally-readable candidates were available",
            candidates.len(),
        ));
    }
    report.line(format!(
        "AME_PREVIEW_CANDIDATES catalog_active_locations={active_locations} selected={} reset_preview_locations={reset_locations}",
        candidates.len(),
    ));

    let hashes_before = source_hash_samples(&candidates)?;
    let mut bucket_metrics = BTreeMap::<u32, BucketMetrics>::new();
    let mut failure_codes = BTreeMap::<String, u64>::new();
    let mut benchmark_locations = Vec::<String>::new();
    for candidate in &candidates {
        if benchmark_locations.len() >= configuration.min_successful_items {
            break;
        }
        let mut all_buckets_ready = true;
        for (cold_edge, warm_edge, bucket) in [(128, 96, 128), (129, 255, 256), (257, 511, 512)] {
            let cold = materialize(candidate, cold_edge, &storage, &preview_store)?;
            if !matches!(cold.location.preview_status, PreviewStatus::Ready) {
                record_failure(&cold.location, &mut failure_codes);
                all_buckets_ready = false;
                break;
            }
            let cold_path = cold.location.preview_path.clone();
            let warm = materialize(candidate, warm_edge, &storage, &preview_store)?;
            if !matches!(warm.location.preview_status, PreviewStatus::Ready) {
                record_failure(&warm.location, &mut failure_codes);
                all_buckets_ready = false;
                break;
            }
            let metrics = bucket_metrics.entry(bucket).or_default();
            metrics.cold_micros.push(cold.elapsed_micros);
            metrics.warm_micros.push(warm.elapsed_micros);
            metrics.reuse_checks = metrics.reuse_checks.saturating_add(1);
            if warm.location.preview_path == cold_path
                && warm.cache_bytes_before == warm.cache_bytes_after
            {
                metrics.reuse_passes = metrics.reuse_passes.saturating_add(1);
            }
        }
        if all_buckets_ready {
            benchmark_locations.push(candidate.location_id.clone());
        }
    }
    if benchmark_locations.len() < configuration.min_successful_items {
        return Err(format!(
            "only {} candidates completed all cold and warm buckets",
            benchmark_locations.len(),
        ));
    }
    for (bucket, metrics) in &mut bucket_metrics {
        report.line(format!(
            "AME_PREVIEW_LATENCY bucket={bucket} cold_samples={} cold_p50_ms={} cold_p95_ms={} cold_max_ms={} warm_samples={} warm_p50_ms={} warm_p95_ms={} warm_max_ms={} reuse_checks={} reuse_passes={}",
            metrics.cold_micros.len(),
            percentile_millis(&mut metrics.cold_micros, 50),
            percentile_millis(&mut metrics.cold_micros, 95),
            maximum_millis(&metrics.cold_micros),
            metrics.warm_micros.len(),
            percentile_millis(&mut metrics.warm_micros, 50),
            percentile_millis(&mut metrics.warm_micros, 95),
            maximum_millis(&metrics.warm_micros),
            metrics.reuse_checks,
            metrics.reuse_passes,
        ));
        if metrics.reuse_checks != metrics.reuse_passes {
            return Err(format!(
                "bucket {bucket} did not reuse every compatible artifact"
            ));
        }
    }
    let benchmark_cache_bytes = preview_store.used_bytes();
    report.line(format!(
        "AME_PREVIEW_CACHE phase=benchmark used_bytes={benchmark_cache_bytes} budget_bytes={}",
        configuration.cache_budget_bytes,
    ));

    let pressure_threshold = configuration
        .cache_budget_bytes
        .saturating_mul(PRESSURE_THRESHOLD_NUMERATOR)
        / PRESSURE_THRESHOLD_DENOMINATOR;
    let mut generated_pressure_locations = Vec::<String>::new();
    let mut automatic_reclamation_observations = 0_u64;
    for candidate in &candidates {
        if preview_store.used_bytes() >= pressure_threshold {
            break;
        }
        let result = materialize(candidate, 1024, &storage, &preview_store)?;
        if matches!(result.location.preview_status, PreviewStatus::Ready) {
            generated_pressure_locations.push(candidate.location_id.clone());
            if result.cache_bytes_after < result.cache_bytes_before {
                automatic_reclamation_observations =
                    automatic_reclamation_observations.saturating_add(1);
            }
        } else {
            record_failure(&result.location, &mut failure_codes);
        }
    }
    let pressure_cache_bytes = preview_store.used_bytes();
    report.line(format!(
        "AME_PREVIEW_CACHE phase=pressure used_bytes={pressure_cache_bytes} budget_bytes={} pressure_locations={} automatic_reclamation_observations={automatic_reclamation_observations}",
        configuration.cache_budget_bytes,
        generated_pressure_locations.len(),
    ));
    if pressure_cache_bytes < pressure_threshold {
        return Err(format!(
            "natural preview generation reached only {pressure_cache_bytes} bytes and could not exercise the 85 percent boundary",
        ));
    }

    let artifacts_before_reclamation = indexed_preview_artifacts(&storage.catalog_path)?;
    let reclamation_started = Instant::now();
    let removed_bytes = reclaim_preview_capacity(
        &storage,
        &preview_store,
        &[],
        configuration.cache_budget_bytes / 10,
    )
    .map_err(scan_error_message)?;
    let reclamation_elapsed = reclamation_started.elapsed();
    let reclaimed_cache_bytes = preview_store.used_bytes();
    let low_watermark = configuration.cache_budget_bytes.saturating_mul(4) / 5;
    let artifacts_after_reclamation = indexed_preview_artifacts(&storage.catalog_path)?;
    let (evicted_location, evicted_bucket) = artifacts_before_reclamation
        .iter()
        .find(|(key, path)| !path.is_empty() && !artifacts_after_reclamation.contains_key(*key))
        .map(|((location_id, bucket), _)| (location_id.clone(), *bucket))
        .ok_or_else(|| "reclamation did not expose an evicted indexed artifact".to_owned())?;
    report.line(format!(
        "AME_PREVIEW_RECLAMATION events=1 duration_ms={} before_bytes={pressure_cache_bytes} after_bytes={reclaimed_cache_bytes} removed_bytes={removed_bytes} target_bytes={low_watermark}",
        elapsed_millis(reclamation_elapsed),
    ));
    if removed_bytes == 0 || reclaimed_cache_bytes > low_watermark {
        return Err("reclamation did not reach the configured low watermark".to_owned());
    }

    let regenerated_candidate = candidates
        .iter()
        .find(|candidate| candidate.location_id == evicted_location)
        .ok_or_else(|| "the evicted location was not in the bounded candidate set".to_owned())?;
    let regenerated = materialize(
        regenerated_candidate,
        evicted_bucket,
        &storage,
        &preview_store,
    )?;
    if !matches!(regenerated.location.preview_status, PreviewStatus::Ready) {
        return Err("an evicted preview could not be regenerated".to_owned());
    }
    let regenerated_path = regenerated.location.preview_path.clone();
    let warm_regenerated = materialize(
        regenerated_candidate,
        evicted_bucket,
        &storage,
        &preview_store,
    )?;
    let immediate_churn = u64::from(
        warm_regenerated.location.preview_path != regenerated_path
            || warm_regenerated.cache_bytes_before != warm_regenerated.cache_bytes_after,
    );
    report.line(format!(
        "AME_PREVIEW_REGENERATION samples=1 duration_ms={} warm_duration_ms={} immediate_boundary_churn={immediate_churn}",
        micros_to_millis(regenerated.elapsed_micros),
        micros_to_millis(warm_regenerated.elapsed_micros),
    ));
    if immediate_churn != 0 {
        return Err("the regenerated preview churned at the same cache boundary".to_owned());
    }
    if preview_store.used_bytes() > configuration.cache_budget_bytes {
        return Err("the preview cache exceeded its hard byte budget".to_owned());
    }

    verify_source_entries(&candidates)?;
    let hashes_after = source_hash_samples(&candidates)?;
    if hashes_after != hashes_before {
        return Err("sampled source bytes changed during preview acceptance".to_owned());
    }
    report.line(format!(
        "AME_PREVIEW_SOURCE entries_checked={} hash_samples={} bytes_unchanged=true entries_unchanged=true",
        candidates.len(),
        hashes_before.len(),
    ));
    report.line(format!(
        "AME_PREVIEW_FAILURES total={} codes={}",
        failure_codes.values().sum::<u64>(),
        compact_failure_codes(&failure_codes),
    ));
    Ok(())
}

fn ensure_fresh_storage(storage_root: &Path) -> Result<(), String> {
    if storage_root.exists() {
        if storage_root
            .read_dir()
            .map_err(|error| format!("could not inspect acceptance storage: {error}"))?
            .next()
            .is_some()
        {
            return Err("acceptance storage must be empty".to_owned());
        }
    } else {
        fs::create_dir_all(storage_root)
            .map_err(|error| format!("could not create acceptance storage: {error}"))?;
    }
    Ok(())
}

fn acceptance_storage(configuration: &AcceptanceConfiguration) -> StoragePaths {
    StoragePaths {
        catalog_path: configuration
            .storage_root
            .join("catalog")
            .join("ame.sqlite3"),
        preview_root: configuration
            .storage_root
            .join("cache")
            .join("previews")
            .join("ame-jpeg-thumbnail-v2-orientation"),
        preview_budget_bytes: configuration.cache_budget_bytes,
        settings_path: configuration
            .storage_root
            .join("settings")
            .join("storage.sqlite3"),
    }
}

fn backup_catalog(source_path: &Path, destination_path: &Path) -> Result<(), String> {
    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create the isolated catalog directory: {error}"))?;
    }
    let source = Connection::open_with_flags(
        source_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("could not open the source catalog read-only: {error}"))?;
    let mut destination = Connection::open(destination_path)
        .map_err(|error| format!("could not create the isolated catalog: {error}"))?;
    let backup = Backup::new(&source, &mut destination)
        .map_err(|error| format!("could not start the catalog snapshot: {error}"))?;
    backup
        .run_to_completion(256, Duration::from_millis(10), None)
        .map_err(|error| format!("could not complete the catalog snapshot: {error}"))
}

fn resolve_catalog_root(
    catalog_path: &Path,
    source_root: &Path,
    logical_root: &str,
) -> Result<String, String> {
    let connection = Connection::open_with_flags(catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("could not inspect the isolated catalog roots: {error}"))?;
    let mut statement = connection
        .prepare("SELECT id, path FROM library_roots WHERE active_scan_id IS NOT NULL")
        .map_err(|error| format!("could not prepare the catalog root query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("could not read the catalog roots: {error}"))?;
    for row in rows {
        let (root_id, path) = row.map_err(|error| format!("invalid catalog root: {error}"))?;
        if resolved_paths_same(Path::new(&path), source_root).map_err(scan_error_message)? {
            return Ok(root_id);
        }
    }
    Err(format!(
        "the isolated catalog has no active root matching {logical_root}",
    ))
}

fn load_candidates(
    catalog_path: &Path,
    root_id: &str,
    source_root: &Path,
    max_items: usize,
    max_source_file_bytes: u64,
) -> Result<(u64, Vec<Candidate>), String> {
    let connection = Connection::open_with_flags(catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("could not inspect the isolated catalog: {error}"))?;
    let active_locations = connection
        .query_row(
            "SELECT COUNT(*)
             FROM asset_locations AS locations
             JOIN library_roots AS roots
               ON roots.id = locations.root_id
              AND roots.active_scan_id = locations.scan_id
             WHERE locations.root_id = ?1",
            [root_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("could not count active catalog locations: {error}"))?;
    let active_locations = u64::try_from(active_locations)
        .map_err(|_| "the active catalog location count is invalid".to_owned())?;
    let candidate_limit = max_items.saturating_mul(CANDIDATE_MULTIPLIER);
    let sql_max_source_file_bytes = i64::try_from(max_source_file_bytes)
        .map_err(|_| "the source file byte limit exceeds SQLite integer range".to_owned())?;
    let sql_candidate_limit = i64::try_from(candidate_limit)
        .map_err(|_| "the candidate limit exceeds SQLite integer range".to_owned())?;
    let mut statement = connection
        .prepare(
            "SELECT locations.location_id, locations.absolute_path,
                    locations.file_size, locations.modified_unix_ms,
                    locations.file_identity_scheme, locations.file_identity_value
             FROM asset_locations AS locations
             JOIN library_roots AS roots
               ON roots.id = locations.root_id
              AND roots.active_scan_id = locations.scan_id
             WHERE locations.root_id = ?1
               AND locations.file_size BETWEEN 1024 AND ?2
               AND locations.width > 0 AND locations.height > 0
             ORDER BY locations.file_size DESC, locations.location_id
             LIMIT ?3",
        )
        .map_err(|error| format!("could not prepare the candidate query: {error}"))?;
    let rows = statement
        .query_map(
            params![root_id, sql_max_source_file_bytes, sql_candidate_limit],
            |row| {
                let scheme = row.get::<_, Option<String>>(4)?;
                let value = row.get::<_, Option<String>>(5)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    scheme,
                    value,
                ))
            },
        )
        .map_err(|error| format!("could not load preview candidates: {error}"))?;
    let mut candidates = Vec::with_capacity(max_items);
    for row in rows {
        let (location_id, absolute_path, file_size, modified_unix_ms, scheme, value) =
            row.map_err(|error| format!("invalid preview candidate: {error}"))?;
        let candidate = Candidate {
            location_id,
            expected: ExpectedFileState {
                absolute_path,
                file_size: u64::try_from(file_size)
                    .map_err(|_| "a preview candidate has an invalid file size".to_owned())?,
                modified_unix_ms,
                file_identity: scheme
                    .zip(value)
                    .map(|(scheme, value)| crate::domain::FileIdentityEvidence { scheme, value }),
            },
        };
        if candidates.len() >= max_items {
            break;
        }
        let source_path = Path::new(&candidate.expected.absolute_path);
        if !resolved_path_is_within(source_path, source_root).map_err(scan_error_message)? {
            continue;
        }
        if revalidate_file_state(&candidate.expected).is_ok() {
            candidates.push(candidate);
        }
    }
    Ok((active_locations, candidates))
}

fn materialize(
    candidate: &Candidate,
    preview_edge: u32,
    storage: &StoragePaths,
    preview_store: &LocalPreviewStore,
) -> Result<MaterializationResult, String> {
    let cache_bytes_before = preview_store.used_bytes();
    let started = Instant::now();
    let location = materialize_preview_with_store(
        PreviewRequest {
            location_id: candidate.location_id.clone(),
            preview_edge,
            retry_failed: true,
            protected_location_ids: vec![candidate.location_id.clone()],
        },
        storage.clone(),
        preview_store,
    )
    .map_err(scan_error_message)?;
    Ok(MaterializationResult {
        location,
        elapsed_micros: elapsed_micros(started.elapsed()),
        cache_bytes_before,
        cache_bytes_after: preview_store.used_bytes(),
    })
}

fn indexed_preview_artifacts(
    catalog_path: &Path,
) -> Result<BTreeMap<(String, u32), String>, String> {
    let connection = Connection::open_with_flags(catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("could not inspect reclaimed previews: {error}"))?;
    let mut statement = connection
        .prepare(
            "SELECT owners.location_id, artifacts.size_bucket, artifacts.artifact_path
             FROM preview_artifact_locations AS owners
             JOIN preview_artifacts AS artifacts
               ON artifacts.artifact_key = owners.artifact_key
             WHERE artifacts.lifecycle_state = 'ready'",
        )
        .map_err(|error| format!("could not prepare the reclaimed preview query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| format!("could not inspect reclaimed preview artifacts: {error}"))?;
    let mut artifacts = BTreeMap::new();
    for row in rows {
        let (location_id, bucket, path) =
            row.map_err(|error| format!("invalid reclaimed preview artifact: {error}"))?;
        let bucket = u32::try_from(bucket)
            .map_err(|_| "a reclaimed preview artifact has an invalid bucket".to_owned())?;
        artifacts.insert((location_id, bucket), path);
    }
    Ok(artifacts)
}

fn source_hash_samples(candidates: &[Candidate]) -> Result<Vec<String>, String> {
    candidates
        .iter()
        .take(MAX_HASH_SAMPLES)
        .map(|candidate| {
            let file = File::open(&candidate.expected.absolute_path)
                .map_err(|error| format!("could not open a source hash sample: {error}"))?;
            let mut hasher = Hasher::new();
            hasher
                .update_reader(file)
                .map_err(|error| format!("could not hash a source sample: {error}"))?;
            Ok(hasher.finalize().to_hex().to_string())
        })
        .collect()
}

fn verify_source_entries(candidates: &[Candidate]) -> Result<(), String> {
    for candidate in candidates {
        revalidate_file_state(&candidate.expected)
            .map_err(|issue| format!("{}: {}", issue.code, issue.message))?;
    }
    Ok(())
}

fn record_failure(location: &AssetLocationView, failures: &mut BTreeMap<String, u64>) {
    let code = location
        .preview_issue_code
        .clone()
        .unwrap_or_else(|| "unknown".to_owned());
    *failures.entry(code).or_default() += 1;
}

fn compact_failure_codes(failures: &BTreeMap<String, u64>) -> String {
    if failures.is_empty() {
        return "none".to_owned();
    }
    failures
        .iter()
        .map(|(code, count)| format!("{code}:{count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn percentile_millis(samples: &mut [u64], percentile: usize) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    samples.sort_unstable();
    let index = samples.len().saturating_mul(percentile).saturating_add(99) / 100;
    micros_to_millis(samples[index.saturating_sub(1).min(samples.len() - 1)])
}

fn maximum_millis(samples: &[u64]) -> u64 {
    samples.iter().copied().max().map_or(0, micros_to_millis)
}

fn elapsed_micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn elapsed_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn micros_to_millis(micros: u64) -> u64 {
    micros.saturating_add(999) / 1000
}

fn required_environment(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required"))
}

fn require_path_type(path: &Path, expects_directory: bool, label: &str) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("the {label} is unavailable ({:?})", error.kind()))?;
    if metadata.is_dir() != expects_directory || metadata.is_file() == expects_directory {
        return Err(format!("the {label} has the wrong filesystem type"));
    }
    Ok(())
}

fn absolute_environment_path(name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(required_environment(name)?);
    if !path.is_absolute() {
        return Err(format!("{name} must be absolute"));
    }
    Ok(path)
}

fn bounded_environment_usize(name: &str, minimum: usize, maximum: usize) -> Result<usize, String> {
    let value = required_environment(name)?
        .parse::<usize>()
        .map_err(|_| format!("{name} must be an integer"))?;
    if !(minimum..=maximum).contains(&value) {
        return Err(format!("{name} must be between {minimum} and {maximum}"));
    }
    Ok(value)
}

fn bounded_environment_u64(name: &str, minimum: u64, maximum: u64) -> Result<u64, String> {
    let value = required_environment(name)?
        .parse::<u64>()
        .map_err(|_| format!("{name} must be an integer"))?;
    if !(minimum..=maximum).contains(&value) {
        return Err(format!("{name} must be between {minimum} and {maximum}"));
    }
    Ok(value)
}

fn scan_error_message(error: crate::domain::ScanError) -> String {
    format!("{}: {}", error.code, error.message)
}

struct AcceptanceReport {
    path: PathBuf,
}

#[test]
fn preview_acceptance_percentiles_round_up_and_select_nearest_rank() {
    let mut samples = vec![1_001, 2_001, 3_001, 4_001, 5_001];

    assert_eq!(percentile_millis(&mut samples, 50), 4);
    assert_eq!(percentile_millis(&mut samples, 95), 6);
    assert_eq!(maximum_millis(&samples), 6);
}

#[test]
fn preview_acceptance_storage_must_be_new_or_empty() {
    let parent = tempfile::tempdir().expect("acceptance storage parent");
    let storage = parent.path().join("isolated");

    ensure_fresh_storage(&storage).expect("missing storage can be created");
    ensure_fresh_storage(&storage).expect("empty storage can be reused before the run");
    fs::write(storage.join("existing"), b"derived").expect("existing derived file");

    assert_eq!(
        ensure_fresh_storage(&storage),
        Err("acceptance storage must be empty".to_owned()),
    );
}

impl AcceptanceReport {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn line(&mut self, line: impl AsRef<str>) {
        println!("{}", line.as_ref());
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).expect("acceptance report directory");
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .expect("acceptance report file");
        writeln!(file, "{}", line.as_ref()).expect("acceptance report line");
    }
}
