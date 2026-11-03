use std::path::{Path, PathBuf};

use crate::domain::ScanError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreviewRootPreparation {
    Ready,
    NamespaceUnavailable,
}

pub(crate) struct PreviewCacheAlgorithm {
    pub id: &'static str,
    pub version: u32,
    pub orientation_contract: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewCacheEntryKind {
    Artifact,
    Temporary,
}

/// Holds the observed root and its ancestor identities until the operation retires.
/// An absent root grants no authority over a directory created later at that path.
pub(crate) trait PreviewCacheDirectory {
    fn root(&self) -> Option<&Path>;
}

pub(crate) trait PreviewCacheStorage {
    type Namespace: PreviewCacheDirectory;

    fn versioned_root(&self, parent: &Path) -> PathBuf;
    fn used_bytes(&self, root: &Path) -> Result<u64, ScanError>;
    fn algorithm(&self) -> PreviewCacheAlgorithm;
    fn current_artifact_key<'path>(&self, path: &'path Path) -> Option<&'path str>;
    fn managed_entry_kind(&self, path: &Path) -> Option<PreviewCacheEntryKind>;
    fn prepare_root(&self, root: &Path) -> Result<PreviewRootPreparation, ScanError>;
    fn open_cleanup_namespace(&self, root: &Path) -> Result<Self::Namespace, ScanError>;
    fn observe_existing_namespace(&self, root: &Path) -> Result<Self::Namespace, ScanError>;
}

/// Access is borrowed from the currently admitted store, including after a capacity handoff.
pub(crate) trait PreviewCacheAccounting {
    fn used_bytes(&self) -> u64;
    fn release(&self, preview_size: u64);
}
