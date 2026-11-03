use std::path::{Path, PathBuf};

use crate::domain::ScanError;

pub(crate) trait PreviewCacheStorage {
    fn versioned_root(&self, parent: &Path) -> PathBuf;
    fn used_bytes(&self, root: &Path) -> Result<u64, ScanError>;
}
