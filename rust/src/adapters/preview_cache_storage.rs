use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::ScanError;
use crate::ports::preview_cache_storage::{
    PreviewCacheAlgorithm, PreviewCacheDirectory, PreviewCacheEntryKind, PreviewCacheStorage,
    PreviewRootPreparation,
};

use super::PreviewCacheNamespace;
use super::preview_cache::{
    PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION, PREVIEW_CACHE_VERSION,
    PREVIEW_ORIENTATION_CONTRACT, current_preview_artifact_key, is_ame_preview_cache_entry,
    prepare_preview_root,
};

pub(crate) struct LocalPreviewCacheStorage;

impl PreviewCacheStorage for LocalPreviewCacheStorage {
    type Namespace = PreviewCacheNamespace;

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

    fn algorithm(&self) -> PreviewCacheAlgorithm {
        PreviewCacheAlgorithm {
            id: PREVIEW_ALGORITHM_ID,
            version: PREVIEW_ALGORITHM_VERSION,
            orientation_contract: PREVIEW_ORIENTATION_CONTRACT,
        }
    }

    fn current_artifact_key<'path>(&self, path: &'path Path) -> Option<&'path str> {
        current_preview_artifact_key(path)
    }

    fn managed_entry_kind(&self, path: &Path) -> Option<PreviewCacheEntryKind> {
        if !is_ame_preview_cache_entry(path) {
            return None;
        }
        if path.extension().and_then(|extension| extension.to_str()) == Some("tmp") {
            Some(PreviewCacheEntryKind::Temporary)
        } else {
            Some(PreviewCacheEntryKind::Artifact)
        }
    }

    fn prepare_root(&self, root: &Path) -> Result<PreviewRootPreparation, ScanError> {
        prepare_preview_root(root)
    }

    fn open_cleanup_namespace(&self, root: &Path) -> Result<Self::Namespace, ScanError> {
        PreviewCacheNamespace::open(root)
    }

    fn observe_existing_namespace(&self, root: &Path) -> Result<Self::Namespace, ScanError> {
        PreviewCacheNamespace::observe_existing(root)
    }
}

impl PreviewCacheDirectory for PreviewCacheNamespace {
    fn root(&self) -> Option<&Path> {
        self.root()
    }
}
