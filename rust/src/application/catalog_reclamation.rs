use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::adapters::SqliteCatalogSpaceMaintenance;
use crate::domain::{
    CatalogAutoVacuumMode, CatalogReclamationPhase, CatalogReclamationSnapshot, CatalogSpaceUsage,
    ScanError,
};
use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogSpaceRepository};

const RECLAMATION_START_BYTES: u64 = 64 * 1024 * 1024;
const RECLAMATION_START_PERCENT: u64 = 25;
const RECLAMATION_TARGET_BYTES: u64 = 8 * 1024 * 1024;
const RECLAMATION_TARGET_PERCENT: u64 = 5;
const INCREMENTAL_BATCH_PAGES: u32 = 256;
const VACUUM_CAPACITY_MARGIN_BYTES: u64 = 64 * 1024 * 1024;
const BUSY_RETRY_INITIAL_DELAY: Duration = Duration::from_millis(25);
const BUSY_RETRY_MAX_DELAY: Duration = Duration::from_millis(3_200);

static OPERATIONS: OnceLock<Mutex<HashMap<PathBuf, Arc<CatalogReclamationOperation>>>> =
    OnceLock::new();
static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

struct BusyRetryBackoff {
    next: Duration,
}

impl BusyRetryBackoff {
    fn new() -> Self {
        Self {
            next: BUSY_RETRY_INITIAL_DELAY,
        }
    }

    fn next_delay(&mut self) -> Duration {
        let delay = self.next;
        self.next = self.next.saturating_mul(2).min(BUSY_RETRY_MAX_DELAY);
        delay
    }
}

struct CatalogReclamationOperation {
    snapshot: Mutex<CatalogReclamationSnapshot>,
    user_cancelled: Arc<AtomicBool>,
    pending_operation_id: Mutex<Option<String>>,
    active_control: Mutex<Option<CatalogMaintenanceControl>>,
}

impl CatalogReclamationOperation {
    fn new(operation_id: String) -> Self {
        let mut snapshot = CatalogReclamationSnapshot::idle();
        snapshot.operation_id = Some(operation_id);
        snapshot.phase = CatalogReclamationPhase::Queued;
        Self {
            snapshot: Mutex::new(snapshot),
            user_cancelled: Arc::new(AtomicBool::new(false)),
            pending_operation_id: Mutex::new(None),
            active_control: Mutex::new(None),
        }
    }

    fn snapshot(&self) -> Result<CatalogReclamationSnapshot, ScanError> {
        self.snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| reclamation_registry_error())
    }

    fn update(&self, update: impl FnOnce(&mut CatalogReclamationSnapshot)) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            update(&mut snapshot);
        }
    }

    fn begin_attempt(&self) -> Result<CatalogMaintenanceControl, ScanError> {
        let control = CatalogMaintenanceControl::new(Arc::clone(&self.user_cancelled));
        let mut active = self
            .active_control
            .lock()
            .map_err(|_| reclamation_registry_error())?;
        *active = Some(control.clone());
        Ok(control)
    }

    fn finish_attempt(&self) {
        if let Ok(mut active) = self.active_control.lock() {
            *active = None;
        }
    }

    fn cancel(&self) {
        self.user_cancelled.store(true, Ordering::Release);
        if let Ok(active) = self.active_control.lock()
            && let Some(control) = active.as_ref()
        {
            control.cancel();
        }
    }

    fn cancel_if_current(&self, operation_id: &str) -> bool {
        let Ok(snapshot) = self.snapshot.lock() else {
            return false;
        };
        if snapshot.operation_id.as_deref() != Some(operation_id) || !snapshot.is_active() {
            return false;
        }
        self.cancel();
        true
    }

    fn preempt(&self) {
        if let Ok(active) = self.active_control.lock()
            && let Some(control) = active.as_ref()
        {
            control.preempt();
        }
    }

    fn request_rescan(&self, operation_id: String) -> Result<(), ScanError> {
        let mut pending = self
            .pending_operation_id
            .lock()
            .map_err(|_| reclamation_registry_error())?;
        *pending = Some(operation_id);
        Ok(())
    }

    fn take_rescan(&self) -> Result<Option<String>, ScanError> {
        self.pending_operation_id
            .lock()
            .map(|mut pending| pending.take())
            .map_err(|_| reclamation_registry_error())
    }

    fn begin_rescan(&self, operation_id: String) {
        self.update(|snapshot| {
            self.user_cancelled.store(false, Ordering::Release);
            *snapshot = CatalogReclamationSnapshot::idle();
            snapshot.operation_id = Some(operation_id);
            snapshot.phase = CatalogReclamationPhase::Queued;
        });
    }
}

