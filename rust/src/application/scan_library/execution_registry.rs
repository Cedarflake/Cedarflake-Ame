use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::domain::{LibraryRootGeneration, ScanError};

static ACTIVE_SCANS: OnceLock<Mutex<HashMap<String, ActiveScanEntry>>> = OnceLock::new();

pub(super) const CONTROL_RUNNING: u8 = 0;
pub(super) const CONTROL_PAUSE: u8 = 1;
pub(super) const CONTROL_CANCEL: u8 = 2;
pub(super) const CONTROL_SUSPEND: u8 = 3;
const CONTROL_RETIRED: u8 = 4;

struct ActiveScanEntry {
    owner: ScanOperationOwner,
    catalog_path: Option<PathBuf>,
    protects_catalog_session: bool,
}

enum ScanOperationOwner {
    Executing {
        control: Arc<AtomicU8>,
        first_import: Option<FirstImportCaptureLease>,
    },
    RetainedCancellation,
}

impl ActiveScanEntry {
    fn control(&self) -> Option<&Arc<AtomicU8>> {
        match &self.owner {
            ScanOperationOwner::Executing { control, .. } => Some(control),
            ScanOperationOwner::RetainedCancellation => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct FirstImportCaptureLease {
    scan_id: String,
    root_id: String,
    root_generation: LibraryRootGeneration,
    control: Arc<AtomicU8>,
}

impl FirstImportCaptureLease {
    pub(crate) fn scan_id(&self) -> &str {
        &self.scan_id
    }

    pub(crate) fn is_current(&self) -> bool {
        self.control.load(Ordering::Acquire) == CONTROL_RUNNING
    }

    pub(crate) fn acquire_publication(&self) -> Result<FirstImportPublicationPermit, ScanError> {
        // Called only after SQLite write admission. A request accepted after this
        // reservation is settled by the scan's subsequent terminal write transaction.
        self.control
            .compare_exchange(
                CONTROL_RUNNING,
                CONTROL_RUNNING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| FirstImportPublicationPermit { _private: () })
            .map_err(|_| {
                ScanError::new(
                    "persistent_journal_first_import_inactive",
                    "The first-import publication lost its executing scan owner",
                )
            })
    }
}

#[derive(Debug)]
pub(crate) struct FirstImportPublicationPermit {
    _private: (),
}

pub(super) struct ScanRegistration {
    pub(super) scan_id: String,
}

impl Drop for ScanRegistration {
    fn drop(&mut self) {
        if let Ok(mut scans) = active_scans().lock()
            && let Some(control) = scans.get(&self.scan_id).and_then(ActiveScanEntry::control)
        {
            control.store(CONTROL_RETIRED, Ordering::Release);
            scans.remove(&self.scan_id);
        }
    }
}

fn active_scans() -> &'static Mutex<HashMap<String, ActiveScanEntry>> {
    ACTIVE_SCANS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn registry_unavailable() -> ScanError {
    ScanError::new("scan_registry_unavailable", "Scan registry is poisoned")
}

pub(super) fn register_scan(scan_id: &str) -> Result<Arc<AtomicU8>, ScanError> {
    let mut scans = active_scans().lock().map_err(|_| registry_unavailable())?;
    if scans.contains_key(scan_id) {
        return Err(ScanError::new(
            "scan_already_active",
            "A scan with this identifier is already active",
        ));
    }
    let token = Arc::new(AtomicU8::new(CONTROL_RUNNING));
    scans.insert(
        scan_id.to_owned(),
        ActiveScanEntry {
            owner: ScanOperationOwner::Executing {
                control: Arc::clone(&token),
                first_import: None,
            },
            catalog_path: None,
            protects_catalog_session: false,
        },
    );
    Ok(token)
}

pub(super) struct RetainedCancellationReservation {
    scan_id: String,
}

impl Drop for RetainedCancellationReservation {
    fn drop(&mut self) {
        if let Ok(mut scans) = active_scans().lock()
            && scans
                .get(&self.scan_id)
                .is_some_and(|scan| matches!(scan.owner, ScanOperationOwner::RetainedCancellation))
        {
            scans.remove(&self.scan_id);
        }
    }
}

pub(super) fn reserve_retained_cancellation(
    scan_id: &str,
    catalog_path: &Path,
) -> Result<RetainedCancellationReservation, ScanError> {
    let mut scans = active_scans().lock().map_err(|_| registry_unavailable())?;
    if scans.contains_key(scan_id) {
        return Err(ScanError::new(
            "scan_retained_operation_conflict",
            "The retained scan already has an executing or cancellation owner",
        ));
    }
    scans.insert(
        scan_id.to_owned(),
        ActiveScanEntry {
            owner: ScanOperationOwner::RetainedCancellation,
            catalog_path: Some(catalog_path.to_path_buf()),
            protects_catalog_session: false,
        },
    );
    Ok(RetainedCancellationReservation {
        scan_id: scan_id.to_owned(),
    })
}

pub(super) fn bind_scan_catalog(scan_id: &str, catalog_path: &Path) -> Result<(), ScanError> {
    let mut scans = active_scans().lock().map_err(|_| registry_unavailable())?;
    let scan = scans.get_mut(scan_id).ok_or_else(|| {
        ScanError::new(
            "scan_registration_missing",
            "The active scan registration disappeared before catalog binding",
        )
    })?;
    scan.catalog_path = Some(catalog_path.to_path_buf());
    Ok(())
}

pub(crate) fn protect_scan_catalog_session(
    scan_id: &str,
    catalog_path: &Path,
) -> Result<(), ScanError> {
    let mut scans = active_scans().lock().map_err(|_| registry_unavailable())?;
    let scan = scans.get_mut(scan_id).ok_or_else(|| {
        ScanError::new(
            "scan_registration_missing",
            "The active scan registration disappeared before catalog protection",
        )
    })?;
    if scan.catalog_path.as_deref() != Some(catalog_path) {
        return Err(ScanError::new(
            "scan_catalog_binding_mismatch",
            "The active scan catalog changed before session protection",
        ));
    }
    scan.protects_catalog_session = true;
    Ok(())
}

pub(crate) fn catalog_has_active_scan(
    catalog_path: &Path,
    exempt_scan_id: Option<&str>,
) -> Result<bool, ScanError> {
    let scans = active_scans().lock().map_err(|_| registry_unavailable())?;
    Ok(scans.iter().any(|(scan_id, scan)| {
        exempt_scan_id != Some(scan_id.as_str())
            && scan.protects_catalog_session
            && scan.catalog_path.as_deref() == Some(catalog_path)
    }))
}

pub(super) fn bind_first_import_capture(
    scan_id: &str,
    catalog_path: &Path,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<(), ScanError> {
    let mut scans = active_scans().lock().map_err(|_| registry_unavailable())?;
    let scan = scans.get_mut(scan_id).ok_or_else(|| {
        ScanError::new(
            "scan_registration_missing",
            "The first import lost its executing scan registration",
        )
    })?;
    if !scan.protects_catalog_session || scan.catalog_path.as_deref() != Some(catalog_path) {
        return Err(ScanError::new(
            "scan_catalog_binding_mismatch",
            "First-import capture requires the executing scan's protected catalog",
        ));
    }
    let ScanOperationOwner::Executing {
        control,
        first_import,
    } = &mut scan.owner
    else {
        return Err(ScanError::new(
            "scan_registration_missing",
            "Retained cancellation cannot own first-import observation",
        ));
    };
    *first_import = Some(FirstImportCaptureLease {
        scan_id: scan_id.to_owned(),
        root_id: root_id.to_owned(),
        root_generation,
        control: Arc::clone(control),
    });
    Ok(())
}

pub(crate) fn first_import_capture_lease(
    catalog_path: &Path,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<Option<FirstImportCaptureLease>, ScanError> {
    let scans = active_scans().lock().map_err(|_| registry_unavailable())?;
    Ok(scans.values().find_map(|scan| {
        let ScanOperationOwner::Executing { first_import, .. } = &scan.owner else {
            return None;
        };
        let capture = first_import.as_ref()?;
        (scan.catalog_path.as_deref() == Some(catalog_path)
            && capture.root_id == root_id
            && capture.root_generation == root_generation
            && capture.is_current())
        .then(|| capture.clone())
    }))
}

pub fn cancel_scan(scan_id: &str) -> bool {
    request_control(scan_id, CONTROL_CANCEL)
}

pub fn pause_scan(scan_id: &str) -> bool {
    request_control(scan_id, CONTROL_PAUSE)
}

fn request_control(scan_id: &str, control: u8) -> bool {
    let Ok(scans) = active_scans().lock() else {
        return false;
    };
    let Some(token) = scans.get(scan_id).and_then(ActiveScanEntry::control) else {
        return false;
    };
    let token = Arc::clone(token);
    drop(scans);
    token
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            if current == CONTROL_RETIRED
                || (current == CONTROL_CANCEL && control != CONTROL_CANCEL)
            {
                None
            } else {
                Some(control)
            }
        })
        .is_ok()
}

pub fn suspend_scan(scan_id: &str) -> bool {
    let Ok(scans) = active_scans().lock() else {
        return false;
    };
    let Some(token) = scans.get(scan_id).and_then(ActiveScanEntry::control) else {
        return false;
    };
    let token = Arc::clone(token);
    drop(scans);
    match token.compare_exchange(
        CONTROL_RUNNING,
        CONTROL_SUSPEND,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) | Err(CONTROL_SUSPEND) => true,
        Err(_) => false,
    }
}

#[cfg(test)]
pub(crate) fn hold_first_import_capture(
    scan_id: &str,
    catalog_path: &Path,
    root_id: &str,
    root_generation: LibraryRootGeneration,
) -> Result<impl Drop + use<>, ScanError> {
    register_scan(scan_id)?;
    let registration = ScanRegistration {
        scan_id: scan_id.to_owned(),
    };
    bind_scan_catalog(scan_id, catalog_path)?;
    protect_scan_catalog_session(scan_id, catalog_path)?;
    bind_first_import_capture(scan_id, catalog_path, root_id, root_generation)?;
    Ok(registration)
}
