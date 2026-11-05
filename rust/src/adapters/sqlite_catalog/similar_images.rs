use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{
    ContentFingerprint, EXACT_FINGERPRINT_SCHEME, FingerprintCandidate,
};
use crate::domain::similar_images::{
    SimilarAnalysisProgress, SimilarAnalysisRun, SimilarContentKey, SimilarEvidence, SimilarSource,
};
use crate::ports::similar_images::SimilarAnalysisRepository;

use super::{SqliteCatalog, database_error};

mod evidence;
mod matching;
mod review;
mod runs;
mod sources;

impl SimilarAnalysisRepository for SqliteCatalog {
    fn begin_similarity(&mut self, run: &SimilarAnalysisRun) -> Result<(), ScanError> {
        let transaction = self.begin_write()?;
        runs::begin(&transaction, run)?;
        transaction.commit().map_err(database_error)
    }

    fn renew_similarity(&mut self, run: &SimilarAnalysisRun) -> Result<(), ScanError> {
        let transaction = self.begin_write()?;
        runs::renew(&transaction, run)?;
        transaction.commit().map_err(database_error)
    }

    fn load_similarity_sources(
        &self,
        after_location_id: &str,
    ) -> Result<Vec<SimilarSource>, ScanError> {
        sources::load(&self.connection, after_location_id)
    }

    fn has_similarity_evidence(&self, content: &SimilarContentKey) -> Result<bool, ScanError> {
        evidence::contains(&self.connection, content)
    }

    fn verify_similarity_source(
        &mut self,
        run: &SimilarAnalysisRun,
        candidate: &FingerprintCandidate,
        fingerprint: &ContentFingerprint,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        runs::require_owner(&transaction, run)?;
        if cancelled.load(Ordering::Acquire)
            || !sources::verify(&transaction, run, candidate, fingerprint)?
        {
            return Ok(false);
        }
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }

    fn publish_similarity(
        &mut self,
        run: &SimilarAnalysisRun,
        candidate: &FingerprintCandidate,
        content: &SimilarContentKey,
        value: &SimilarEvidence,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        content.validate()?;
        value.validate()?;
        if content.scheme != EXACT_FINGERPRINT_SCHEME {
            return Err(ScanError::new(
                "similarity_source_rejected",
                "Similarity publication requires the verified fingerprint scheme",
            ));
        }
        let transaction = self.begin_write()?;
        runs::require_owner(&transaction, run)?;
        let fingerprint = ContentFingerprint {
            digest: content.digest.clone(),
            byte_length: content.byte_length,
        };
        if cancelled.load(Ordering::Acquire)
            || !sources::verify(&transaction, run, candidate, &fingerprint)?
        {
            return Ok(false);
        }
        let inserted = evidence::publish(&transaction, run, content, value)?;
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        transaction.commit().map_err(database_error)?;
        Ok(inserted)
    }

    fn record_similarity_failure(
        &mut self,
        run: &SimilarAnalysisRun,
        candidate: &FingerprintCandidate,
        issue_code: &str,
    ) -> Result<(), ScanError> {
        let transaction = self.begin_write()?;
        runs::require_owner(&transaction, run)?;
        sources::record_failure(&transaction, candidate, issue_code)?;
        transaction.commit().map_err(database_error)
    }

    fn match_similarity_window(
        &mut self,
        run: &SimilarAnalysisRun,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        runs::require_owner(&transaction, run)?;
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        let worked = matching::advance(&transaction, run, cancelled)?;
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        transaction.commit().map_err(database_error)?;
        Ok(worked)
    }

    fn finish_similarity(
        &mut self,
        run: &SimilarAnalysisRun,
        progress: &SimilarAnalysisProgress,
    ) -> Result<(), ScanError> {
        let transaction = self.begin_write()?;
        runs::finish(&transaction, run, progress)?;
        transaction.commit().map_err(database_error)
    }
}

#[cfg(test)]
mod tests;