pub(crate) fn schedule_catalog_reclamation(path: PathBuf) {
    let operation_id = automatic_operation_id();
    let _ = schedule(path, operation_id, true);
}

#[cfg(not(test))]
pub(crate) fn schedule_catalog_reclamation_recovery(path: PathBuf) {
    let operation_id = automatic_operation_id();
    let _ = schedule(path, operation_id, false);
}

pub(crate) fn preempt_catalog_reclamation(path: &Path) {
    let key = reclamation_key(path);
    let operation = operations()
        .lock()
        .ok()
        .and_then(|operations| operations.get(&key).cloned());
    if let Some(operation) = operation
        && operation
            .snapshot()
            .is_ok_and(|snapshot| snapshot.is_active())
    {
        operation.preempt();
        operation.update(|snapshot| snapshot.phase = CatalogReclamationPhase::WaitingForIdle);
    }
}

pub fn start_catalog_reclamation(
    operation_id: String,
) -> Result<CatalogReclamationSnapshot, ScanError> {
    if operation_id.trim().is_empty() {
        return Err(ScanError::new(
            "catalog_reclamation_id_empty",
            "Catalog reclamation requires an operation identifier",
        ));
    }
    let storage = super::storage_paths()?;
    schedule(storage.catalog_path, operation_id, true)
}

pub fn load_catalog_reclamation() -> Result<CatalogReclamationSnapshot, ScanError> {
    let storage = super::storage_paths()?;
    catalog_reclamation_snapshot(&storage.catalog_path)
}

pub fn cancel_catalog_reclamation(operation_id: &str) -> bool {
    let operation = operations().lock().ok().and_then(|operations| {
        operations.values().find_map(|operation| {
            operation
                .snapshot()
                .ok()
                .filter(|snapshot| {
                    snapshot.operation_id.as_deref() == Some(operation_id) && snapshot.is_active()
                })
                .map(|_| Arc::clone(operation))
        })
    });
    let Some(operation) = operation else {
        return false;
    };
    operation.cancel_if_current(operation_id)
}

pub(crate) fn catalog_reclamation_snapshot(
    path: &Path,
) -> Result<CatalogReclamationSnapshot, ScanError> {
    let key = reclamation_key(path);
    if let Some(operation) = operations()
        .lock()
        .map_err(|_| reclamation_registry_error())?
        .get(&key)
        .cloned()
    {
        return operation.snapshot();
    }
    if !path.is_file() {
        return Ok(CatalogReclamationSnapshot::idle());
    }
    let maintenance = SqliteCatalogSpaceMaintenance::new(path.to_path_buf());
    match maintenance.inspect_catalog_space()? {
        CatalogMaintenanceAttempt::Completed(usage) => Ok(snapshot_for_usage(usage)),
        CatalogMaintenanceAttempt::Busy | CatalogMaintenanceAttempt::Interrupted => {
            let mut snapshot = CatalogReclamationSnapshot::idle();
            snapshot.catalog_file_bytes = fs::metadata(path).map_or(0, |metadata| metadata.len());
            snapshot.live_bytes = snapshot.catalog_file_bytes;
            Ok(snapshot)
        }
    }
}

fn schedule(
    path: PathBuf,
    operation_id: String,
    replace_terminal: bool,
) -> Result<CatalogReclamationSnapshot, ScanError> {
    let key = reclamation_key(&path);
    let operation = {
        let mut operations = operations()
            .lock()
            .map_err(|_| reclamation_registry_error())?;
        if let Some(existing) = operations.get(&key) {
            let snapshot = existing.snapshot()?;
            if snapshot.is_active() {
                if replace_terminal {
                    existing.request_rescan(operation_id)?;
                }
                return Ok(snapshot);
            }
            if !replace_terminal {
                return Ok(snapshot);
            }
        }
        let operation = Arc::new(CatalogReclamationOperation::new(operation_id));
        operations.insert(key, Arc::clone(&operation));
        operation
    };
    let worker_operation = Arc::clone(&operation);
    if let Err(error) = thread::Builder::new()
        .name("ame-catalog-reclamation".to_owned())
        .spawn(move || run_reclamation(path, worker_operation))
    {
        operation.update(|snapshot| {
            snapshot.phase = CatalogReclamationPhase::Failed;
            snapshot.error_code = Some("catalog_reclamation_worker_unavailable".to_owned());
            snapshot.error_message = Some(format!(
                "Could not start catalog space reclamation: {error}"
            ));
        });
    }
    operation.snapshot()
}

