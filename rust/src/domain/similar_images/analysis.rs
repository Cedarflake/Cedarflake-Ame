use super::{ScanError, SimilarContentKey};
use crate::domain::exact_duplicates::FingerprintCandidate;

pub(crate) const SIMILARITY_SOURCE_PAGE_SIZE: usize = 32;

#[derive(Clone, Debug)]
pub(crate) struct SimilarAnalysisRun {
    pub run_id: String,
    pub source_run_id: String,
    pub started_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimilarAnalysisPhase {
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug)]
pub struct SimilarAnalysisProgress {
    pub run_id: String,
    pub phase: SimilarAnalysisPhase,
    pub checked_sources: u64,
    pub analyzed_contents: u64,
    pub skipped_sources: u64,
    pub current_path: Option<String>,
    pub issue_code: Option<String>,
}

impl SimilarAnalysisProgress {
    pub(crate) fn running(run_id: String) -> Self {
        Self {
            run_id,
            phase: SimilarAnalysisPhase::Running,
            checked_sources: 0,
            analyzed_contents: 0,
            skipped_sources: 0,
            current_path: None,
            issue_code: None,
        }
    }
}

pub(crate) struct SimilarSource {
    pub candidate: FingerprintCandidate,
    pub content: Option<SimilarContentKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SimilarVariant {
    pub candidate: [u16; 4],
    pub confirmation: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimilarEvidence {
    Ready { variants: Vec<SimilarVariant> },
    InsufficientStructure,
    DecodeFailed { issue_code: String },
}

impl SimilarEvidence {
    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        let valid = match self {
            Self::Ready { variants } => {
                (1..=8).contains(&variants.len())
                    && variants
                        .iter()
                        .any(|value| value.candidate != [0; 4] && value.candidate != [u16::MAX; 4])
                    && variants
                        .iter()
                        .enumerate()
                        .all(|(index, value)| !variants[..index].contains(value))
            }
            Self::InsufficientStructure => true,
            Self::DecodeFailed { issue_code } => !issue_code.is_empty() && issue_code.len() <= 128,
        };
        if valid {
            Ok(())
        } else {
            Err(ScanError::new(
                "similarity_evidence_invalid",
                "Similarity evidence must contain bounded distinct variants or a terminal disposition",
            ))
        }
    }
}
