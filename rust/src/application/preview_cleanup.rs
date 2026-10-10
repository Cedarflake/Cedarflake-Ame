use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

#[cfg(test)]
use crate::adapters::SqliteCatalog;
use crate::adapters::{
    PreviewCacheNamespace, SqliteStorageSettings, is_ame_preview_cache_entry, user_visible_path,
};
use crate::domain::{LibraryChangeLane, PreviewCleanupEvent, ScanError, ScanIssue};
use crate::ports::{CatalogRepository, StorageSettingsRepository};

use super::{StoragePaths, storage_paths};

const PROGRESS_INTERVAL: u64 = 32;

#[cfg(all(test, windows))]
#[path = "preview_cleanup/source_admission_tests.rs"]
mod source_admission_tests;

static PREVIEW_ACCESS: OnceLock<PreviewAccess> = OnceLock::new();
static ACTIVE_CLEANUP: OnceLock<Mutex<Option<ActiveCleanup>>> = OnceLock::new();

#[derive(Default)]
struct PreviewAccessState {
    active_generations: usize,
    generation_waiters: usize,
    next_generation_ticket: u128,
    recovery_active: bool,
    recovery_waiters: usize,
    foreground_turn_through: Option<u128>,
    foreground_turn_remaining: usize,
}

struct PreviewAccess {
    state: Mutex<PreviewAccessState>,
    changed: Condvar,
}

pub(crate) struct PreviewGenerationGuard {
    access: &'static PreviewAccess,
}

pub(crate) struct PreviewReclamationGuard {
    access: &'static PreviewAccess,
}

struct ActiveCleanup {
    operation_id: String,
    cancellation: Arc<AtomicBool>,
}

enum CleanupScope {
    Active(StoragePaths),
    Retired {
        preview_root: PathBuf,
        catalog_path: PathBuf,
        settings_path: PathBuf,
        stored_preview_root: String,
    },
}

impl CleanupScope {
    fn preview_root(&self) -> &Path {
        match self {
            Self::Active(storage) => &storage.preview_root,
            Self::Retired { preview_root, .. } => preview_root,
        }
    }

    fn catalog_path(&self) -> &Path {
        match self {
            Self::Active(storage) => &storage.catalog_path,
            Self::Retired { catalog_path, .. } => catalog_path,
        }
    }