fn run_reclamation(path: PathBuf, operation: Arc<CatalogReclamationOperation>) {
    loop {
        let result = run_reclamation_inner(&path, &operation);
        operation.finish_attempt();
        match operation.take_rescan() {
            Ok(Some(operation_id)) => {
                operation.begin_rescan(operation_id);
                continue;
            }
            Ok(None) => {}
            Err(error) => {
                operation.update(|snapshot| {
                    snapshot.phase = CatalogReclamationPhase::Failed;
                    snapshot.error_code = Some(error.code);
                    snapshot.error_message = Some(error.message);
                });
                return;
            }
        }
        match result {
            Ok(()) => {}
            Err(error) if error.code == "catalog_reclamation_cancelled" => {}
            Err(error) => {
                operation.update(|snapshot| {
                    snapshot.phase = CatalogReclamationPhase::Failed;
                    snapshot.error_code = Some(error.code);
                    snapshot.error_message = Some(error.message);
                });
            }
        }
        match operation.take_rescan() {
            Ok(Some(operation_id)) => operation.begin_rescan(operation_id),
            Ok(None) => return,
            Err(error) => {
                operation.update(|snapshot| {
                    snapshot.phase = CatalogReclamationPhase::Failed;
                    snapshot.error_code = Some(error.code);
                    snapshot.error_message = Some(error.message);
                });
                return;
            }
        }
    }
}

fn run_reclamation_inner(
    path: &Path,
    operation: &CatalogReclamationOperation,
) -> Result<(), ScanError> {
    operation.update(|snapshot| {
        snapshot.phase = CatalogReclamationPhase::Inspecting;
        snapshot.error_code = None;
        snapshot.error_message = None;
        snapshot.required_temporary_bytes = None;
        snapshot.available_temporary_bytes = None;
    });
    let maintenance = SqliteCatalogSpaceMaintenance::new(path.to_path_buf());
    let Some(mut usage) = inspect_with_retry(&maintenance, operation)? else {
        return finish_cancelled(operation);
    };
    operation.update(|snapshot| apply_usage(snapshot, usage));

    if usage.auto_vacuum == CatalogAutoVacuumMode::Full {
        let Some(converted) = convert_with_retry(path, &maintenance, operation)? else {
            return finish_cancelled(operation);
        };
        usage = converted;
        operation.update(|snapshot| apply_usage(snapshot, usage));
    }

    if should_start_reclamation(usage) && usage.auto_vacuum == CatalogAutoVacuumMode::None {
        operation.update(|snapshot| snapshot.phase = CatalogReclamationPhase::CheckingCapacity);
        let required = required_vacuum_capacity(usage.catalog_file_bytes())?;
        let available = available_space_for(path)?;
        operation.update(|snapshot| {
            snapshot.required_temporary_bytes = Some(required);
            snapshot.available_temporary_bytes = Some(available);
        });
        if available < required {
            return Err(ScanError::new(
                "catalog_reclamation_space_insufficient",
                format!(
                    "Catalog compaction requires {required} bytes of free space, but only {available} bytes are available"
                ),
            ));
        }
        let Some(converted) = convert_with_retry(path, &maintenance, operation)? else {
            return finish_cancelled(operation);
        };
        usage = converted;
        operation.update(|snapshot| apply_usage(snapshot, usage));
    }

    if should_start_reclamation(usage) && usage.auto_vacuum == CatalogAutoVacuumMode::Incremental {
        while !reclamation_target_reached(usage) {
            if operation.user_cancelled.load(Ordering::Acquire) {
                return finish_cancelled(operation);
            }
            operation.update(|snapshot| snapshot.phase = CatalogReclamationPhase::Reclaiming);
            let Some(next) = reclaim_batch_with_retry(path, &maintenance, operation)? else {
                return finish_cancelled(operation);
            };
            if next.freelist_count >= usage.freelist_count && next.page_count >= usage.page_count {
                return Err(ScanError::new(
                    "catalog_reclamation_no_progress",
                    "Incremental catalog reclamation could not release another page",
                ));
            }
            usage = next;
            operation.update(|snapshot| apply_usage(snapshot, usage));
            thread::yield_now();
        }
    }

    let Some(()) = checkpoint_with_retry(path, &maintenance, operation)? else {
        return finish_cancelled(operation);
    };

    operation.update(|snapshot| snapshot.phase = CatalogReclamationPhase::Completed);
    Ok(())
}

