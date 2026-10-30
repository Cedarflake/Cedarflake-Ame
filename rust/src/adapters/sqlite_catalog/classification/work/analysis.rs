use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::classification::PredictionEvidence;
use crate::domain::classification::work::{ClassificationWorkLease, ClassificationWorkTarget};
use crate::ports::classification_analysis::{
    ClassificationAnalysisRepository, ClassificationAnalysisSource,
};

use super::super::super::{SqliteCatalog, database_error, exact_duplicates};
use super::super::predictions;
use super::{completion, invalid, leases, queue};

impl ClassificationAnalysisRepository for SqliteCatalog {
    fn prepare_classification_analysis(
        &mut self,
        lease: &ClassificationWorkLease,
        cancelled: &AtomicBool,
    ) -> Result<Option<ClassificationAnalysisSource>, ScanError> {
        let transaction = self.begin_write()?;
        if cancelled.load(Ordering::Acquire) || !current(&transaction, lease)? {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        }
        let ClassificationWorkTarget::AnalyzeContent { source, .. } = &lease.target else {
            return Err(invalid());
        };
        let candidate =
            exact_duplicates::queries::load_selected(&transaction, source)?.ok_or_else(invalid)?;
        let run = predictions::load_run(&transaction, lease.run_id.as_str())?;
        transaction.commit().map_err(database_error)?;
        Ok(Some(ClassificationAnalysisSource { candidate, run }))
    }

    fn classification_analysis_is_current(
        &mut self,
        lease: &ClassificationWorkLease,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        let valid = !cancelled.load(Ordering::Acquire) && current(&transaction, lease)?;
        transaction.commit().map_err(database_error)?;
        Ok(valid)
    }

    fn commit_classification_analysis(
        &mut self,
        lease: &ClassificationWorkLease,
        evidence: &PredictionEvidence,
        cancelled: &AtomicBool,
    ) -> Result<bool, ScanError> {
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        let transaction = self.begin_write()?;
        let published = completion::publish(&transaction, lease, evidence)?;
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        transaction.commit().map_err(database_error)?;
        Ok(published)
    }
}

fn current(connection: &Connection, lease: &ClassificationWorkLease) -> Result<bool, ScanError> {
    if leases::current(connection, lease)?.is_none() {
        return Ok(false);
    }
    let ClassificationWorkTarget::AnalyzeContent { source, content } = &lease.target else {
        return Err(invalid());
    };
    if queue::verified_content(connection, source)?.as_ref() == Some(content) {
        return Ok(true);
    }
    let phase = if queue::content_source(connection, content)?.is_some() {
        "queued"
    } else {
        "retired"
    };
    leases::checkpoint(connection, lease.work_id, phase)?;
    Ok(false)
}