    fn prepare(&self) -> Result<(), ScanError> {
        if let Self::Active(storage) = self {
            let mut catalog = super::catalog_session::open_catalog(
                &storage.catalog_path,
                LibraryChangeLane::Recovery,
            )?;
            catalog.reset_all_previews_for_cleanup()?;
            super::preview::invalidate_active_preview_store()?;
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), ScanError> {
        let Self::Retired {
            settings_path,
            stored_preview_root,
            ..
        } = self
        else {
            return Ok(());
        };
        let mut settings = SqliteStorageSettings::open(settings_path.clone())?;
        if !settings.forget_retired_preview_root(stored_preview_root)? {
            return Err(ScanError::new(
                "retired_preview_root_ownership_changed",
                "The retired preview root is no longer owned by this cleanup operation",
            ));
        }
        Ok(())
    }
}

pub(crate) fn acquire_preview_generation() -> Result<PreviewGenerationGuard, ScanError> {
    let cleanups = active_cleanup()
        .lock()
        .map_err(|_| preview_cleanup_registry_error())?;
    if cleanups.is_some() {
        return Err(ScanError::new(
            "preview_cleanup_active",
            "Preview generation is paused while preview cleanup is active",
        ));
    }
    drop(cleanups);
    preview_access().acquire_generation()
}

pub(crate) fn preview_generation_waiter_count() -> usize {
    preview_access().generation_waiter_count()
}

pub(crate) fn acquire_preview_reclamation() -> Result<PreviewReclamationGuard, ScanError> {
    let cleanups = active_cleanup()
        .lock()
        .map_err(|_| preview_cleanup_registry_error())?;
    if cleanups.is_some() {
        return Err(ScanError::new(
            "preview_cleanup_active",
            "Preview reclamation is paused while preview cleanup is active",
        ));
    }
    drop(cleanups);
    preview_access().acquire_reclamation()
}

pub fn clear_previews(
    operation_id: String,
    publish: impl FnMut(PreviewCleanupEvent) -> bool,
) -> Result<(), ScanError> {
    validate_operation_id(&operation_id)?;
    let storage = storage_paths()?;
    clear_previews_with_storage(operation_id, publish, storage)
}

pub fn clear_retired_previews(
    preview_root: String,
    operation_id: String,
    publish: impl FnMut(PreviewCleanupEvent) -> bool,
) -> Result<(), ScanError> {
    validate_operation_id(&operation_id)?;
    let requested_root = PathBuf::from(&preview_root);
    if !requested_root.is_absolute() {
        return Err(ScanError::new(
            "retired_preview_root_invalid",
            "The retired preview root must be an absolute path",
        ));
    }
    let storage = storage_paths()?;
    if super::storage::resolved_paths_same(&requested_root, &storage.preview_root)? {
        return Err(ScanError::new(
            "retired_preview_root_active",
            "The active preview root cannot be cleaned as a retired root",
        ));
    }
    let mut settings = SqliteStorageSettings::open(storage.settings_path.clone())?;
    let mut stored_preview_root = None;
    for stored in settings.load_retired_preview_roots()? {
        if super::storage::resolved_paths_same(Path::new(&stored), &requested_root)? {
            stored_preview_root = Some(stored);
            break;
        }
    }
    let stored_preview_root = stored_preview_root.ok_or_else(|| {
        ScanError::new(
            "retired_preview_root_not_owned",
            "The requested directory is not an Ame-owned retired preview root",
        )
    })?;
    clear_preview_scope(
        operation_id,
        publish,
        CleanupScope::Retired {
            preview_root: PathBuf::from(&stored_preview_root),
            catalog_path: storage.catalog_path,
            settings_path: storage.settings_path,
            stored_preview_root,
        },
    )
}

pub fn cancel_preview_cleanup(operation_id: &str) -> bool {
    let Ok(cleanup) = active_cleanup().lock() else {
        return false;
    };
    let Some(cleanup) = cleanup.as_ref() else {
        return false;
    };
    if cleanup.operation_id != operation_id {
        return false;
    }
    cleanup.cancellation.store(true, Ordering::Release);
    true
}

fn clear_previews_with_storage(
    operation_id: String,
    publish: impl FnMut(PreviewCleanupEvent) -> bool,
    storage: StoragePaths,
) -> Result<(), ScanError> {
    clear_preview_scope(operation_id, publish, CleanupScope::Active(storage))
}

fn clear_preview_scope(
    operation_id: String,
    mut publish: impl FnMut(PreviewCleanupEvent) -> bool,
    scope: CleanupScope,
) -> Result<(), ScanError> {
    validate_operation_id(&operation_id)?;
    let cancellation = register_cleanup(&operation_id)?;
    let _registration = CleanupRegistration {
        operation_id: operation_id.clone(),
    };
    let _exclusive_access = preview_access().acquire_reclamation()?;
    let namespace = PreviewCacheNamespace::open(scope.preview_root())?;
    let _source_exclusion =
        super::storage::source_cleanup_admission::reserve_preview_cleanup(scope.preview_root())?;
    super::storage::validate_preview_root_outside_sources(
        scope.catalog_path(),
        scope.preview_root(),
    )?;
    let Some(summary) = summarize_managed_files(&namespace, &cancellation)? else {
        publish(cancelled_event(&operation_id, 0, 0, 0));
        return Ok(());
    };
    if !publish(PreviewCleanupEvent::Started {
        operation_id: operation_id.clone(),
        total_files: summary.files,
        total_bytes: summary.bytes,
    }) {
        return Ok(());
    }
    if cancellation.load(Ordering::Acquire) {
        publish(cancelled_event(&operation_id, 0, 0, 0));
        return Ok(());
    }

    scope.prepare()?;

    let mut processed_files = 0_u64;
    let mut removed_files = 0_u64;
    let mut removed_bytes = 0_u64;
    let mut issue_count = 0_u64;
    if let Some(preview_root) = namespace.root() {
        let entries = fs::read_dir(preview_root).map_err(|error| {
            ScanError::new(
                "preview_cleanup_directory_unavailable",
                format!("Could not inspect the preview cache for cleanup: {error}"),
            )
        })?;
        for entry in entries {
            if cancellation.load(Ordering::Acquire) {
                publish(cancelled_event(
                    &operation_id,
                    removed_files,
                    removed_bytes,
                    issue_count,
                ));
                return Ok(());
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    issue_count = issue_count.saturating_add(1);
                    if !publish(PreviewCleanupEvent::Issue {
                        operation_id: operation_id.clone(),
                        issue: ScanIssue {
                            path: None,
                            code: "preview_cleanup_entry_unavailable".to_owned(),
                            message: error.to_string(),
                        },
                    }) {
                        return Ok(());
                    }
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_file() || !is_ame_preview_cache_entry(&path) {
                continue;
            }
            processed_files = processed_files.saturating_add(1);
            let byte_size = entry.metadata().map_or(0, |metadata| metadata.len());
            match fs::remove_file(&path) {
                Ok(()) => {
                    removed_files = removed_files.saturating_add(1);
                    removed_bytes = removed_bytes.saturating_add(byte_size);
                }
                Err(error) => {
                    issue_count = issue_count.saturating_add(1);
                    if !publish(PreviewCleanupEvent::Issue {
                        operation_id: operation_id.clone(),
                        issue: ScanIssue {
                            path: Some(user_visible_path(&path.to_string_lossy())),
                            code: "preview_cleanup_remove_failed".to_owned(),
                            message: error.to_string(),
                        },
                    }) {
                        return Ok(());
                    }
                }
            }
            if (processed_files.is_multiple_of(PROGRESS_INTERVAL)
                || processed_files == summary.files)
                && !publish(progress_event(
                    &operation_id,
                    processed_files,
                    removed_files,
                    removed_bytes,
                    issue_count,
                    &summary,
                ))
            {
                return Ok(());
            }
        }
    }
    if processed_files != 0 && !processed_files.is_multiple_of(PROGRESS_INTERVAL) {
        publish(progress_event(
            &operation_id,
            processed_files,
            removed_files,
            removed_bytes,
            issue_count,
            &summary,
        ));
    }
    if cancellation.load(Ordering::Acquire) {
        publish(cancelled_event(
            &operation_id,
            removed_files,
            removed_bytes,
            issue_count,
        ));
        return Ok(());
    }
    let Some(remaining) = summarize_managed_files(&namespace, &cancellation)? else {
        publish(cancelled_event(
            &operation_id,
            removed_files,
            removed_bytes,
            issue_count,
        ));
        return Ok(());
    };
    if remaining.files == 0 && issue_count == 0 {
        scope.finish()?;
    } else if remaining.files != 0 {
        issue_count = issue_count.saturating_add(1);
        if !publish(PreviewCleanupEvent::Issue {
            operation_id: operation_id.clone(),
            issue: ScanIssue {
                path: Some(user_visible_path(&scope.preview_root().to_string_lossy())),
                code: "preview_cleanup_verification_failed".to_owned(),
                message: "Managed preview files remain after cleanup verification".to_owned(),
            },
        }) {
            return Ok(());
        }
    }
    publish(PreviewCleanupEvent::Completed {
        operation_id,
        removed_files,
        removed_bytes,
        issue_count,
    });
    Ok(())
}

fn summarize_managed_files(
    namespace: &PreviewCacheNamespace,
    cancellation: &AtomicBool,
) -> Result<Option<CleanupSummary>, ScanError> {
    let Some(preview_root) = namespace.root() else {
        return Ok(Some(CleanupSummary { files: 0, bytes: 0 }));
    };
    let entries = fs::read_dir(preview_root).map_err(|error| {
        ScanError::new(
            "preview_cleanup_directory_unavailable",
            format!("Could not inspect the preview cache for cleanup: {error}"),
        )
    })?;
    let mut summary = CleanupSummary { files: 0, bytes: 0 };
    for entry in entries {
        if cancellation.load(Ordering::Acquire) {
            return Ok(None);
        }
        let entry = entry.map_err(|error| {
            ScanError::new("preview_cleanup_entry_unavailable", error.to_string())
        })?;
        let path = entry.path();
        if !path.is_file() || !is_ame_preview_cache_entry(&path) {
            continue;
        }
        summary.files = summary.files.saturating_add(1);
        if let Ok(metadata) = entry.metadata() {
            summary.bytes = summary.bytes.saturating_add(metadata.len());
        }
    }
    Ok(Some(summary))
}

fn progress_event(
    operation_id: &str,
    processed_files: u64,
    removed_files: u64,
    removed_bytes: u64,
    issue_count: u64,
    summary: &CleanupSummary,
) -> PreviewCleanupEvent {
    PreviewCleanupEvent::Progress {
        operation_id: operation_id.to_owned(),
        processed_files,
        removed_files,
        removed_bytes,
        issue_count,
        total_files: summary.files,
        total_bytes: summary.bytes,
    }
}

fn cancelled_event(
    operation_id: &str,
    removed_files: u64,
    removed_bytes: u64,
    issue_count: u64,
) -> PreviewCleanupEvent {
    PreviewCleanupEvent::Cancelled {
        operation_id: operation_id.to_owned(),
        removed_files,
        removed_bytes,
        issue_count,
    }
}

fn validate_operation_id(operation_id: &str) -> Result<(), ScanError> {
    if operation_id.trim().is_empty() {
        return Err(ScanError::new(
            "preview_cleanup_id_empty",
            "The preview cleanup operation identifier is required",
        ));
    }
    Ok(())
}

fn register_cleanup(operation_id: &str) -> Result<Arc<AtomicBool>, ScanError> {
    let mut cleanup = active_cleanup()
        .lock()
        .map_err(|_| preview_cleanup_registry_error())?;
    if cleanup.is_some() {
        return Err(ScanError::new(
            "preview_cleanup_already_active",
            "Another preview cleanup is already active",
        ));
    }
    let cancellation = Arc::new(AtomicBool::new(false));
    *cleanup = Some(ActiveCleanup {
        operation_id: operation_id.to_owned(),
        cancellation: Arc::clone(&cancellation),
    });
    Ok(cancellation)
}

fn preview_access() -> &'static PreviewAccess {
    PREVIEW_ACCESS.get_or_init(|| PreviewAccess {
        state: Mutex::new(PreviewAccessState::default()),
        changed: Condvar::new(),
    })
}

impl PreviewAccess {
    fn acquire_generation(&'static self) -> Result<PreviewGenerationGuard, ScanError> {
        let mut state = self.state.lock().map_err(|_| preview_access_error())?;
        let ticket = state.next_generation_ticket;
        state.next_generation_ticket =
            state.next_generation_ticket.checked_add(1).ok_or_else(|| {
                ScanError::new(
                    "preview_access_ticket_exhausted",
                    "Preview access cannot admit another generation ticket",
                )
            })?;
        state.generation_waiters = state.generation_waiters.saturating_add(1);
        while state.recovery_active
            || (state.recovery_waiters > 0
                && !state
                    .foreground_turn_through
                    .is_some_and(|through| ticket <= through))
        {
            state = self
                .changed
                .wait(state)
                .map_err(|_| preview_access_error())?;
        }
        state.generation_waiters = state.generation_waiters.saturating_sub(1);
        if state.recovery_waiters > 0 {
            state.foreground_turn_remaining = state.foreground_turn_remaining.saturating_sub(1);
        }
        state.active_generations = state.active_generations.saturating_add(1);
        Ok(PreviewGenerationGuard { access: self })
    }