fn inspect_with_retry(
    maintenance: &SqliteCatalogSpaceMaintenance,
    operation: &CatalogReclamationOperation,
) -> Result<Option<CatalogSpaceUsage>, ScanError> {
    let mut backoff = BusyRetryBackoff::new();
    loop {
        if operation.user_cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        match maintenance.inspect_catalog_space()? {
            CatalogMaintenanceAttempt::Completed(usage) => return Ok(Some(usage)),
            CatalogMaintenanceAttempt::Interrupted => return Ok(None),
            CatalogMaintenanceAttempt::Busy => {
                operation
                    .update(|snapshot| snapshot.phase = CatalogReclamationPhase::WaitingForIdle);
                if wait_with_cancellation(operation, backoff.next_delay()) {
                    return Ok(None);
                }
            }
        }
    }
}

fn convert_with_retry(
    path: &Path,
    maintenance: &SqliteCatalogSpaceMaintenance,
    operation: &CatalogReclamationOperation,
) -> Result<Option<CatalogSpaceUsage>, ScanError> {
    retry_controlled_maintenance(operation, CatalogReclamationPhase::Converting, |control| {
        finish_maintenance_attempt(path, maintenance.try_convert_to_incremental(control))
    })
}

fn reclaim_batch_with_retry(
    path: &Path,
    maintenance: &SqliteCatalogSpaceMaintenance,
    operation: &CatalogReclamationOperation,
) -> Result<Option<CatalogSpaceUsage>, ScanError> {
    retry_controlled_maintenance(operation, CatalogReclamationPhase::Reclaiming, |control| {
        finish_maintenance_attempt(
            path,
            maintenance.try_reclaim_incremental(INCREMENTAL_BATCH_PAGES, control),
        )
    })
}

fn checkpoint_with_retry(
    path: &Path,
    maintenance: &SqliteCatalogSpaceMaintenance,
    operation: &CatalogReclamationOperation,
) -> Result<Option<()>, ScanError> {
    retry_controlled_maintenance(operation, CatalogReclamationPhase::Reclaiming, |control| {
        finish_maintenance_attempt(path, maintenance.try_checkpoint_wal(control))
    })
}

fn retry_controlled_maintenance<T>(
    operation: &CatalogReclamationOperation,
    active_phase: CatalogReclamationPhase,
    attempt: impl FnMut(&CatalogMaintenanceControl) -> Result<CatalogMaintenanceAttempt<T>, ScanError>,
) -> Result<Option<T>, ScanError> {
    retry_controlled_maintenance_with_wait(operation, active_phase, attempt, wait_with_cancellation)
}

