use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::domain::ScanError;

use super::{normalized_path, normalized_path_is_within, resolved_normalized_path};

const MAX_ACTIVE_RESERVATIONS: usize = 256;
static RESERVATIONS: OnceLock<Mutex<Vec<Weak<Reservation>>>> = OnceLock::new();

pub(crate) struct SourceRegistrationPermit {
    _reservation: Arc<Reservation>,
}

pub(crate) struct PreviewCleanupPermit {
    _reservation: Arc<Reservation>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    RegisterSource,
    DeletePreviews,
}

struct Reservation {
    scope: PathScope,
    access: Access,
}

struct PathScope {
    lexical: String,
    physical: String,
}

/// Held through the root-registration commit, not the scan's enumeration or publication.
pub(crate) fn reserve_source_registration(
    root: &Path,
) -> Result<SourceRegistrationPermit, ScanError> {
    Ok(SourceRegistrationPermit {
        _reservation: reserve(root, Access::RegisterSource)?,
    })
}

/// Held from the catalog's source-overlap check through the final filesystem mutation.
pub(crate) fn reserve_preview_cleanup(root: &Path) -> Result<PreviewCleanupPermit, ScanError> {
    Ok(PreviewCleanupPermit {
        _reservation: reserve(root, Access::DeletePreviews)?,
    })
}

fn reserve(root: &Path, access: Access) -> Result<Arc<Reservation>, ScanError> {
    // Resolution can inspect filesystem aliases; it must never hold the admission registry.
    let reservation = Arc::new(Reservation {
        scope: PathScope {
            lexical: normalized_path(root),
            physical: resolved_normalized_path(root)?,
        },
        access,
    });
    let mut entries = RESERVATIONS
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|_| unavailable())?;
    entries.retain(|entry| entry.strong_count() > 0);
    for active in entries.iter().filter_map(Weak::upgrade) {
        if (access == Access::DeletePreviews || active.access == Access::DeletePreviews)
            && reservation.scope.overlaps(&active.scope)
        {
            return Err(match access {
                Access::RegisterSource => ScanError::new(
                    "source_root_cleanup_active",
                    "所选文件夹正在清理预览缓存，请等待清理结束后再添加。尚未登记来源或开始扫描。",
                ),
                Access::DeletePreviews => ScanError::new(
                    "preview_cleanup_source_registration_active",
                    "此目录正在登记为图库来源，暂时不能清理预览缓存。未删除任何文件。",
                ),
            });
        }
    }
    if entries.len() >= MAX_ACTIVE_RESERVATIONS {
        return Err(unavailable());
    }
    entries.push(Arc::downgrade(&reservation));
    Ok(reservation)
}

impl PathScope {
    fn overlaps(&self, other: &Self) -> bool {
        overlap(&self.lexical, &other.lexical) || overlap(&self.physical, &other.physical)
    }
}

fn overlap(left: &str, right: &str) -> bool {
    normalized_path_is_within(left, right) || normalized_path_is_within(right, left)
}

fn unavailable() -> ScanError {
    ScanError::new(
        "source_cleanup_admission_unavailable",
        "无法安全协调来源登记与缓存清理，请稍后重试。当前操作尚未开始。",
    )
}

#[cfg(test)]
mod tests;