    fn acquire_reclamation(&'static self) -> Result<PreviewReclamationGuard, ScanError> {
        let mut state = self.state.lock().map_err(|_| preview_access_error())?;
        state.recovery_waiters = state.recovery_waiters.saturating_add(1);
        while state.recovery_active
            || state.active_generations > 0
            || state.foreground_turn_remaining > 0
        {
            state = self
                .changed
                .wait(state)
                .map_err(|_| preview_access_error())?;
        }
        state.recovery_waiters = state.recovery_waiters.saturating_sub(1);
        state.recovery_active = true;
        state.foreground_turn_through = None;
        Ok(PreviewReclamationGuard { access: self })
    }

    fn generation_waiter_count(&self) -> usize {
        self.state
            .lock()
            .map_or(0, |state| state.generation_waiters)
    }
}

impl Drop for PreviewGenerationGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.access.state.lock() {
            state.active_generations = state.active_generations.saturating_sub(1);
            self.access.changed.notify_all();
        }
    }
}

impl Drop for PreviewReclamationGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.access.state.lock() {
            state.recovery_active = false;
            if state.recovery_waiters > 0 && state.generation_waiters > 0 {
                state.foreground_turn_through =
                    Some(state.next_generation_ticket.saturating_sub(1));
                state.foreground_turn_remaining = state.generation_waiters;
            } else {
                state.foreground_turn_through = None;
                state.foreground_turn_remaining = 0;
            }
            self.access.changed.notify_all();
        }
    }
}

