use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::ClassificationContent;
use crate::domain::exact_duplicates::FingerprintCandidate;
use crate::ports::classification_analysis::{ClassificationImageLease, ClassificationImageReader};

use super::analysis_source::{AnalysisContent, AnalysisSourceFailure, GuardedAnalysisSource};

mod pixels;

pub(crate) struct GuardedClassificationImageReader;

pub(crate) struct GuardedClassificationImage {
    source: GuardedAnalysisSource,
}

impl ClassificationImageReader for GuardedClassificationImageReader {
    type Lease = GuardedClassificationImage;

    fn open(
        &self,
        candidate: &FingerprintCandidate,
        content: &ClassificationContent,
        cancelled: &AtomicBool,
    ) -> Result<Self::Lease, ScanError> {
        let source = GuardedAnalysisSource::open(
            candidate,
            &AnalysisContent {
                scheme: content.scheme.as_str(),
                digest: content.digest,
                byte_length: content.byte_length,
            },
            cancelled,
        )
        .map_err(source_failure)?;
        Ok(GuardedClassificationImage { source })
    }
}

impl ClassificationImageLease for GuardedClassificationImage {
    fn prepare_pixels(&mut self, cancelled: &AtomicBool) -> Result<Vec<f32>, ScanError> {
        pixels::prepare(self.source.reader(), cancelled)
    }

    fn revalidate(&self) -> Result<(), ScanError> {
        self.source.revalidate()
    }
}

fn source_failure(error: AnalysisSourceFailure) -> ScanError {
    match error {
        AnalysisSourceFailure::Cancelled => pixels::cancelled_error(),
        AnalysisSourceFailure::InvalidContent => ScanError::new(
            "classification_source_rejected",
            "The classification source has no compatible bounded content evidence",
        ),
        AnalysisSourceFailure::UnprovenRoot => ScanError::new(
            "classification_root_unproven",
            "The source root has no publication identity",
        ),
        AnalysisSourceFailure::Changed => ScanError::new(
            "classification_source_changed",
            "The source content changed before analysis",
        ),
        AnalysisSourceFailure::Source(error) => error,
    }
}

#[cfg(test)]
mod tests;
