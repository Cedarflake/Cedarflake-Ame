use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::adapters::{
    SqliteCatalog, current_preview_artifact_key, is_managed_preview_cleanup_entry,
};
use crate::domain::ScanError;
use crate::ports::CatalogRepository;

use super::{StoragePaths, acquire_preview_reclamation};

const RECOVERY_BATCH: usize = 64;
const BATCH_YIELD: Duration = Duration::from_millis(8);

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
    let mut catalog = SqliteCatalog::open(storage.catalog_path.clone())?;
    reconcile_directory(storage, &catalog)?;
    update_snapshot(|snapshot| snapshot.phase = PreviewRecoveryPhase::Index);
    reconcile_index(storage, &mut catalog)?;
    update_snapshot(|snapshot| snapshot.phase = PreviewRecoveryPhase::Completed);
    Ok(())
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
        let _exclusive_access = acquire_recovery_access()?;
        let mut exhausted = false;
        let mut removed_in_batch = false;
        for _ in 0..RECOVERY_BATCH {
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
            match fs::remove_file(&path) {
                Ok(()) => {
                    removed_in_batch = true;
                    update_snapshot(|snapshot| {
                        snapshot.removed_files = snapshot.removed_files.saturating_add(1);
                    });
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(_) => update_snapshot(|snapshot| {
                    snapshot.issue_count = snapshot.issue_count.saturating_add(1);
                }),
            }
        }
        if removed_in_batch {
            super::preview::invalidate_active_preview_store()?;
        }
        drop(_exclusive_access);
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
        let next_cursor = candidates
            .last()
            .map(|candidate| candidate.artifact_key.clone());
        let _exclusive_access = acquire_recovery_access()?;
        let mut removed_in_batch = false;
        for candidate in candidates {
            update_snapshot(|snapshot| {
                snapshot.inspected_artifacts = snapshot.inspected_artifacts.saturating_add(1);
            });
            let path = Path::new(&candidate.path);
            if path.parent() != Some(storage.preview_root.as_path())
                || !is_managed_preview_cleanup_entry(path)
            {
                if catalog.invalidate_preview_recovery_artifact(&candidate)? {
                    removed_in_batch = true;
                    update_snapshot(|snapshot| {
                        snapshot.missing_artifacts = snapshot.missing_artifacts.saturating_add(1);
                    });
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
                        removed_in_batch = true;
                        update_snapshot(|snapshot| {
                            snapshot.missing_artifacts =
                                snapshot.missing_artifacts.saturating_add(1);
                        });
                    }
                }
                Ok(_) | Err(_) => update_snapshot(|snapshot| {
                    snapshot.issue_count = snapshot.issue_count.saturating_add(1);
                }),
            }
        }
        if removed_in_batch {
            super::preview::invalidate_active_preview_store()?;
        }
        drop(_exclusive_access);
        after_artifact_key = next_cursor;
        thread::sleep(BATCH_YIELD);
    }
}

fn acquire_recovery_access() -> Result<std::sync::RwLockWriteGuard<'static, ()>, ScanError> {
    loop {
        match acquire_preview_reclamation() {
            Ok(access) => return Ok(access),
            Err(error) if error.code == "preview_cleanup_active" => thread::sleep(BATCH_YIELD),
            Err(error) => return Err(error),
        }
    }
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

    use rusqlite::Connection;
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
        let foreign = preview_root.join("keep.txt");
        fs::write(&unreferenced, b"unreferenced").expect("unreferenced preview");
        fs::write(&temporary, b"temporary").expect("temporary preview");
        fs::write(&legacy, b"legacy").expect("legacy preview");
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
        assert_eq!(fs::read(&legacy).expect("legacy after"), b"legacy");
        assert_eq!(fs::read(&foreign).expect("foreign after"), b"foreign");
        let snapshot = preview_recovery_snapshot();
        assert_eq!(snapshot.phase, PreviewRecoveryPhase::Completed);
        assert_eq!(snapshot.removed_files, 2);
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
        let location = AssetLocationView {
            asset_id: format!("recovery-{suffix}-asset"),
            location_id,
            root_id: root_id.clone(),
            absolute_path: format!("{root_path}\\one.png"),
            display_path: format!("{root_path}\\one.png"),
            relative_path: "one.png".to_owned(),
            preview_path: path.to_string_lossy().into_owned(),
            file_size: 100,
            created_unix_ms: Some(10),
            modified_unix_ms: 20,
            file_identity: None,
            width: 4_032,
            height: 3_024,
            preview_status: PreviewStatus::Ready,
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
            )
            .expect("publish fixture artifact");
    }
}