fn preview_access_error() -> ScanError {
    ScanError::new("preview_access_unavailable", "Preview access is poisoned")
}

fn active_cleanup() -> &'static Mutex<Option<ActiveCleanup>> {
    ACTIVE_CLEANUP.get_or_init(|| Mutex::new(None))
}

fn preview_cleanup_registry_error() -> ScanError {
    ScanError::new(
        "preview_cleanup_registry_unavailable",
        "Preview cleanup registry is poisoned",
    )
}

struct CleanupRegistration {
    operation_id: String,
}

impl Drop for CleanupRegistration {
    fn drop(&mut self) {
        if let Ok(mut cleanup) = active_cleanup().lock()
            && cleanup
                .as_ref()
                .is_some_and(|active| active.operation_id == self.operation_id)
        {
            *cleanup = None;
        }
    }
}

struct CleanupSummary {
    files: u64,
    bytes: u64,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use tempfile::tempdir;

    use crate::domain::{
        AssetLocationView, PreviewArtifact, PreviewStatus, ScanRequest, SourceRevisionEvidence,
        StorageConfiguration,
    };

    use super::*;

    #[test]
    fn waiting_recovery_blocks_late_generations_until_its_bounded_turn_finishes() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let active_generation = acquire_preview_generation().expect("active preview generation");
        let (events_tx, events_rx) = mpsc::channel();
        let (release_recovery_tx, release_recovery_rx) = mpsc::channel();
        let recovery_events = events_tx.clone();
        let recovery = thread::spawn(move || {
            let access = acquire_preview_reclamation().expect("waiting preview recovery");
            recovery_events.send("recovery").expect("recovery event");
            release_recovery_rx.recv().expect("release recovery");
            drop(access);
        });
        wait_for_access_state(|state| state.recovery_waiters == 1);

