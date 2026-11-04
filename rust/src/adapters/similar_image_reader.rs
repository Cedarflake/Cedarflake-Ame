use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::FingerprintCandidate;
use crate::domain::similar_images::{SimilarContentKey, SimilarEvidence};
use crate::ports::similar_images::{SimilarImageLease, SimilarImageReader};

#[cfg(windows)]
use super::analysis_source::{AnalysisContent, AnalysisSourceFailure, GuardedAnalysisSource};

#[cfg(any(windows, test))]
mod input;
#[cfg(any(windows, test))]
mod pixels;
#[cfg(any(windows, test))]
mod signature;

pub(crate) struct GuardedSimilarImageReader;

pub(crate) struct GuardedSimilarImage {
    evidence: SimilarEvidence,
    #[cfg(windows)]
    source: GuardedAnalysisSource,
}

impl SimilarImageLease for GuardedSimilarImage {
    fn evidence(&self) -> &SimilarEvidence {
        &self.evidence
    }

    fn revalidate(&self) -> Result<(), ScanError> {
        #[cfg(windows)]
        {
            self.source.revalidate()
        }
        #[cfg(not(windows))]
        {
            Err(unsupported())
        }
    }
}

impl SimilarImageReader for GuardedSimilarImageReader {
    type Lease = GuardedSimilarImage;

    fn analyze(
        &self,
        candidate: &FingerprintCandidate,
        content: &SimilarContentKey,
        cancelled: &AtomicBool,
    ) -> Result<Option<Self::Lease>, ScanError> {
        if cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        content.validate()?;
        #[cfg(not(windows))]
        {
            let _ = candidate;
            Err(unsupported())
        }
        #[cfg(windows)]
        {
            let digest = blake3::Hash::from_hex(&content.digest).map_err(|_| {
                ScanError::new(
                    "similarity_identity_invalid",
                    "Similarity requires a canonical content digest",
                )
            })?;
            let source = match GuardedAnalysisSource::open(
                candidate,
                &AnalysisContent {
                    scheme: &content.scheme,
                    digest: *digest.as_bytes(),
                    byte_length: content.byte_length,
                },
                cancelled,
            ) {
                Ok(source) => source,
                Err(AnalysisSourceFailure::Cancelled) => return Ok(None),
                Err(error) => return Err(source_failure(error)),
            };
            let Some(evidence) = analyze_reader(&mut source.reader(), cancelled)? else {
                return Ok(None);
            };
            source.revalidate()?;
            Ok(Some(GuardedSimilarImage { evidence, source }))
        }
    }
}

#[cfg(any(windows, test))]
fn analyze_reader(
    source: &mut (impl std::io::Read + std::io::Seek),
    cancelled: &AtomicBool,
) -> Result<Option<SimilarEvidence>, ScanError> {
    let result = (|| {
        let image = input::decode(source, cancelled)?;
        let sample = pixels::prepare(image, cancelled)?;
        signature::analyze(&sample, cancelled)
    })();
    match result {
        Ok(evidence) => {
            evidence.validate()?;
            Ok(Some(evidence))
        }
        Err(input::InputFailure::Cancelled) => Ok(None),
        Err(input::InputFailure::Read) => Err(ScanError::new(
            "similarity_source_read_failed",
            "The image could not be read for similarity analysis",
        )),
        Err(input::InputFailure::Limit) => Ok(Some(SimilarEvidence::DecodeFailed {
            issue_code: "similarity_image_limit".into(),
        })),
        Err(input::InputFailure::Decode) => Ok(Some(SimilarEvidence::DecodeFailed {
            issue_code: "similarity_decode_failed".into(),
        })),
    }
}

#[cfg(windows)]
fn source_failure(error: AnalysisSourceFailure) -> ScanError {
    match error {
        AnalysisSourceFailure::Source(error) => error,
        AnalysisSourceFailure::Cancelled => {
            ScanError::new("similarity_cancelled", "Similarity analysis was cancelled")
        }
        AnalysisSourceFailure::InvalidContent => ScanError::new(
            "similarity_source_rejected",
            "The image has no compatible bounded content evidence",
        ),
        AnalysisSourceFailure::UnprovenRoot => ScanError::new(
            "similarity_root_unproven",
            "The source root has no publication identity",
        ),
        AnalysisSourceFailure::Changed => ScanError::new(
            "similarity_source_changed",
            "The image changed before similarity analysis",
        ),
    }
}

#[cfg(not(windows))]
fn unsupported() -> ScanError {
    ScanError::new(
        "similarity_source_platform_unsupported",
        "Guarded similarity analysis requires Windows",
    )
}

#[cfg(test)]
mod tests;
