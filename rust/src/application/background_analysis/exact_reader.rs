use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{ContentFingerprint, FingerprintCandidate};
use crate::ports::exact_duplicates::{ExactContentReader, FingerprintReadLease};

use super::{BackgroundAnalysisGate, BackgroundAnalysisLease};

pub(crate) struct AdmittedExactContentReader<'a, R> {
    reader: R,
    gate: &'a BackgroundAnalysisGate,
}

impl<'a, R> AdmittedExactContentReader<'a, R> {
    pub(crate) fn new(reader: R, gate: &'a BackgroundAnalysisGate) -> Self {
        Self { reader, gate }
    }
}

pub(crate) struct AdmittedFingerprint<'a, L> {
    source: L,
    // Field drop order releases the source before admitting the next background task.
    _admission: BackgroundAnalysisLease<'a>,
}

impl<'a, R: ExactContentReader> ExactContentReader for AdmittedExactContentReader<'a, R> {
    type Lease = AdmittedFingerprint<'a, R::Lease>;

    fn read_fingerprint(
        &self,
        candidate: &FingerprintCandidate,
        cancelled: &AtomicBool,
    ) -> Result<Option<Self::Lease>, ScanError> {
        let Some(admission) = self.gate.acquire(cancelled)? else {
            return Ok(None);
        };
        Ok(self
            .reader
            .read_fingerprint(candidate, cancelled)?
            .map(|source| AdmittedFingerprint {
                source,
                _admission: admission,
            }))
    }
}

impl<L: FingerprintReadLease> FingerprintReadLease for AdmittedFingerprint<'_, L> {
    fn fingerprint(&self) -> &ContentFingerprint {
        self.source.fingerprint()
    }

    fn revalidate(&self) -> Result<(), ScanError> {
        self.source.revalidate()
    }
}

#[cfg(test)]
mod tests;
