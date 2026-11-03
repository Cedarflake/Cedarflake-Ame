use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::ScanError;
use crate::ports::preview_cache_storage::PreviewCacheStorage;

use super::preview_cache::{PREVIEW_CACHE_VERSION, is_ame_preview_cache_entry};

pub(crate) struct LocalPreviewCacheStorage;

impl PreviewCacheStorage for LocalPreviewCacheStorage {
    fn versioned_root(&self, parent: &Path) -> PathBuf {
        parent.join(PREVIEW_CACHE_VERSION)
    }

    fn used_bytes(&self, path: &Path) -> Result<u64, ScanError> {
        if !path.exists() {
            return Ok(0);
        }
        let entries = fs::read_dir(path).map_err(|error| {
            ScanError::new(
                "storage_usage_unavailable",
                format!("Could not inspect preview cache usage: {error}"),
            )
        })?;
        let mut size = 0_u64;
        for entry in entries {
            let entry = entry
                .map_err(|error| ScanError::new("storage_usage_unavailable", error.to_string()))?;
            if !is_ame_preview_cache_entry(&entry.path()) {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(ScanError::new(
                        "storage_usage_unavailable",
                        error.to_string(),
                    ));
                }
            };
            if metadata.is_file() {
                size = size.saturating_add(metadata.len());
            }
        }
        Ok(size)
    }
}
