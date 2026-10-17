#[cfg(any(windows, test))]
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{
    ContentFingerprint, FingerprintCandidate, MAX_FINGERPRINT_SOURCE_BYTES,
};
use crate::ports::exact_duplicates::{ExactContentReader, FingerprintReadLease};

pub(crate) struct GuardedExactContentReader;

pub(crate) struct GuardedFingerprint {
    fingerprint: ContentFingerprint,
    guard: super::PreviewPublicationGuard,
    expected: crate::domain::ExpectedFileState,
}

impl FingerprintReadLease for GuardedFingerprint {
    fn fingerprint(&self) -> &ContentFingerprint {
        &self.fingerprint
    }

    fn revalidate(&self) -> Result<(), ScanError> {
        super::revalidate_open_preview_source(self.guard.source_file(), &self.expected)
            .map(|_| ())
            .map_err(|issue| ScanError::new(issue.code, issue.message))
    }
}

impl ExactContentReader for GuardedExactContentReader {
    type Lease = GuardedFingerprint;

    fn read_fingerprint(
        &self,
        candidate: &FingerprintCandidate,
        cancelled: &AtomicBool,
    ) -> Result<Option<Self::Lease>, ScanError> {
        if cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        let location = &candidate.location;
        if location.file_size == 0 || location.file_size > MAX_FINGERPRINT_SOURCE_BYTES {
            return Err(ScanError::new(
                "exact_source_size_exceeded",
                "The encoded source is outside the bounded content-read size",
            ));
        }
        #[cfg(not(windows))]
        {
            Err(ScanError::new(
                "exact_source_platform_unsupported",
                "Guarded exact-content analysis requires Windows",
            ))
        }
        #[cfg(windows)]
        {
            let root = &candidate.root;
            let root_identity = root.publication_root_identity.as_ref().ok_or_else(|| {
                ScanError::new(
                    "exact_root_identity_unproven",
                    "The content source root has no publication identity",
                )
            })?;
            let expected = crate::domain::ExpectedFileState {
                absolute_path: location.absolute_path.clone(),
                file_size: location.file_size,
                modified_unix_ms: location.modified_unix_ms,
                file_identity: location.file_identity.clone(),
                source_revision: location.source_revision.clone(),
            };
            let (_, guard) = super::open_viewer_source_guard(
                &expected,
                std::path::Path::new(&root.root_path),
                std::path::Path::new(&location.relative_path),
                root_identity,
            )
            .map_err(|issue| ScanError::new(issue.code, issue.message))?;
            let Some(fingerprint) =
                hash_reader(guard.source_file(), location.file_size, cancelled)?
            else {
                return Ok(None);
            };
            let result = GuardedFingerprint {
                fingerprint,
                guard,
                expected,
            };
            result.revalidate()?;
            Ok(Some(result))
        }
    }
}

// The byte engine only receives an admitted reader; it cannot reopen a path or mutate a source.
#[cfg(any(windows, test))]
fn hash_reader(
    reader: impl Read,
    expected_length: u64,
    cancelled: &AtomicBool,
) -> Result<Option<ContentFingerprint>, ScanError> {
    let mut reader = reader.take(expected_length.saturating_add(1));
    let mut buffer = vec![0_u8; 256 * 1024];
    let mut hasher = blake3::Hasher::new();
    let mut byte_length = 0_u64;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        let count = reader
            .read(&mut buffer)
            .map_err(|error| ScanError::new("exact_source_read_failed", error.to_string()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        byte_length += count as u64;
    }
    if byte_length != expected_length {
        return Err(ScanError::new(
            "exact_source_changed",
            "The encoded source length changed during analysis",
        ));
    }
    if cancelled.load(Ordering::Acquire) {
        return Ok(None);
    }
    Ok(Some(ContentFingerprint {
        digest: hasher.finalize().to_hex().to_string(),
        byte_length,
    }))
}

#[cfg(test)]
mod tests;