fn retry_controlled_maintenance_with_wait<T>(
    operation: &CatalogReclamationOperation,
    active_phase: CatalogReclamationPhase,
    mut attempt: impl FnMut(
        &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<T>, ScanError>,
    mut wait: impl FnMut(&CatalogReclamationOperation, Duration) -> bool,
) -> Result<Option<T>, ScanError> {
    let mut backoff = BusyRetryBackoff::new();
    loop {
        if operation.user_cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        operation.update(|snapshot| snapshot.phase = active_phase);
        let control = operation.begin_attempt()?;
        let result = attempt(&control);
        operation.finish_attempt();
        match result? {
            CatalogMaintenanceAttempt::Completed(value) => return Ok(Some(value)),
            CatalogMaintenanceAttempt::Interrupted if control.is_user_cancelled() => {
                return Ok(None);
            }
            CatalogMaintenanceAttempt::Interrupted | CatalogMaintenanceAttempt::Busy => {
                operation
                    .update(|snapshot| snapshot.phase = CatalogReclamationPhase::WaitingForIdle);
                if wait(operation, backoff.next_delay())
                    || operation.user_cancelled.load(Ordering::Acquire)
                {
                    return Ok(None);
                }
            }
        }
    }
}

fn finish_maintenance_attempt<T>(
    path: &Path,
    attempt: Result<CatalogMaintenanceAttempt<T>, ScanError>,
) -> Result<CatalogMaintenanceAttempt<T>, ScanError> {
    if matches!(attempt, Ok(CatalogMaintenanceAttempt::Busy)) {
        return attempt;
    }
    let invalidation = super::catalog_session::invalidate_catalog_session(path);
    match attempt {
        Err(error) => Err(error),
        Ok(value) => {
            invalidation?;
            Ok(value)
        }
    }
}

fn finish_cancelled(operation: &CatalogReclamationOperation) -> Result<(), ScanError> {
    operation.update(|snapshot| snapshot.phase = CatalogReclamationPhase::Cancelled);
    Ok(())
}

fn wait_with_cancellation(operation: &CatalogReclamationOperation, delay: Duration) -> bool {
    let deadline = std::time::Instant::now() + delay;
    while std::time::Instant::now() < deadline {
        if operation.user_cancelled.load(Ordering::Acquire) {
            return true;
        }
        thread::sleep(
            Duration::from_millis(10)
                .min(deadline.saturating_duration_since(std::time::Instant::now())),
        );
    }
    operation.user_cancelled.load(Ordering::Acquire)
}

fn apply_usage(snapshot: &mut CatalogReclamationSnapshot, usage: CatalogSpaceUsage) {
    let previous_total = snapshot
        .reclaimed_bytes
        .saturating_add(snapshot.reclaimable_bytes);
    let total = previous_total.max(usage.reclaimable_bytes());
    snapshot.catalog_file_bytes = usage.catalog_file_bytes();
    snapshot.live_bytes = usage.live_bytes();
    snapshot.reclaimable_bytes = usage.reclaimable_bytes();
    snapshot.reclaimed_bytes = total.saturating_sub(snapshot.reclaimable_bytes);
}

fn snapshot_for_usage(usage: CatalogSpaceUsage) -> CatalogReclamationSnapshot {
    let mut snapshot = CatalogReclamationSnapshot::idle();
    apply_usage(&mut snapshot, usage);
    snapshot
}

fn should_start_reclamation(usage: CatalogSpaceUsage) -> bool {
    usage.reclaimable_bytes() >= RECLAMATION_START_BYTES
        && percentage_at_least(
            usage.freelist_count,
            usage.page_count,
            RECLAMATION_START_PERCENT,
        )
}

fn reclamation_target_reached(usage: CatalogSpaceUsage) -> bool {
    usage.freelist_count == 0
        || usage.reclaimable_bytes() <= RECLAMATION_TARGET_BYTES
        || percentage_at_most(
            usage.freelist_count,
            usage.page_count,
            RECLAMATION_TARGET_PERCENT,
        )
}

fn percentage_at_least(part: u64, total: u64, percent: u64) -> bool {
    total != 0 && u128::from(part) * 100 >= u128::from(total) * u128::from(percent)
}

fn percentage_at_most(part: u64, total: u64, percent: u64) -> bool {
    total != 0 && u128::from(part) * 100 <= u128::from(total) * u128::from(percent)
}

fn required_vacuum_capacity(catalog_file_bytes: u64) -> Result<u64, ScanError> {
    catalog_file_bytes
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(VACUUM_CAPACITY_MARGIN_BYTES))
        .ok_or_else(|| {
            ScanError::new(
                "catalog_reclamation_capacity_overflow",
                "The catalog is too large to calculate a safe compaction capacity",
            )
        })
}

#[cfg(windows)]
fn available_space_for(path: &Path) -> Result<u64, ScanError> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let directory = path.parent().ok_or_else(|| {
        ScanError::new(
            "catalog_reclamation_directory_invalid",
            "The catalog does not have a parent directory",
        )
    })?;
    let wide = directory
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut available = 0_u64;
    // SAFETY: `wide` is a live NUL-terminated UTF-16 buffer for the duration of the call, and the
    // only non-null output points to a valid initialized `u64` owned by this stack frame.
    let succeeded = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if succeeded == 0 {
        return Err(ScanError::new(
            "catalog_reclamation_capacity_unavailable",
            format!(
                "Could not inspect free space for catalog compaction: {}",
                std::io::Error::last_os_error()
            ),
        ));
    }
    Ok(available)
}

#[cfg(all(not(windows), test))]
fn available_space_for(_path: &Path) -> Result<u64, ScanError> {
    Ok(u64::MAX)
}

#[cfg(all(not(windows), not(test)))]
fn available_space_for(_path: &Path) -> Result<u64, ScanError> {
    Err(ScanError::new(
        "catalog_reclamation_platform_unsupported",
        "Catalog compaction is currently supported only on Windows",
    ))
}

fn operations() -> &'static Mutex<HashMap<PathBuf, Arc<CatalogReclamationOperation>>> {
    OPERATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn reclamation_key(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn automatic_operation_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let sequence = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
    format!("catalog-reclamation-{now}-{sequence}")
}

