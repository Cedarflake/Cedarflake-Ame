use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::adapters::{
    SqliteCatalog, current_preview_artifact_key, is_managed_preview_cleanup_entry,
};
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::CatalogRepository;

use super::{StoragePaths, acquire_preview_reclamation};

const RECOVERY_BATCH: usize = 64;
const BATCH_YIELD: Duration = Duration::from_millis(8);
const RECOVERY_BATCH_BUDGET: Duration = Duration::from_millis(4);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PreviewRecoverySnapshot {
    pub phase: PreviewRecoveryPhase,
    pub inspected_files: u64,
    pub inspected_artifacts: u64,
    pub removed_files: u64,
    pub missing_artifacts: u64,
    pub corrected_sizes: u64,
    pub issue_count: u64,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PreviewRecoveryPhase {
    #[default]
    Idle,
    Directory,
    Index,
    Completed,
    Failed,
}

static RECOVERY_STARTED: OnceLock<()> = OnceLock::new();
static RECOVERY_SNAPSHOT: OnceLock<Mutex<PreviewRecoverySnapshot>> = OnceLock::new();

pub(crate) fn start_preview_recovery(storage: StoragePaths) {
    RECOVERY_STARTED.get_or_init(|| {
        set_snapshot(PreviewRecoverySnapshot {
            phase: PreviewRecoveryPhase::Directory,
            ..PreviewRecoverySnapshot::default()
        });
        if let Err(error) = thread::Builder::new()
            .name("ame-preview-recovery".to_owned())
            .spawn(move || {
                if let Err(error) = run_preview_recovery(&storage) {
                    update_snapshot(|snapshot| {
                        snapshot.phase = PreviewRecoveryPhase::Failed;
                        snapshot.failure_code = Some(error.code);
                        snapshot.failure_message = Some(error.message);
                    });
                }
            })
        {
            update_snapshot(|snapshot| {
                snapshot.phase = PreviewRecoveryPhase::Failed;
                snapshot.failure_code = Some("preview_recovery_start_failed".to_owned());
                snapshot.failure_message = Some(error.to_string());
            });
        }
    });
}

pub fn preview_recovery_snapshot() -> PreviewRecoverySnapshot {
    recovery_snapshot().lock().map_or_else(
        |_| PreviewRecoverySnapshot::default(),
        |value| value.clone(),
    )
}

fn run_preview_recovery(storage: &StoragePaths) -> Result<(), ScanError> {
    let mut catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    reconcile_directory(storage, &catalog)?;
    update_snapshot(|snapshot| snapshot.phase = PreviewRecoveryPhase::Index);
    reconcile_index(storage, &mut catalog)?;
    update_snapshot(|snapshot| snapshot.phase = PreviewRecoveryPhase::Completed);
    Ok(())
}

#[cfg(test)]
pub(super) fn run_preview_recovery_for_test(storage: &StoragePaths) -> Result<(), ScanError> {
    set_snapshot(PreviewRecoverySnapshot {
        phase: PreviewRecoveryPhase::Directory,
        ..PreviewRecoverySnapshot::default()
    });
    run_preview_recovery(storage)
}

fn reconcile_directory(storage: &StoragePaths, catalog: &SqliteCatalog) -> Result<(), ScanError> {
    if !storage.preview_root.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(&storage.preview_root).map_err(|error| {
        ScanError::new(
            "preview_recovery_directory_unavailable",
            format!("Could not inspect preview storage during recovery: {error}"),
        )
    })?;
    loop {
        let batch_started = Instant::now();
        let mut exhausted = false;
        for processed in 0..RECOVERY_BATCH {
            if should_yield_recovery_batch(
                processed,
                batch_started.elapsed(),
                super::preview_cleanup::preview_generation_waiter_count(),
            ) {
                break;
            }
            let Some(entry) = entries.next() else {
                exhausted = true;
                break;
            };
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    update_snapshot(|snapshot| {
                        snapshot.issue_count = snapshot.issue_count.saturating_add(1);
                    });
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_file() || !is_managed_preview_cleanup_entry(&path) {
                continue;
            }
            update_snapshot(|snapshot| {
                snapshot.inspected_files = snapshot.inspected_files.saturating_add(1);
            });
            let is_temporary = path.extension().and_then(|value| value.to_str()) == Some("tmp");
            let artifact_key = current_preview_artifact_key(&path);
            let is_unreferenced = !is_temporary
                && !catalog
                    .is_preview_artifact_path_indexed(&path.to_string_lossy(), artifact_key)?;
            if !is_temporary && !is_unreferenced {
                continue;
            }
            let _exclusive_access = acquire_recovery_access()?;
            if !is_temporary
                && catalog.is_preview_artifact_path_indexed(
                    &path.to_string_lossy(),
                    current_preview_artifact_key(&path),
                )?
            {
                continue;
            }
            match fs::remove_file(&path) {
                Ok(()) => {
                    update_snapshot(|snapshot| {
                        snapshot.removed_files = snapshot.removed_files.saturating_add(1);
                    });
                    super::preview::invalidate_active_preview_store()?;
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(_) => update_snapshot(|snapshot| {
                    snapshot.issue_count = snapshot.issue_count.saturating_add(1);
                }),
            }
        }
        if exhausted {
            return Ok(());
        }
        thread::sleep(BATCH_YIELD);
    }
}

fn reconcile_index(storage: &StoragePaths, catalog: &mut SqliteCatalog) -> Result<(), ScanError> {
    let root_prefix = preview_root_prefix(&storage.preview_root);
    let mut after_artifact_key = None;
    loop {
        let candidates = catalog.load_preview_recovery_artifacts(
            &root_prefix,
            after_artifact_key.as_deref(),
            RECOVERY_BATCH as u32,
        )?;
        if candidates.is_empty() {
            return Ok(());
        }
        let batch_started = Instant::now();
        let mut next_cursor = None;
        for (processed, candidate) in candidates.into_iter().enumerate() {
            if should_yield_recovery_batch(
                processed,
                batch_started.elapsed(),
                super::preview_cleanup::preview_generation_waiter_count(),
            ) {
                break;
            }
            next_cursor = Some(candidate.artifact_key.clone());
            update_snapshot(|snapshot| {
                snapshot.inspected_artifacts = snapshot.inspected_artifacts.saturating_add(1);
            });
            let path = Path::new(&candidate.path);
            if path.parent() != Some(storage.preview_root.as_path())
                || !is_managed_preview_cleanup_entry(path)
            {
                if catalog.invalidate_preview_recovery_artifact(&candidate)? {
                    update_snapshot(|snapshot| {
                        snapshot.missing_artifacts = snapshot.missing_artifacts.saturating_add(1);
                    });
                    invalidate_recovered_preview_store()?;
                }
                continue;
            }
            match path.metadata() {
                Ok(metadata) if metadata.is_file() => {
                    if catalog.reconcile_preview_artifact_bytes(&candidate, metadata.len())? {
                        update_snapshot(|snapshot| {
                            snapshot.corrected_sizes = snapshot.corrected_sizes.saturating_add(1);
                        });
                    }
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    if catalog.invalidate_preview_recovery_artifact(&candidate)? {
                        update_snapshot(|snapshot| {
                            snapshot.missing_artifacts =
                                snapshot.missing_artifacts.saturating_add(1);
                        });
                        invalidate_recovered_preview_store()?;
                    }
                }
                Ok(_) | Err(_) => update_snapshot(|snapshot| {
                    snapshot.issue_count = snapshot.issue_count.saturating_add(1);
                }),
            }
        }
        after_artifact_key = next_cursor;
        thread::sleep(BATCH_YIELD);
    }
}

fn invalidate_recovered_preview_store() -> Result<(), ScanError> {
    let _exclusive_access = acquire_recovery_access()?;
    super::preview::invalidate_active_preview_store()
}

fn acquire_recovery_access() -> Result<super::preview_cleanup::PreviewReclamationGuard, ScanError> {
    loop {
        match acquire_preview_reclamation() {
            Ok(access) => return Ok(access),
            Err(error) if error.code == "preview_cleanup_active" => thread::sleep(BATCH_YIELD),
            Err(error) => return Err(error),
        }
    }
}

fn should_yield_recovery_batch(
    processed: usize,
    elapsed: Duration,
    preview_generation_waiters: usize,
) -> bool {
    processed > 0 && (elapsed >= RECOVERY_BATCH_BUDGET || preview_generation_waiters > 0)
}

fn preview_root_prefix(path: &Path) -> String {
    let mut prefix = path
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_owned();
    prefix.push(std::path::MAIN_SEPARATOR);
    prefix
}

fn recovery_snapshot() -> &'static Mutex<PreviewRecoverySnapshot> {
    RECOVERY_SNAPSHOT.get_or_init(|| Mutex::new(PreviewRecoverySnapshot::default()))
}

fn set_snapshot(snapshot: PreviewRecoverySnapshot) {
    if let Ok(mut current) = recovery_snapshot().lock() {
        *current = snapshot;
    }
}

fn update_snapshot(update: impl FnOnce(&mut PreviewRecoverySnapshot)) {
    if let Ok(mut snapshot) = recovery_snapshot().lock() {
        update(&mut snapshot);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc;

    use rusqlite::{Connection, TransactionBehavior};
    use tempfile::tempdir;

    use crate::adapters::{
        PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION, PREVIEW_CACHE_VERSION,
        PREVIEW_ORIENTATION_CONTRACT,
    };
    use crate::domain::{AssetLocationView, PreviewArtifact, PreviewStatus, ScanRequest};

    use super::*;

    #[test]
    fn recovery_removes_interrupted_and_unreferenced_files_only() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let directory = tempdir().expect("temporary directory");
        let preview_root = directory.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let hash = "a".repeat(64);
        let unreferenced = preview_root.join(format!("{PREVIEW_CACHE_VERSION}-{hash}.jpg"));
        let temporary = preview_root.join(format!("{PREVIEW_CACHE_VERSION}-{hash}.123-4.tmp"));
        let legacy = preview_root.join(format!("{}.jpg", "d".repeat(64)));
        let legacy_temporary = preview_root.join(format!("{}.456-7.tmp", "e".repeat(64)));
        let foreign = preview_root.join("keep.txt");
        fs::write(&unreferenced, b"unreferenced").expect("unreferenced preview");
        fs::write(&temporary, b"temporary").expect("temporary preview");
        fs::write(&legacy, b"legacy").expect("legacy preview");
        fs::write(&legacy_temporary, b"legacy temporary").expect("legacy temporary preview");
        fs::write(&foreign, b"foreign").expect("foreign file");
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        set_snapshot(PreviewRecoverySnapshot {
            phase: PreviewRecoveryPhase::Directory,
            ..PreviewRecoverySnapshot::default()
        });

        run_preview_recovery(&storage).expect("preview recovery");

        assert!(!unreferenced.exists());
        assert!(!temporary.exists());
        assert!(!legacy_temporary.exists());
        assert!(!legacy.exists());
        assert_eq!(fs::read(&foreign).expect("foreign after"), b"foreign");
        let snapshot = preview_recovery_snapshot();
        assert_eq!(snapshot.phase, PreviewRecoveryPhase::Completed);
        assert_eq!(snapshot.removed_files, 4);
        assert_eq!(snapshot.issue_count, 0);
    }

    #[test]
    fn recovery_resets_missing_artifacts_and_corrects_accounted_bytes() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let directory = tempdir().expect("temporary directory");
        let preview_root = directory.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let valid_path = managed_artifact_path(&preview_root, 'b');
        let missing_path = managed_artifact_path(&preview_root, 'c');
        fs::write(&valid_path, vec![1_u8; 17]).expect("valid preview");
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        publish_artifact(&mut catalog, "valid", &valid_path, 5);
        publish_artifact(&mut catalog, "missing", &missing_path, 7);
        drop(catalog);
        set_snapshot(PreviewRecoverySnapshot {
            phase: PreviewRecoveryPhase::Directory,
            ..PreviewRecoverySnapshot::default()
        });

        run_preview_recovery(&storage).expect("preview recovery");

        let catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        let missing = catalog
            .load_active_location("recovery-missing-location")
            .expect("missing location query")
            .expect("missing location");
        assert!(matches!(missing.preview_status, PreviewStatus::Pending));
        assert!(missing.preview_path.is_empty());
        assert_eq!((missing.width, missing.height), (4_032, 3_024));
        let connection = Connection::open(storage.catalog_path).expect("catalog connection");
        let valid_bytes: i64 = connection
            .query_row(
                "SELECT byte_size FROM preview_artifacts WHERE artifact_key = ?1",
                ["recovery-valid-artifact"],
                |row| row.get(0),
            )
            .expect("valid artifact size");
        assert_eq!(valid_bytes, 17);
        let snapshot = preview_recovery_snapshot();
        assert_eq!(snapshot.phase, PreviewRecoveryPhase::Completed);
        assert_eq!(snapshot.missing_artifacts, 1);
        assert_eq!(snapshot.corrected_sizes, 1);
    }

    #[test]
    fn recovery_batches_yield_for_time_budget_and_preview_waiters() {
        assert!(!should_yield_recovery_batch(0, RECOVERY_BATCH_BUDGET, 1,));
        assert!(!should_yield_recovery_batch(
            1,
            RECOVERY_BATCH_BUDGET - Duration::from_millis(1),
            0,
        ));
        assert!(should_yield_recovery_batch(1, RECOVERY_BATCH_BUDGET, 0,));
        assert!(should_yield_recovery_batch(1, Duration::ZERO, 1,));
    }

    #[test]
    fn blocked_recovery_database_write_does_not_hold_preview_reclamation_access() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let directory = tempdir().expect("temporary directory");
        let preview_root = directory.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let missing_path = managed_artifact_path(&preview_root, 'f');
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog").join("ame.sqlite3"),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        publish_artifact(&mut catalog, "blocked-write", &missing_path, 7);
        drop(catalog);
        set_snapshot(PreviewRecoverySnapshot {
            phase: PreviewRecoveryPhase::Directory,
            ..PreviewRecoverySnapshot::default()
        });

        let mut blocker = Connection::open(storage.catalog_path.clone()).expect("blocking catalog");
        let blocker_transaction = blocker
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("blocking writer transaction");
        blocker_transaction
            .execute("UPDATE catalog_state SET revision = revision", [])
            .expect("hold catalog writer");
        let recovery_storage = storage.clone();
        let recovery = thread::spawn(move || run_preview_recovery(&recovery_storage));
        let deadline = Instant::now() + Duration::from_secs(1);
        while preview_recovery_snapshot().inspected_artifacts == 0 {
            assert!(
                Instant::now() < deadline,
                "recovery did not reach index reconciliation"
            );
            thread::yield_now();
        }

        let (generation_tx, generation_rx) = mpsc::channel();
        let generation = thread::spawn(move || {
            let _access = super::super::acquire_preview_generation()
                .expect("foreground preview access during blocked recovery write");
            generation_tx.send(()).expect("foreground access event");
        });
        generation_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("database admission must not be nested inside preview reclamation access");

        blocker_transaction
            .commit()
            .expect("release catalog writer");
        generation.join().expect("foreground generation thread");
        recovery
            .join()
            .expect("recovery thread")
            .expect("recovery after writer release");
    }

    fn managed_artifact_path(root: &Path, hash_character: char) -> PathBuf {
        root.join(format!(
            "{PREVIEW_CACHE_VERSION}-{}.jpg",
            hash_character.to_string().repeat(64),
        ))
    }

    fn publish_artifact(
        catalog: &mut SqliteCatalog,
        suffix: &str,
        path: &Path,
        recorded_bytes: u64,
    ) {
        let scan_id = format!("recovery-{suffix}-scan");
        let root_id = format!("recovery-{suffix}-root");
        let location_id = format!("recovery-{suffix}-location");
        let root_path = format!("C:\\RecoverySource\\{suffix}");
        let request = ScanRequest {
            scan_id: scan_id.clone(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        catalog
            .begin_scan(&request, &root_id, &root_path)
            .expect("begin fixture scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&scan_id)
            .expect("prove fixture first-import handoff");
        let location = AssetLocationView {
            asset_id: format!("recovery-{suffix}-asset"),
            location_id: location_id.clone(),
            root_id: root_id.clone(),
            scan_id: scan_id.clone(),
            absolute_path: format!("{root_path}\\one.png"),
            display_path: format!("{root_path}\\one.png"),
            relative_path: "one.png".to_owned(),
            preview_path: String::new(),
            file_size: 100,
            created_unix_ms: Some(10),
            modified_unix_ms: 20,
            file_identity: None,
            source_revision: Some(crate::domain::SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0000000000000001".to_owned(),
            }),
            source_generation: 0,
            width: 4_032,
            height: 3_024,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: "fixture".to_owned(),
            metadata_engine_version: "1".to_owned(),
            capture_time: None,
        };
        catalog
            .stage_location(&scan_id, &root_id, &location)
            .expect("stage fixture location");
        catalog
            .publish_scan(&scan_id, &root_id, 1, 0)
            .expect("publish fixture scan");
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active fixture location query")
            .expect("active fixture location");
        location.preview_path = path.to_string_lossy().into_owned();
        location.preview_status = PreviewStatus::Ready;
        catalog
            .update_active_preview(
                &location,
                Some(&PreviewArtifact {
                    artifact_key: format!("recovery-{suffix}-artifact"),
                    algorithm_id: PREVIEW_ALGORITHM_ID.to_owned(),
                    algorithm_version: PREVIEW_ALGORITHM_VERSION,
                    orientation_contract: PREVIEW_ORIENTATION_CONTRACT.to_owned(),
                    size_bucket: 256,
                    path: location.preview_path.clone(),
                    byte_size: recorded_bytes,
                    encoded_width: 256,
                    encoded_height: 192,
                    width: location.width,
                    height: location.height,
                }),
                None,
            )
            .expect("publish fixture artifact");
    }
}
