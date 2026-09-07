use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use crate::domain::ScanError;

use super::{
    StoragePaths, load_configured_storage, resolved_normalized_path, resolved_paths_same,
    validate_source_root_storage_paths,
};

const ADMISSION_TIMEOUT: Duration = Duration::from_secs(5);
static ADMISSIONS: OnceLock<Mutex<HashMap<String, Weak<CatalogAdmission>>>> = OnceLock::new();

#[derive(Default)]
struct CatalogAdmission {
    occupied: Mutex<bool>,
    available: Condvar,
}

pub(super) struct CatalogAdmissionPermit {
    admission: Arc<CatalogAdmission>,
}

/// The permit spans configuration persistence or root registration, never a scan run.
pub(super) fn reserve_catalog_transition(
    storage: &StoragePaths,
) -> Result<CatalogAdmissionPermit, ScanError> {
    admission_for(storage)?.reserve(Instant::now() + ADMISSION_TIMEOUT)
}

fn admission_for(storage: &StoragePaths) -> Result<Arc<CatalogAdmission>, ScanError> {
    // Resolve aliases before locking the registry; no filesystem or database work holds it.
    let identity = resolved_normalized_path(&storage.settings_path)?;
    let admission = {
        let mut entries = ADMISSIONS
            .get_or_init(Mutex::default)
            .lock()
            .map_err(|_| admission_unavailable())?;
        entries.retain(|_, entry| entry.strong_count() > 0);
        if let Some(entry) = entries.get(&identity).and_then(Weak::upgrade) {
            entry
        } else {
            let entry = Arc::new(CatalogAdmission::default());
            entries.insert(identity, Arc::downgrade(&entry));
            entry
        }
    };
    Ok(admission)
}

impl CatalogAdmission {
    fn reserve(self: Arc<Self>, deadline: Instant) -> Result<CatalogAdmissionPermit, ScanError> {
        let mut occupied = self.occupied.lock().map_err(|_| admission_unavailable())?;
        while *occupied {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ScanError::new(
                    "storage_catalog_admission_timeout",
                    "存储设置或图库任务正在提交，请稍后重试。当前操作尚未更改数据库位置或开始导入。",
                ));
            }
            occupied = self
                .available
                .wait_timeout(occupied, remaining)
                .map_err(|_| admission_unavailable())?
                .0;
        }
        *occupied = true;
        drop(occupied);
        Ok(CatalogAdmissionPermit { admission: self })
    }
}

impl Drop for CatalogAdmissionPermit {
    fn drop(&mut self) {
        if let Ok(mut occupied) = self.admission.occupied.lock() {
            *occupied = false;
        }
        self.admission.available.notify_one();
    }
}

pub(crate) fn with_scan_start<T>(
    storage: &StoragePaths,
    source_root: &Path,
    begin: impl FnOnce() -> Result<T, ScanError>,
) -> Result<T, ScanError> {
    let _permit = reserve_catalog_transition(storage)?;
    let _source = super::source_cleanup_admission::reserve_source_registration(source_root)?;
    validate_source_root_storage_paths(source_root, storage)?;
    if storage.settings_path.exists() {
        let configured = load_configured_storage(storage)?;
        if !resolved_paths_same(Path::new(&configured.catalog_path), &storage.catalog_path)? {
            return Err(ScanError::new(
                "catalog_location_restart_required",
                "图库数据位置已更改，请先重启 Ame 再添加或继续导入文件夹。重启前不会向旧数据库写入新的导入任务。",
            ));
        }
    }
    begin()
}

fn admission_unavailable() -> ScanError {
    ScanError::new(
        "storage_catalog_admission_unavailable",
        "无法安全协调存储设置与图库导入，请重启 Ame 后重试。",
    )
}

#[cfg(test)]
mod tests;
