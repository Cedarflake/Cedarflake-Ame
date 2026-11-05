use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::ScanError;
use crate::domain::similar_images::{
    SimilarAnalysisPhase, SimilarAnalysisProgress, SimilarAnalysisRun,
};

mod execution;
mod production;
pub(crate) mod review;

static ACTIVE: Mutex<Option<String>> = Mutex::new(None);

#[derive(Default)]
struct AnalysisControl {
    cancelled: AtomicBool,
    suspended: AtomicBool,
}

pub(crate) struct SimilarImageAnalysis {
    pending: Mutex<Option<String>>,
    control: AnalysisControl,
    progress: Mutex<SimilarAnalysisProgress>,
}

struct Admission(String);

impl Drop for Admission {
    fn drop(&mut self) {
        if let Ok(mut active) = ACTIVE.lock()
            && active.as_ref() == Some(&self.0)
        {
            *active = None;
        }
    }
}

pub(crate) fn open(run_id: String) -> Result<SimilarImageAnalysis, ScanError> {
    if run_id.is_empty() || run_id.len() > 96 || run_id.contains('\0') {
        return Err(ScanError::new(
            "similarity_run_invalid",
            "Similarity analysis requires a bounded run identifier",
        ));
    }
    Ok(SimilarImageAnalysis {
        progress: Mutex::new(SimilarAnalysisProgress::running(run_id.clone())),
        pending: Mutex::new(Some(run_id)),
        control: AnalysisControl::default(),
    })
}

impl SimilarImageAnalysis {
    pub(crate) fn run(&self) -> Result<SimilarAnalysisProgress, ScanError> {
        let run_id = self
            .pending
            .lock()
            .map_err(|_| unavailable())?
            .take()
            .ok_or_else(|| {
                ScanError::new(
                    "similarity_already_started",
                    "This similarity session has already run",
                )
            })?;
        let result = self.execute(run_id);
        if let Err(issue) = &result {
            let mut progress = self.progress.lock().map_err(|_| unavailable())?;
            progress.phase = SimilarAnalysisPhase::Failed;
            progress.current_path = None;
            progress.issue_code = Some(issue.code.clone());
        }
        result
    }

    fn execute(&self, run_id: String) -> Result<SimilarAnalysisProgress, ScanError> {
        if self.control.cancelled.load(Ordering::Acquire) {
            let mut progress = self.progress.lock().map_err(|_| unavailable())?;
            progress.phase = SimilarAnalysisPhase::Cancelled;
            return Ok(progress.clone());
        }
        let _admission = {
            let mut active = ACTIVE.lock().map_err(|_| unavailable())?;
            if active.is_some() {
                return Err(ScanError::new(
                    "similarity_analysis_busy",
                    "A similarity analysis is already running",
                ));
            }
            *active = Some(run_id.clone());
            Admission(run_id.clone())
        };
        let run = SimilarAnalysisRun {
            source_run_id: format!("similarity-source:{run_id}"),
            run_id,
            started_unix_ms: i64::try_from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| unavailable())?
                    .as_millis(),
            )
            .map_err(|_| unavailable())?,
        };
        let mut repository = production::repository()?;
        execution::analyze(
            &mut repository,
            &production::fingerprint_reader(),
            &production::image_reader(),
            &run,
            &self.control,
            &mut |progress| {
                if let Ok(mut stored) = self.progress.lock() {
                    *stored = progress;
                } else {
                    self.control.cancelled.store(true, Ordering::Release);
                }
            },
        )
    }

    pub(crate) fn progress(&self) -> Result<SimilarAnalysisProgress, ScanError> {
        self.progress
            .lock()
            .map(|value| value.clone())
            .map_err(|_| unavailable())
    }

    pub(crate) fn cancel(&self) {
        self.control.cancelled.store(true, Ordering::Release);
    }
    pub(crate) fn suspend(&self, suspended: bool) {
        self.control.suspended.store(suspended, Ordering::Release);
    }
}

fn unavailable() -> ScanError {
    ScanError::new(
        "similarity_analysis_unavailable",
        "Similarity analysis is temporarily unavailable",
    )
}

#[cfg(test)]
mod tests;
