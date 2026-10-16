use super::{AssetLocationView, IncrementalCatalogRoot};

pub(crate) const EXACT_FINGERPRINT_SCHEME: &str = "blake3-256-full-v1";
pub(crate) const EXACT_FINGERPRINT_ENGINE: &str = "blake3";
pub(crate) const EXACT_FINGERPRINT_VERSION: &str = "1.8.6";
pub(crate) const EXACT_FINGERPRINT_PARAMETERS: &str = "full-file;output=256";
pub(crate) const MAX_FINGERPRINT_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const EXACT_DUPLICATE_PAGE_SIZE: usize = 64;

#[derive(Clone, Debug)]
pub(crate) struct FingerprintCandidate {
    pub location: AssetLocationView,
    pub root: IncrementalCatalogRoot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContentFingerprint {
    pub digest: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct ExactAnalysisRun {
    pub run_id: String,
    pub started_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactAnalysisPhase {
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug)]
pub struct ExactAnalysisProgress {
    pub run_id: String,
    pub phase: ExactAnalysisPhase,
    pub checked_files: u64,
    pub fingerprinted_files: u64,
    pub skipped_files: u64,
    pub bytes_read: u64,
    pub current_path: Option<String>,
    pub issue_code: Option<String>,
}

impl ExactAnalysisProgress {
    pub(crate) fn running(run_id: String) -> Self {
        Self {
            run_id,
            phase: ExactAnalysisPhase::Running,
            checked_files: 0,
            fingerprinted_files: 0,
            skipped_files: 0,
            bytes_read: 0,
            current_path: None,
            issue_code: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateGroup {
    pub digest: String,
    pub byte_length: u64,
    pub location_count: u64,
    pub physical_file_count: u64,
    pub representative_location_id: String,
    pub representative_path: String,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateGroupPage {
    pub groups: Vec<ExactDuplicateGroup>,
    pub next: Option<ExactDuplicateGroupCursor>,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateGroupCursor {
    pub digest: String,
    pub byte_length: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuplicateLocationAvailability {
    Local,
    CloudOnly,
    Offline,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateLocation {
    pub asset_id: String,
    pub location_id: String,
    pub root_id: String,
    pub display_path: String,
    pub source_generation: u64,
    pub availability: DuplicateLocationAvailability,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateLocationPage {
    pub locations: Vec<ExactDuplicateLocation>,
    pub next_location_id: Option<String>,
}
