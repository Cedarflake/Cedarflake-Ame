use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;

pub(crate) struct IndexedFileHint {
    pub relative_path: String,
    pub file_size: Option<u64>,
    pub modified_unix_ms: Option<i64>,
    pub is_image_candidate: bool,
}

pub(crate) trait SearchIndex {
    fn read(
        &self,
        root_path: &str,
        cancelled: &AtomicBool,
        accept: &mut dyn FnMut(&[IndexedFileHint]) -> Result<(), ScanError>,
    ) -> Result<u64, ScanError>;
}
