use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::exact_duplicates::{
    EXACT_FINGERPRINT_SCHEME, ExactAnalysisRun, FingerprintCandidate,
};
use crate::domain::semantic_discovery::ImageContentIdentity;
use crate::domain::{ExactAnalysisPhase, ExactAnalysisProgress, ScanError};
use crate::ports::exact_duplicates::{
    ExactContentReader, ExactDuplicateRepository, FingerprintReadLease,
};

use super::{changed, check_cancelled};

#[derive(Default)]
pub(crate) struct SemanticVerification {
    progress: Option<ExactAnalysisProgress>,
}

impl SemanticVerification {
    pub(crate) fn read(
        &mut self,
        catalog: &mut impl ExactDuplicateRepository,
        reader: &impl ExactContentReader,
        candidate: &FingerprintCandidate,
        cancelled: &AtomicBool,
    ) -> Result<ImageContentIdentity, ScanError> {
        if self.progress.is_none() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            let run = ExactAnalysisRun {
                run_id: format!("discovery-{}-{}", std::process::id(), now.as_nanos()),
                started_unix_ms: now.as_millis().min(i64::MAX as u128) as i64,
            };
            catalog.begin_exact_analysis(&run)?;
            self.progress = Some(ExactAnalysisProgress::running(run.run_id));
        }
        let progress = self
            .progress
            .as_mut()
            .expect("the verification run is admitted");
        progress.checked_files += 1;
        let result = (|| {
            let lease = reader
                .read_fingerprint(candidate, cancelled)?
                .ok_or_else(changed)?;
            check_cancelled(cancelled)?;
            lease.revalidate()?;
            let fingerprint = lease.fingerprint();
            progress.bytes_read += fingerprint.byte_length;
            if !catalog.publish_fingerprint(&progress.run_id, candidate, fingerprint, cancelled)? {
                return Err(changed());
            }
            let digest = blake3::Hash::from_hex(&fingerprint.digest).map_err(|_| changed())?;
            let content = ImageContentIdentity::new(
                EXACT_FINGERPRINT_SCHEME.to_owned(),
                *digest.as_bytes(),
                fingerprint.byte_length,
            )?;
            catalog.publish_exact_analysis_batch(
                &progress.run_id,
                &[candidate.location.source_generation],
            )?;
            progress.fingerprinted_files += 1;
            drop(lease);
            Ok(content)
        })();
        if let Err(error) = &result {
            progress.skipped_files += 1;
            progress.issue_code = Some(error.code.clone());
            if cancelled.load(Ordering::Acquire) {
                progress.phase = ExactAnalysisPhase::Cancelled;
            }
        }
        result
    }

    pub(crate) fn finish(
        &mut self,
        catalog: &mut impl ExactDuplicateRepository,
    ) -> Result<(), ScanError> {
        if let Some(progress) = &mut self.progress {
            if progress.phase == ExactAnalysisPhase::Running {
                progress.phase = ExactAnalysisPhase::Completed;
            }
            catalog.finish_exact_analysis(progress)?;
            self.progress = None;
        }
        Ok(())
    }
}