fn reclamation_registry_error() -> ScanError {
    ScanError::new(
        "catalog_reclamation_registry_unavailable",
        "The catalog reclamation task registry is unavailable",
    )
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::*;
    use crate::adapters::SqliteCatalog;

    fn usage(page_count: u64, freelist_count: u64) -> CatalogSpaceUsage {
        CatalogSpaceUsage {
            page_size: 4_096,
            page_count,
            freelist_count,
            auto_vacuum: CatalogAutoVacuumMode::Incremental,
        }
    }

    #[test]
    fn reclamation_requires_both_absolute_and_ratio_thresholds() {
        assert!(should_start_reclamation(usage(100_000, 25_000)));
        assert!(!should_start_reclamation(usage(1_000_000, 20_000)));
        assert!(!should_start_reclamation(usage(10_000, 5_000)));
    }

    #[test]
    fn incremental_reclamation_stops_at_the_low_watermark() {
        assert!(reclamation_target_reached(usage(100_000, 1_000)));
        assert!(reclamation_target_reached(usage(100_000, 5_000)));
        assert!(!reclamation_target_reached(usage(100_000, 6_000)));
    }

    #[test]
    fn incremental_reclamation_keeps_the_256_page_write_window() {
        assert_eq!(INCREMENTAL_BATCH_PAGES, 256);
    }

    #[test]
    fn large_catalog_model_converges_with_bounded_page_batches() {
        let mut current = usage(296_127, 294_956);
        let mut batches = 0_u32;

        while !reclamation_target_reached(current) {
            let reclaimed = current
                .freelist_count
                .min(u64::from(INCREMENTAL_BATCH_PAGES));
            current.page_count -= reclaimed;
            current.freelist_count -= reclaimed;
            batches += 1;
            assert!(batches <= 2_000, "bounded reclamation must converge");
        }

        assert_eq!(batches, 1_145);
        assert_eq!(current.page_count, 3_007);
        assert_eq!(current.freelist_count, 1_836);
        assert!(reclamation_target_reached(current));
    }

    #[test]
    fn transient_maintenance_waits_past_the_old_retry_limit_and_cancels() {
        let operation = CatalogReclamationOperation::new("busy-until-cancelled".to_owned());
        let mut attempts = 0_u32;
        let mut delays = Vec::new();

        let result = retry_controlled_maintenance_with_wait(
            &operation,
            CatalogReclamationPhase::Reclaiming,
            |_| {
                attempts += 1;
                Ok::<_, ScanError>(CatalogMaintenanceAttempt::<()>::Busy)
            },
            |operation, delay| {
                delays.push(delay);
                if delays.len() == 10 {
                    operation.cancel();
                }
                operation.user_cancelled.load(Ordering::Acquire)
            },
        )
        .expect("transient contention must not become a permanent failure");

        assert!(result.is_none());
        assert_eq!(attempts, 10);
        assert_eq!(
            delays,
            vec![
                Duration::from_millis(25),
                Duration::from_millis(50),
                Duration::from_millis(100),
                Duration::from_millis(200),
                Duration::from_millis(400),
                Duration::from_millis(800),
                Duration::from_millis(1_600),
                Duration::from_millis(3_200),
                Duration::from_millis(3_200),
                Duration::from_millis(3_200),
            ]
        );
        finish_cancelled(&operation).expect("publish cancellation");
        assert_eq!(
            operation.snapshot().expect("cancelled snapshot").phase,
            CatalogReclamationPhase::Cancelled
        );
    }

    #[test]
    fn non_transient_maintenance_failure_is_not_retried() {
        let operation = CatalogReclamationOperation::new("hard-failure".to_owned());
        let mut attempts = 0_u32;
        let error = retry_controlled_maintenance_with_wait::<()>(
            &operation,
            CatalogReclamationPhase::Reclaiming,
            |_| {
                attempts += 1;
                Err(ScanError::new(
                    "catalog_reclamation_injected_failure",
                    "Injected permanent failure",
                ))
            },
            |_, _| panic!("permanent failures must not enter backoff"),
        )
        .expect_err("permanent failure must propagate");

        assert_eq!(error.code, "catalog_reclamation_injected_failure");
        assert_eq!(attempts, 1);
    }

    #[test]
    fn a_new_reclamation_request_supersedes_cancellation_without_losing_the_request() {
        let operation = CatalogReclamationOperation::new("cancelled-operation".to_owned());
        operation.cancel();
        operation
            .request_rescan("replacement-operation".to_owned())
            .expect("queue replacement reclamation");

        let operation_id = operation
            .take_rescan()
            .expect("read replacement reclamation")
            .expect("replacement reclamation remains pending");
        operation.begin_rescan(operation_id);

        let snapshot = operation.snapshot().expect("replacement snapshot");
        assert_eq!(
            snapshot.operation_id.as_deref(),
            Some("replacement-operation")
        );
        assert_eq!(snapshot.phase, CatalogReclamationPhase::Queued);
        assert!(!operation.user_cancelled.load(Ordering::Acquire));
        assert!(
            operation
                .take_rescan()
                .expect("read coalesced replacement state")
                .is_none()
        );
    }

    #[test]
    fn an_old_cancellation_cannot_cancel_a_new_reclamation_generation() {
        let operation = Arc::new(CatalogReclamationOperation::new("old-operation".to_owned()));
        let (selected_tx, selected_rx) = std::sync::mpsc::sync_channel(0);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(0);
        let delayed_cancel = {
            let operation = Arc::clone(&operation);
            thread::spawn(move || {
                let selected = operation.snapshot().expect("select old operation");
                assert_eq!(selected.operation_id.as_deref(), Some("old-operation"));
                selected_tx.send(()).expect("report selection");
                release_rx.recv().expect("resume cancellation");
                operation.cancel_if_current("old-operation")
            })
        };
        selected_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("old operation selected");
        operation.begin_rescan("new-operation".to_owned());
        let new_control = operation.begin_attempt().expect("new generation control");
        release_tx.send(()).expect("release old cancellation");
        assert!(!delayed_cancel.join().expect("cancellation worker"));
        assert!(!new_control.is_user_cancelled());
        assert!(operation.cancel_if_current("new-operation"));
        assert!(new_control.is_user_cancelled());
        finish_cancelled(&operation).expect("finish new cancellation");
        assert!(!operation.cancel_if_current("new-operation"));
    }

    #[test]
    fn vacuum_capacity_uses_twice_the_main_database_plus_margin() {
        assert_eq!(
            required_vacuum_capacity(1_024).expect("bounded capacity"),
            2_048 + VACUUM_CAPACITY_MARGIN_BYTES,
        );
        assert_eq!(
            required_vacuum_capacity(u64::MAX).unwrap_err().code,
            "catalog_reclamation_capacity_overflow",
        );
    }

    #[test]
    fn full_auto_vacuum_is_converted_before_reclamation_completes() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        Connection::open(&path)
            .expect("full-mode connection")
            .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = FULL; VACUUM;")
            .expect("enable full auto-vacuum");
        let operation = CatalogReclamationOperation::new("full-mode".to_owned());

        run_reclamation_inner(&path, &operation).expect("reclaim full-mode catalog");

        let maintenance = SqliteCatalogSpaceMaintenance::new(path);
        let after = match maintenance
            .inspect_catalog_space()
            .expect("inspect converted catalog")
        {
            CatalogMaintenanceAttempt::Completed(usage) => usage,
            CatalogMaintenanceAttempt::Busy => panic!("inspection unexpectedly busy"),
            CatalogMaintenanceAttempt::Interrupted => {
                panic!("inspection unexpectedly interrupted")
            }
        };
        assert_eq!(after.auto_vacuum, CatalogAutoVacuumMode::Incremental);
        assert_eq!(
            operation.snapshot().expect("completed snapshot").phase,
            CatalogReclamationPhase::Completed
        );
    }

    #[test]
    fn maintenance_success_and_failure_invalidate_the_cached_catalog_session() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        Connection::open(&path)
            .expect("legacy-mode connection")
            .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = NONE; VACUUM;")
            .expect("create legacy auto-vacuum fixture");
        super::super::catalog_session::reset_catalog_session(&path);
        crate::adapters::reset_full_schema_validation_count(&path);
        drop(
            super::super::catalog_session::open_catalog(
                &path,
                crate::domain::LibraryChangeLane::Recovery,
            )
            .expect("prime validated session"),
        );
        assert_eq!(crate::adapters::full_schema_validation_count(&path), 1);

        let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
        let operation = CatalogReclamationOperation::new("session-invalidation".to_owned());
        let converted = convert_with_retry(&path, &maintenance, &operation)
            .expect("convert legacy catalog")
            .expect("conversion was not cancelled");
        assert_eq!(converted.auto_vacuum, CatalogAutoVacuumMode::Incremental);
        drop(
            super::super::catalog_session::open_catalog(
                &path,
                crate::domain::LibraryChangeLane::Recovery,
            )
            .expect("revalidate after conversion"),
        );
        assert_eq!(crate::adapters::full_schema_validation_count(&path), 2);

        let injected = ScanError::new(
            "catalog_reclamation_injected_failure",
            "Injected maintenance failure",
        );
        assert_eq!(
            finish_maintenance_attempt::<()>(&path, Err(injected.clone()))
                .err()
                .expect("injected failure")
                .code,
            injected.code,
        );
        drop(
            super::super::catalog_session::open_catalog(
                &path,
                crate::domain::LibraryChangeLane::Recovery,
            )
            .expect("revalidate after failed maintenance"),
        );
        assert_eq!(crate::adapters::full_schema_validation_count(&path), 3);
    }

    #[test]
    fn busy_maintenance_retains_the_validated_catalog_session() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        let writer = Connection::open(&path).expect("WAL writer");
        writer
            .execute_batch(
                "PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE busy_checkpoint_fixture(value INTEGER);
             INSERT INTO busy_checkpoint_fixture VALUES (1);",
            )
            .expect("prepare WAL");
        let session = super::super::catalog_session::validated_catalog_session(&path)
            .expect("prime validated session");
        crate::adapters::reset_full_schema_validation_count(&path);
        let reader = Connection::open(&path).expect("WAL reader");
        reader.execute_batch("BEGIN;").expect("hold read snapshot");
        let _: i64 = reader
            .query_row("SELECT COUNT(*) FROM busy_checkpoint_fixture", [], |row| {
                row.get(0)
            })
            .expect("pin read snapshot");
        writer
            .execute("INSERT INTO busy_checkpoint_fixture VALUES (2)", [])
            .expect("append WAL");
        let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));

        assert!(matches!(
            finish_maintenance_attempt(&path, maintenance.try_checkpoint_wal(&control))
                .expect("busy maintenance remains retryable"),
            CatalogMaintenanceAttempt::Busy,
        ));
        let current = super::super::catalog_session::validated_catalog_session(&path)
            .expect("reuse unchanged session");

        assert!(Arc::ptr_eq(&session, &current));
        drop(
            current
                .open_in_lane(crate::domain::LibraryChangeLane::Recovery)
                .expect("native identity and schema signature remain current"),
        );
        assert_eq!(crate::adapters::full_schema_validation_count(&path), 0);
        reader.execute_batch("ROLLBACK;").expect("release reader");
    }

    #[test]
    fn busy_conversion_and_incremental_vacuum_preserve_the_validated_signature() {
        for mode in ["NONE", "FULL", "INCREMENTAL"] {
            let directory = tempdir().expect("catalog directory");
            let path = directory.path().join("ame.sqlite3");
            drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
            let writer = Connection::open(&path).expect("blocking writer");
            writer
                .execute_batch(&format!(
                    "PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = {mode}; VACUUM;
                 CREATE TABLE busy_vacuum_fixture(payload BLOB);
                 INSERT INTO busy_vacuum_fixture VALUES (zeroblob(65536));
                 DROP TABLE busy_vacuum_fixture;"
                ))
                .expect("prepare reclamation mode");
            let session = super::super::catalog_session::validated_catalog_session(&path)
                .expect("prime validated session");
            crate::adapters::reset_full_schema_validation_count(&path);
            let before: i64 = writer
                .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
                .expect("original mode");
            writer
                .execute_batch("BEGIN IMMEDIATE;")
                .expect("hold SQLite writer");
            let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
            let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
            let attempt = if mode == "INCREMENTAL" {
                maintenance.try_reclaim_incremental(1, &control)
            } else {
                maintenance.try_convert_to_incremental(&control)
            };

            assert!(
                matches!(
                    finish_maintenance_attempt(&path, attempt).expect("busy result"),
                    CatalogMaintenanceAttempt::Busy
                ),
                "{mode}"
            );
            writer
                .execute_batch("ROLLBACK;")
                .expect("release SQLite writer");
            let current = super::super::catalog_session::validated_catalog_session(&path)
                .expect("reuse unchanged session");
            assert!(Arc::ptr_eq(&session, &current), "{mode}");
            drop(
                current
                    .open_in_lane(crate::domain::LibraryChangeLane::Recovery)
                    .expect("native identity and schema signature remain current"),
            );
            assert_eq!(
                crate::adapters::full_schema_validation_count(&path),
                0,
                "{mode}"
            );
            let after: i64 = writer
                .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
                .expect("retained mode");
            assert_eq!(before, after, "{mode}");
        }
    }
}
