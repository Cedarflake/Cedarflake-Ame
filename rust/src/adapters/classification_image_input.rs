use std::path::Path;
use std::sync::atomic::AtomicBool;

use crate::domain::classification::ClassificationContent;
use crate::domain::exact_duplicates::{
    EXACT_FINGERPRINT_SCHEME, FingerprintCandidate, MAX_FINGERPRINT_SOURCE_BYTES,
};
use crate::domain::{ExpectedFileState, ScanError};
use crate::ports::classification_analysis::{ClassificationImageLease, ClassificationImageReader};

use super::{PreviewPublicationGuard, open_viewer_source_guard, revalidate_open_preview_source};

mod pixels;

pub(crate) struct GuardedClassificationImageReader;

pub(crate) struct GuardedClassificationImage {
    guard: PreviewPublicationGuard,
    expected: ExpectedFileState,
}

impl ClassificationImageReader for GuardedClassificationImageReader {
    type Lease = GuardedClassificationImage;

    fn open(
        &self,
        candidate: &FingerprintCandidate,
        content: &ClassificationContent,
        cancelled: &AtomicBool,
    ) -> Result<Self::Lease, ScanError> {
        pixels::check_cancelled(cancelled)?;
        let location = &candidate.location;
        if content.scheme.as_str() != EXACT_FINGERPRINT_SCHEME
            || content.byte_length != location.file_size
            || location.file_size == 0
            || location.file_size > MAX_FINGERPRINT_SOURCE_BYTES
        {
            return Err(ScanError::new(
                "classification_source_rejected",
                "The classification source has no compatible bounded content evidence",
            ));
        }
        let root = &candidate.root;
        let root_identity = root.publication_root_identity.as_ref().ok_or_else(|| {
            ScanError::new(
                "classification_root_unproven",
                "The source root has no publication identity",
            )
        })?;
        let expected = ExpectedFileState {
            absolute_path: location.absolute_path.clone(),
            file_size: location.file_size,
            modified_unix_ms: location.modified_unix_ms,
            file_identity: location.file_identity.clone(),
            source_revision: location.source_revision.clone(),
        };
        let (_, guard) = open_viewer_source_guard(
            &expected,
            Path::new(&root.root_path),
            Path::new(&location.relative_path),
            root_identity,
        )
        .map_err(|issue| ScanError::new(issue.code, issue.message))?;
        // Without a durable revision, metadata cannot prove that a cached fingerprint is current.
        if expected.source_revision.is_none() || expected.file_identity.is_none() {
            let fingerprint = super::exact_content_reader::hash_reader(
                guard.source_file(),
                location.file_size,
                cancelled,
            )?
            .ok_or_else(pixels::cancelled_error)?;
            let digest = blake3::Hash::from_bytes(content.digest)
                .to_hex()
                .to_string();
            if fingerprint.digest != digest {
                return Err(ScanError::new(
                    "classification_source_changed",
                    "The source content changed before analysis",
                ));
            }
        }
        pixels::check_cancelled(cancelled)?;
        Ok(GuardedClassificationImage { guard, expected })
    }
}

impl ClassificationImageLease for GuardedClassificationImage {
    fn prepare_pixels(&mut self, cancelled: &AtomicBool) -> Result<Vec<f32>, ScanError> {
        pixels::prepare(self.guard.source_file(), cancelled)
    }

    fn revalidate(&self) -> Result<(), ScanError> {
        revalidate_open_preview_source(self.guard.source_file(), &self.expected)
            .map(|_| ())
            .map_err(|issue| ScanError::new(issue.code, issue.message))
    }
}

#[cfg(test)]
mod tests;
