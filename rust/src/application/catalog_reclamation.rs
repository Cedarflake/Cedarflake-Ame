use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::adapters::SqliteCatalogSpaceMaintenance;
use crate::domain::{
    CatalogAutoVacuumMode, CatalogReclamationPhase, CatalogReclamationSnapshot, CatalogSpaceUsage,
    ScanError,
};
use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogSpaceRepository};

mod operation_registry;

use operation_registry::{CatalogReclamationOperation, ReclamationCompletion, registry};

const RECLAMATION_START_BYTES: u64 = 64 * 1024 * 1024;
const RECLAMATION_START_PERCENT: u64 = 25;
const RECLAMATION_TARGET_BYTES: u64 = 8 * 1024 * 1024;
const RECLAMATION_TARGET_PERCENT: u64 = 5;
const INCREMENTAL_BATCH_PAGES: u32 = 256;
const VACUUM_CAPACITY_MARGIN_BYTES: u64 = 64 * 1024 * 1024;
const BUSY_RETRY_INITIAL_DELAY: Duration = Duration::from_millis(25);
const BUSY_RETRY_MAX_DELAY: Duration = Duration::from_millis(3_200);

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
    registry().preempt(&reclamation_key(path));
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
    registry().cancel(operation_id)
}

pub(crate) fn catalog_reclamation_snapshot(
    path: &Path,
) -> Result<CatalogReclamationSnapshot, ScanError> {
    let key = reclamation_key(path);
    if let Some(snapshot) = registry().snapshot(&key)? {
        return Ok(snapshot);
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
    let admission = registry().admit(reclamation_key(&path), operation_id, replace_terminal)?;
    let Some(operation) = admission.worker else {
        return Ok(admission.snapshot);
    };
    let worker_operation = operation.clone();
    if let Err(error) = thread::Builder::new()
        .name("ame-catalog-reclamation".to_owned())
        .spawn(move || run_reclamation(path, worker_operation))
    {
        operation.worker_unavailable(ScanError::new(
            "catalog_reclamation_worker_unavailable",
            format!("Could not start catalog space reclamation: {error}"),
        ));
    }
    operation.snapshot()
}

fn run_reclamation(path: PathBuf, mut operation: CatalogReclamationOperation) {
    loop {
        let result = run_reclamation_inner(&path, &operation);
        match operation.finish(result) {
            Ok(Some(next)) => operation = next,
            Ok(None) => return,
            Err(error) => {
                operation.worker_unavailable(error);
                return;
            }
        }
    }
}

fn run_reclamation_inner(
    path: &Path,
    operation: &CatalogReclamationOperation,
) -> Result<ReclamationCompletion, ScanError> {
    operation.update(|snapshot| {
        snapshot.phase = CatalogReclamationPhase::Inspecting;
        snapshot.error_code = None;
        snapshot.error_message = None;
        snapshot.required_temporary_bytes = None;
        snapshot.available_temporary_bytes = None;
    });
    let maintenance = SqliteCatalogSpaceMaintenance::new(path.to_path_buf());
    let Some(mut usage) = inspect_with_retry(&maintenance, operation)? else {
        return Ok(ReclamationCompletion::Cancelled);
    };
    operation.update(|snapshot| apply_usage(snapshot, usage));

    if usage.auto_vacuum == CatalogAutoVacuumMode::Full {
        let Some(converted) = convert_with_retry(path, &maintenance, operation)? else {
            return Ok(ReclamationCompletion::Cancelled);
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
            return Ok(ReclamationCompletion::Cancelled);
        };
        usage = converted;
        operation.update(|snapshot| apply_usage(snapshot, usage));
    }

    if should_start_reclamation(usage) && usage.auto_vacuum == CatalogAutoVacuumMode::Incremental {
        while !reclamation_target_reached(usage) {
            if operation.is_cancelled() {
                return Ok(ReclamationCompletion::Cancelled);
            }
            operation.update(|snapshot| snapshot.phase = CatalogReclamationPhase::Reclaiming);
            let Some(next) = reclaim_batch_with_retry(path, &maintenance, operation)? else {
                return Ok(ReclamationCompletion::Cancelled);
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

    let Some(()) = checkpoint_with_retry(&maintenance, operation)? else {
        return Ok(ReclamationCompletion::Cancelled);
    };

    Ok(ReclamationCompletion::Completed)
}

fn inspect_with_retry(
    maintenance: &SqliteCatalogSpaceMaintenance,
    operation: &CatalogReclamationOperation,
) -> Result<Option<CatalogSpaceUsage>, ScanError> {
    let mut backoff = BusyRetryBackoff::new();
    loop {
        if operation.is_cancelled() {
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
        super::catalog_session::with_catalog_maintenance(path, || {
            maintenance.try_convert_to_incremental(control)
        })
    })
}

fn reclaim_batch_with_retry(
    path: &Path,
    maintenance: &SqliteCatalogSpaceMaintenance,
    operation: &CatalogReclamationOperation,
) -> Result<Option<CatalogSpaceUsage>, ScanError> {
    retry_controlled_maintenance(operation, CatalogReclamationPhase::Reclaiming, |control| {
        super::catalog_session::with_catalog_maintenance(path, || {
            maintenance.try_reclaim_incremental(INCREMENTAL_BATCH_PAGES, control)
        })
    })
}

fn checkpoint_with_retry(
    maintenance: &SqliteCatalogSpaceMaintenance,
    operation: &CatalogReclamationOperation,
) -> Result<Option<()>, ScanError> {
    retry_controlled_maintenance(operation, CatalogReclamationPhase::Reclaiming, |control| {
        maintenance.try_checkpoint_wal(control)
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
        if operation.is_cancelled() {
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
                if wait(operation, backoff.next_delay()) || operation.is_cancelled() {
                    return Ok(None);
                }
            }
        }
    }
}

fn wait_with_cancellation(operation: &CatalogReclamationOperation, delay: Duration) -> bool {
    let deadline = std::time::Instant::now() + delay;
    while std::time::Instant::now() < deadline {
        if operation.is_cancelled() {
            return true;
        }
        thread::sleep(
            Duration::from_millis(10)
                .min(deadline.saturating_duration_since(std::time::Instant::now())),
        );
    }
    operation.is_cancelled()
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

#[cfg(test)]
mod tests;
