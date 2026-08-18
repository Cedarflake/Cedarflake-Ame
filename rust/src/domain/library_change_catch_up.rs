use super::{LibraryChangeObservation, LibraryRootGeneration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeCatchUpCheckpoint {
    pub volume_id: String,
    pub journal_id: String,
    pub next_usn: String,
    pub root_set_fingerprint: String,
    pub catalog_revision: u64,
    pub updated_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeCatchUpEvidence {
    pub source: String,
    pub watermark: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeCatchUpRootResult {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub observations: Vec<LibraryChangeObservation>,
    pub fallback_code: Option<String>,
    pub evidence: Option<LibraryChangeCatchUpEvidence>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryChangeCatchUpBatch {
    pub roots: Vec<LibraryChangeCatchUpRootResult>,
    pub checkpoints: Vec<LibraryChangeCatchUpCheckpoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LibraryChangeCatchUpLimits {
    pub max_records_per_volume: usize,
    pub max_evidence_bytes_per_volume: usize,
    pub max_observations_per_root: usize,
}

impl LibraryChangeCatchUpLimits {
    pub const MAX_RECORDS_PER_VOLUME: usize = 65_536;
    pub const MAX_EVIDENCE_BYTES_PER_VOLUME: usize = 64 * 1_024 * 1_024;
    pub const MAX_OBSERVATIONS_PER_ROOT: usize = 4_096;

    pub const fn is_valid(self) -> bool {
        self.max_records_per_volume > 0
            && self.max_records_per_volume <= Self::MAX_RECORDS_PER_VOLUME
            && self.max_evidence_bytes_per_volume > 0
            && self.max_evidence_bytes_per_volume <= Self::MAX_EVIDENCE_BYTES_PER_VOLUME
            && self.max_observations_per_root > 0
            && self.max_observations_per_root <= Self::MAX_OBSERVATIONS_PER_ROOT
    }
}

impl Default for LibraryChangeCatchUpLimits {
    fn default() -> Self {
        Self {
            max_records_per_volume: Self::MAX_RECORDS_PER_VOLUME,
            max_evidence_bytes_per_volume: Self::MAX_EVIDENCE_BYTES_PER_VOLUME,
            max_observations_per_root: Self::MAX_OBSERVATIONS_PER_ROOT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeCatchUpCompletedRoot {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub fallback_code: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryChangeCatchUpReport {
    pub completed_roots: Vec<LibraryChangeCatchUpCompletedRoot>,
    pub observation_count: u64,
    pub fallback_count: u32,
    pub checkpoint_count: u32,
}
