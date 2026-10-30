use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::work::{
    ClassificationVerificationResult, ClassificationWorkLease,
};
use crate::ports::classification_source::{
    ClassificationSourcePreparation, ClassificationSourceRepository,
};
use crate::ports::exact_duplicates::{ExactContentReader, FingerprintReadLease};

pub fn verify_classification_source(
    database_path: &std::path::Path,
    lease: &ClassificationWorkLease,
    now_unix_ms: u64,
    cancelled: &AtomicBool,
) -> Result<ClassificationVerificationResult, ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationVerificationResult::Cancelled);
    }
    let mut catalog = super::catalog_session::open_catalog(
        database_path,
        crate::domain::LibraryChangeLane::Recovery,
    )?;
    verify(
        &mut catalog,
        &crate::adapters::GuardedExactContentReader,
        lease,
        now_unix_ms,
        cancelled,
    )
}

pub(crate) fn verify(
    repository: &mut impl ClassificationSourceRepository,
    reader: &impl ExactContentReader,
    work: &ClassificationWorkLease,
    now_unix_ms: u64,
    cancelled: &AtomicBool,
) -> Result<ClassificationVerificationResult, ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationVerificationResult::Cancelled);
    }
    let prepared = repository.prepare_classification_source(work, cancelled)?;
    if matches!(prepared, ClassificationSourcePreparation::Completed) {
        return Ok(ClassificationVerificationResult::Completed);
    }
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationVerificationResult::Cancelled);
    }
    let candidate = match prepared {
        ClassificationSourcePreparation::Completed => {
            return Ok(ClassificationVerificationResult::Completed);
        }
        ClassificationSourcePreparation::Superseded => {
            return Ok(ClassificationVerificationResult::Superseded);
        }
        ClassificationSourcePreparation::Read(candidate) => candidate,
    };
    let Some(source) = reader.read_fingerprint(&candidate, cancelled)? else {
        return Ok(ClassificationVerificationResult::Cancelled);
    };
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationVerificationResult::Cancelled);
    }
    source.revalidate()?;
    if cancelled.load(Ordering::Acquire) {
        return Ok(ClassificationVerificationResult::Cancelled);
    }
    let committed = repository.commit_classification_source(
        work,
        &candidate,
        source.fingerprint(),
        now_unix_ms,
        cancelled,
    )?;
    // Retain the source and namespace handles through the commit, including its error path.
    drop(source);
    if committed {
        Ok(ClassificationVerificationResult::Completed)
    } else if cancelled.load(Ordering::Acquire) {
        Ok(ClassificationVerificationResult::Cancelled)
    } else {
        Ok(ClassificationVerificationResult::Superseded)
    }
}
