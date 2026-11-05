use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::exact_duplicates::{
    EXACT_FINGERPRINT_SCHEME, FingerprintCandidate, MAX_FINGERPRINT_SOURCE_BYTES,
};
use crate::domain::{ExpectedFileState, ScanError};

use super::{PreviewPublicationGuard, open_viewer_source_guard, revalidate_open_preview_source};

pub(super) struct AnalysisContent<'a> {
    pub scheme: &'a str,
    pub digest: [u8; 32],
    pub byte_length: u64,
}

pub(super) enum AnalysisSourceFailure {
    Cancelled,
    InvalidContent,
    UnprovenRoot,
    Changed,
    Source(ScanError),
}

pub(super) struct GuardedAnalysisSource {
    guard: PreviewPublicationGuard,
    expected: ExpectedFileState,
}

impl GuardedAnalysisSource {
    pub(super) fn open(
        candidate: &FingerprintCandidate,
        content: &AnalysisContent<'_>,
        cancelled: &AtomicBool,
    ) -> Result<Self, AnalysisSourceFailure> {
        check_cancelled(cancelled)?;
        let location = &candidate.location;
        if content.scheme != EXACT_FINGERPRINT_SCHEME
            || content.byte_length != location.file_size
            || location.file_size == 0
            || location.file_size > MAX_FINGERPRINT_SOURCE_BYTES
        {
            return Err(AnalysisSourceFailure::InvalidContent);
        }
        let root = &candidate.root;
        let root_identity = root
            .publication_root_identity
            .as_ref()
            .ok_or(AnalysisSourceFailure::UnprovenRoot)?;
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
        .map_err(|issue| {
            AnalysisSourceFailure::Source(ScanError::new(issue.code, issue.message))
        })?;
        // Without a durable revision, metadata cannot prove that cached content is current.
        if expected.source_revision.is_none() || expected.file_identity.is_none() {
            let fingerprint = super::exact_content_reader::hash_reader(
                guard.source_file(),
                location.file_size,
                cancelled,
            )
            .map_err(AnalysisSourceFailure::Source)?
            .ok_or(AnalysisSourceFailure::Cancelled)?;
            if fingerprint.digest != blake3::Hash::from_bytes(content.digest).to_hex().as_str() {
                return Err(AnalysisSourceFailure::Changed);
            }
        }
        check_cancelled(cancelled)?;
        Ok(Self { guard, expected })
    }

    pub(super) fn reader(&self) -> &File {
        self.guard.source_file()
    }

    pub(super) fn revalidate(&self) -> Result<(), ScanError> {
        revalidate_open_preview_source(self.guard.source_file(), &self.expected)
            .map(|_| ())
            .map_err(|issue| ScanError::new(issue.code, issue.message))
    }
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), AnalysisSourceFailure> {
    if cancelled.load(Ordering::Acquire) {
        Err(AnalysisSourceFailure::Cancelled)
    } else {
        Ok(())
    }
}
