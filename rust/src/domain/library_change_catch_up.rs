#[cfg(test)]
use super::LibraryChangeIntent;

#[cfg(test)]
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

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryChangeCatchUpQueueBatch {
    pub intents: Vec<LibraryChangeIntent>,
    pub evidence: Option<LibraryChangeCatchUpEvidence>,
}