        let generation_events = events_tx.clone();
        let generation = thread::spawn(move || {
            let _access = acquire_preview_generation().expect("late preview generation");
            generation_events
                .send("generation")
                .expect("generation event");
        });
        wait_for_access_state(|state| state.generation_waiters == 1);
        drop(active_generation);

        assert_eq!(
            events_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("recovery must converge"),
            "recovery",
        );
        assert!(
            events_rx.recv_timeout(Duration::from_millis(20)).is_err(),
            "a late generation must not enter the active recovery turn",
        );
        release_recovery_tx.send(()).expect("finish recovery");
        assert_eq!(
            events_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("generation after recovery"),
            "generation",
        );
        recovery.join().expect("recovery thread");
        generation.join().expect("generation thread");
    }

    #[test]
    fn queued_foreground_generation_runs_before_a_second_recovery_turn() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let first_recovery = acquire_preview_reclamation().expect("first recovery turn");
        let (events_tx, events_rx) = mpsc::channel();
        let second_recovery_events = events_tx.clone();
        let second_recovery = thread::spawn(move || {
            let _access = acquire_preview_reclamation().expect("second recovery turn");
            second_recovery_events
                .send("recovery")
                .expect("second recovery event");
        });
        wait_for_access_state(|state| state.recovery_waiters == 1);

        let (release_generation_tx, release_generation_rx) = mpsc::channel();
        let generation_events = events_tx.clone();
        let generation = thread::spawn(move || {
            let access = acquire_preview_generation().expect("queued foreground generation");
            generation_events
                .send("generation")
                .expect("foreground event");
            release_generation_rx.recv().expect("release generation");
            drop(access);
        });
        wait_for_access_state(|state| state.generation_waiters == 1);
        drop(first_recovery);

        assert_eq!(
            events_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("foreground turn"),
            "generation",
        );
        assert!(
            events_rx.recv_timeout(Duration::from_millis(20)).is_err(),
            "the next recovery must wait for the admitted foreground cohort",
        );
        release_generation_tx.send(()).expect("finish generation");
        assert_eq!(
            events_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("second recovery turn"),
            "recovery",
        );
        generation.join().expect("generation thread");
        second_recovery.join().expect("second recovery thread");
    }

    fn wait_for_access_state(predicate: impl Fn(&PreviewAccessState) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let matches = preview_access()
                .state
                .lock()
                .is_ok_and(|state| predicate(&state));
            if matches {
                return;
            }
            assert!(Instant::now() < deadline, "preview access state timed out");
            thread::yield_now();
        }
    }

    #[test]
    fn cleanup_removes_only_managed_previews_and_preserves_geometry() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let storage = tempdir().expect("storage");
        let preview_root = storage.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let artifact_path = preview_root.join(format!(
            "ame-jpeg-thumbnail-v2-orientation-{}.jpg",
            "a".repeat(64)
        ));
        let temporary_path = preview_root.join(format!(
            "ame-jpeg-thumbnail-v2-orientation-{}.123-1.tmp",
            "b".repeat(64)
        ));
        let legacy_path = preview_root.join(format!("{}.jpg", "e".repeat(64)));
        let legacy_temporary_path = preview_root.join(format!("{}.456-2.tmp", "f".repeat(64)));
        let foreign_hash_path = preview_root.join(format!("{}.jpg", "F".repeat(64)));
        let unrelated_path = preview_root.join("keep.txt");
        fs::write(&artifact_path, b"artifact").expect("artifact");
        fs::write(&temporary_path, b"temporary").expect("temporary");
        fs::write(&legacy_path, b"legacy").expect("legacy");
        fs::write(&legacy_temporary_path, b"legacy temporary").expect("legacy temporary");
        fs::write(&foreign_hash_path, b"foreign hash").expect("foreign hash");
        fs::write(&unrelated_path, b"unrelated").expect("unrelated");
        let storage_paths = StoragePaths {
            catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
            preview_root: preview_root.clone(),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        publish_ready_location(&storage_paths, &artifact_path);
        let preview_store_before =
            crate::application::preview::active_preview_store(&storage_paths)
                .expect("active preview store before cleanup");
        let mut events = Vec::new();

        clear_previews_with_storage(
            "cleanup-test".to_owned(),
            |event| {
                events.push(event);
                true
            },
            storage_paths.clone(),
        )
        .expect("preview cleanup");

        assert!(!artifact_path.exists());
        assert!(!temporary_path.exists());
        assert!(!legacy_path.exists());
        assert!(!legacy_temporary_path.exists());
        assert_eq!(
            fs::read(&foreign_hash_path).expect("foreign hash after"),
            b"foreign hash"
        );
        assert_eq!(
            fs::read(&unrelated_path).expect("unrelated after"),
            b"unrelated"
        );
        let catalog = SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("catalog");
        let location = catalog
            .load_active_location("cleanup-location")
            .expect("location query")
            .expect("location");
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert!(location.preview_path.is_empty());
        assert_eq!((location.width, location.height), (4_032, 3_024));
        let preview_store_after = crate::application::preview::active_preview_store(&storage_paths)
            .expect("active preview store after cleanup");
        assert!(!Arc::ptr_eq(&preview_store_before, &preview_store_after));
        assert!(matches!(
            events.last(),
            Some(PreviewCleanupEvent::Completed {
                removed_files: 4,
                issue_count: 0,
                ..
            })
        ));
    }

    #[test]
    fn cancellation_before_catalog_reset_leaves_existing_preview_ready() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let storage = tempdir().expect("storage");
        let preview_root = storage.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let artifact_path = preview_root.join(format!(
            "ame-jpeg-thumbnail-v2-orientation-{}.jpg",
            "c".repeat(64)
        ));
        fs::write(&artifact_path, b"artifact").expect("artifact");
        let storage_paths = StoragePaths {
            catalog_path: storage.path().join("catalog").join("ame.sqlite3"),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        publish_ready_location(&storage_paths, &artifact_path);
        let mut events = Vec::new();

        clear_previews_with_storage(
            "cancel-cleanup-test".to_owned(),
            |event| {
                let is_started = matches!(event, PreviewCleanupEvent::Started { .. });
                events.push(event);
                if is_started {
                    assert!(cancel_preview_cleanup("cancel-cleanup-test"));
                }
                true
            },
            storage_paths.clone(),
        )
        .expect("cancelled preview cleanup");

        assert!(artifact_path.exists());
        let catalog = SqliteCatalog::open(storage_paths.catalog_path).expect("catalog");
        let location = catalog
            .load_active_location("cleanup-location")
            .expect("location query")
            .expect("location");
        assert!(matches!(location.preview_status, PreviewStatus::Ready));
        assert!(matches!(
            events.last(),
            Some(PreviewCleanupEvent::Cancelled {
                removed_files: 0,
                ..
            })
        ));
    }

    #[test]
    fn cleanup_rejects_a_preview_root_that_resolves_inside_a_source() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let storage = tempdir().expect("storage");
        let catalog_parent = storage.path().join("catalog");
        let preview_root = catalog_parent.join("source");
        fs::create_dir_all(&preview_root).expect("preview root");
        let artifact_path = preview_root.join(format!(
            "ame-jpeg-thumbnail-v2-orientation-{}.jpg",
            "d".repeat(64)
        ));
        fs::write(&artifact_path, b"source bytes").expect("source-like artifact");
        let storage_paths = StoragePaths {
            catalog_path: catalog_parent.join("ame.sqlite3"),
            preview_root,
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        publish_ready_location(&storage_paths, &artifact_path);

        let error =
            clear_previews_with_storage("overlap-cleanup-test".to_owned(), |_| true, storage_paths)
                .expect_err("overlapping cleanup must be rejected");

        assert_eq!(error.code, "preview_root_overlaps_source");
        assert_eq!(
            fs::read(&artifact_path).expect("preserved source-like file"),
            b"source bytes"
        );
    }

    #[test]
    fn retired_cleanup_requires_owned_root_and_preserves_foreign_files() {
        let _test_lock = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
            .lock()
            .expect("preview lifecycle test lock");
        let storage = tempdir().expect("storage");
        let retired_root = storage.path().join("retired-previews");
        let active_root = storage.path().join("active-previews");
        fs::create_dir_all(&retired_root).expect("retired preview root");
        let artifact_path = retired_root.join(format!(
            "ame-jpeg-thumbnail-v2-orientation-{}.jpg",
            "d".repeat(64)
        ));
        let foreign_path = retired_root.join("keep.txt");
        fs::write(&artifact_path, b"artifact").expect("artifact");
        fs::write(&foreign_path, b"foreign").expect("foreign");
        let settings_path = storage.path().join("settings.sqlite3");
        let initial = StorageConfiguration {
            catalog_path: storage
                .path()
                .join("catalog.sqlite3")
                .to_string_lossy()
                .into_owned(),
            preview_root: retired_root.to_string_lossy().into_owned(),
            preview_budget_bytes: 64 * 1024 * 1024,
        };
        let active = StorageConfiguration {
            preview_root: active_root.to_string_lossy().into_owned(),
            ..initial.clone()
        };
        let mut settings = SqliteStorageSettings::open(settings_path.clone()).expect("settings");
        settings
            .load_or_initialize(&initial)
            .expect("initial settings");
        settings
            .save(&active, Some(&initial.preview_root))
            .expect("pending root");
        settings
            .activate_preview_root(&active.preview_root)
            .expect("retire old root");
        drop(settings);
        let mut events = Vec::new();

        clear_preview_scope(
            "retired-cleanup-test".to_owned(),
            |event| {
                events.push(event);
                true
            },
            CleanupScope::Retired {
                preview_root: retired_root.clone(),
                catalog_path: PathBuf::from(&initial.catalog_path),
                settings_path: settings_path.clone(),
                stored_preview_root: initial.preview_root,
            },
        )
        .expect("retired preview cleanup");

        assert!(!artifact_path.exists());
        assert_eq!(fs::read(&foreign_path).expect("foreign after"), b"foreign");
        let mut settings = SqliteStorageSettings::open(settings_path).expect("settings");
        assert!(
            settings
                .load_retired_preview_roots()
                .expect("retired roots")
                .is_empty()
        );
        assert!(matches!(
            events.last(),
            Some(PreviewCleanupEvent::Completed {
                removed_files: 1,
                issue_count: 0,
                ..
            })
        ));
    }

    fn publish_ready_location(storage: &StoragePaths, artifact_path: &Path) {
        let mut catalog = SqliteCatalog::open(storage.catalog_path.clone()).expect("catalog");
        let request = ScanRequest {
            scan_id: "cleanup-scan".to_owned(),
            root_path: storage
                .catalog_path
                .parent()
                .expect("catalog parent")
                .join("source")
                .to_string_lossy()
                .into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 512,
        };
        catalog
            .begin_scan(&request, "cleanup-root", &request.root_path)
            .expect("begin scan");
        catalog
            .prove_live_only_first_import_handoff_for_test(&request.scan_id)
            .expect("prove cleanup fixture first-import handoff");
        catalog
            .stage_location(
                &request.scan_id,
                "cleanup-root",
                &AssetLocationView {
                    asset_id: "cleanup-asset".to_owned(),
                    location_id: "cleanup-location".to_owned(),
                    root_id: "cleanup-root".to_owned(),
                    scan_id: request.scan_id.clone(),
                    absolute_path: PathBuf::from(&request.root_path)
                        .join("one.png")
                        .to_string_lossy()
                        .into_owned(),
                    display_path: "source\\one.png".to_owned(),
                    relative_path: "one.png".to_owned(),
                    preview_path: String::new(),
                    file_size: 100,
                    created_unix_ms: Some(10),
                    modified_unix_ms: 20,
                    file_identity: None,
                    source_revision: Some(SourceRevisionEvidence {
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
                },
            )
            .expect("stage location");
        catalog
            .publish_scan(&request.scan_id, "cleanup-root", 1, 0)
            .expect("publish scan");
        let mut location = catalog
            .load_active_location("cleanup-location")
            .expect("active location query")
            .expect("active location");
        location.preview_path = artifact_path.to_string_lossy().into_owned();
        location.preview_status = PreviewStatus::Ready;
        let artifact = PreviewArtifact {
            artifact_key: "cleanup-artifact".to_owned(),
            algorithm_id: "ame-jpeg-thumbnail".to_owned(),
            algorithm_version: 2,
            orientation_contract: "exif-display-v1".to_owned(),
            size_bucket: 512,
            path: artifact_path.to_string_lossy().into_owned(),
            byte_size: fs::metadata(artifact_path)
                .expect("artifact metadata")
                .len(),
            encoded_width: 512,
            encoded_height: 384,
            width: location.width,
            height: location.height,
        };
        catalog
            .update_active_preview(&location, Some(&artifact), None)
            .expect("publish preview artifact");
    }
}
