use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::adapters::classification_runtime::LocalClassificationEncoderFactory;
use crate::adapters::{SqliteCatalog, SqliteClassificationFeatureCache};
use crate::domain::classification::work::{
    ClassificationAnalysisResult, ClassificationVerificationResult, ClassificationWorkFailure,
    ClassificationWorkLease, ClassificationWorkTarget,
};
use crate::domain::classification::{ClassificationAnalysisSetup, ClassificationRecordId};
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_execution::{
    ClassificationExecutionError, ClassificationExecutionOutcome, ClassificationWorkExecutor,
};
use crate::ports::classification_setup::ClassificationSetupRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::classification_analysis::analyze_classification_content;
use super::classification_dispatch::{
    ClassificationDispatchSignals, ClassificationDispatchStep, ClassificationDispatcher,
};
use super::classification_residency::{ClassificationEncoderResidency, MonotonicResidencyClock};
use super::classification_screening::ScreeningClassifier;
use super::classification_verification::verify_classification_source;

mod admitted_text;
mod control;
mod ownership;
mod setup;
use admitted_text::AdmittedTextFactory;
pub use control::ClassificationRuntimeControl;

pub(crate) fn while_retired<T>(
    change: impl FnOnce() -> Result<T, ScanError>,
) -> Result<T, ScanError> {
    let _ownership = ownership::acquire()?;
    change()
}

pub struct ClassificationRuntimePaths {
    pub database: PathBuf,
    pub feature_cache: PathBuf,
    pub worker: PathBuf,
    pub models: PathBuf,
    pub runtime: PathBuf,
}

pub struct NativeClassificationRuntime {
    dispatcher: ClassificationDispatcher<SqliteCatalog, NativeExecutor>,
    control: ClassificationRuntimeControl,
    ownership: ownership::RuntimeOwnership<'static>,
}

impl NativeClassificationRuntime {
    pub fn open(
        paths: ClassificationRuntimePaths,
        session: ClassificationRecordId,
        signals: ClassificationDispatchSignals,
    ) -> Result<Self, ScanError> {
        let ownership = ownership::acquire()?;
        let catalog =
            super::catalog_session::open_catalog(&paths.database, LibraryChangeLane::Recovery)?;
        let state = catalog.classification_dispatch_state()?;
        let run_id = state.run_id.as_ref().ok_or_else(setup::mismatch)?;
        let setup = catalog.load_classification_setup(run_id)?.ok_or_else(|| {
            ScanError::new(
                "classification_setup_missing",
                "The saved analysis settings are incomplete; choose an analysis configuration",
            )
        })?;
        let control = ClassificationRuntimeControl::new(
            paths.database.clone(),
            session.clone(),
            signals.clone(),
        );
        let executor = NativeExecutor {
            database: paths.database,
            cache_path: paths.feature_cache,
            cache: None,
            setup,
            classifier: None,
            residency: ClassificationEncoderResidency::new(
                AdmittedTextFactory(LocalClassificationEncoderFactory {
                    executable: paths.worker,
                    model_directory: paths.models,
                    runtime_directory: paths.runtime,
                }),
                MonotonicResidencyClock::default(),
                Duration::from_secs(60),
            ),
        };
        let dispatcher = ClassificationDispatcher::open(catalog, executor, session, signals)?;
        Ok(Self {
            dispatcher,
            control,
            ownership,
        })
    }

    pub fn control(&self) -> ClassificationRuntimeControl {
        self.control.clone()
    }
    pub fn run(&mut self) -> Result<(), ScanError> {
        self.dispatcher.run()
    }
    pub fn step(&mut self) -> Result<ClassificationDispatchStep, ScanError> {
        self.dispatcher.step()
    }
}

impl Drop for NativeClassificationRuntime {
    fn drop(&mut self) {
        self.ownership.require_retirement();
        let _ = self.control.signals().close();
        if self.dispatcher.retire().is_ok() {
            self.ownership.confirm_retirement();
        }
    }
}

struct NativeExecutor {
    database: PathBuf,
    cache_path: PathBuf,
    cache: Option<SqliteClassificationFeatureCache>,
    setup: ClassificationAnalysisSetup,
    classifier: Option<ScreeningClassifier>,
    residency: ClassificationEncoderResidency<
        AdmittedTextFactory<LocalClassificationEncoderFactory>,
        MonotonicResidencyClock,
    >,
}

impl NativeExecutor {
    fn analyze(
        &mut self,
        lease: &ClassificationWorkLease,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationAnalysisResult, ScanError> {
        if self.cache.is_none() {
            self.cache = Some(SqliteClassificationFeatureCache::open(&self.cache_path)?);
        }
        let cache = self.cache.as_mut().expect("the feature cache is owned");
        if self.classifier.is_none() {
            self.classifier = Some(setup::prepare(
                &self.setup,
                &mut self.residency,
                cache,
                cancelled,
            )?);
        }
        analyze_classification_content(
            &self.database,
            lease,
            self.classifier.as_ref().expect("the matcher is prepared"),
            &mut self.residency,
            cache,
            cancelled,
        )
    }
}

impl ClassificationWorkExecutor for NativeExecutor {
    fn execute(
        &mut self,
        lease: &ClassificationWorkLease,
        now_unix_ms: u64,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationExecutionOutcome, ClassificationExecutionError> {
        if cancelled.load(Ordering::Acquire) {
            return Ok(ClassificationExecutionOutcome::Cancelled);
        }
        if &lease.run_id != self.setup.run().id() {
            return Err(ClassificationExecutionError::Blocked(setup::mismatch()));
        }
        let result = match &lease.target {
            ClassificationWorkTarget::VerifySource(_) => {
                verify_classification_source(&self.database, lease, now_unix_ms, cancelled).map(
                    |result| match result {
                        ClassificationVerificationResult::Completed => {
                            ClassificationExecutionOutcome::Completed
                        }
                        ClassificationVerificationResult::Cancelled => {
                            ClassificationExecutionOutcome::Cancelled
                        }
                        ClassificationVerificationResult::Superseded => {
                            ClassificationExecutionOutcome::Superseded
                        }
                    },
                )
            }
            ClassificationWorkTarget::AnalyzeContent { .. } => {
                self.analyze(lease, cancelled).map(|result| match result {
                    ClassificationAnalysisResult::Completed { .. } => {
                        ClassificationExecutionOutcome::Completed
                    }
                    ClassificationAnalysisResult::Cancelled => {
                        ClassificationExecutionOutcome::Cancelled
                    }
                    ClassificationAnalysisResult::Superseded => {
                        ClassificationExecutionOutcome::Superseded
                    }
                })
            }
        };
        match result {
            Err(_) if cancelled.load(Ordering::Acquire) => {
                Ok(ClassificationExecutionOutcome::Cancelled)
            }
            result => result.map_err(execution_error),
        }
    }

    fn retire(&mut self) -> Result<(), ScanError> {
        self.residency.retire()
    }
    fn retire_if_idle(&mut self) -> Result<(), ScanError> {
        self.residency.retire_if_idle()
    }
    fn idle_delay(&self) -> Option<Duration> {
        self.residency.idle_delay()
    }
}

fn execution_error(error: ScanError) -> ClassificationExecutionError {
    let kind = match error.code.as_str() {
        "viewer_source_guard_failed"
        | "preview_source_missing"
        | "preview_root_unavailable"
        | "preview_source_open_failed"
        | "classification_source_changed"
        | "exact_source_changed"
        | "preview_root_identity_changed"
        | "preview_source_changed"
        | "source_revalidation_failed"
        | "source_identity_unavailable"
        | "source_revision_unavailable"
        | "source_became_unavailable"
        | "classification_source_read_failed"
        | "source_changed_during_scan"
        | "source_replaced_during_scan"
        | "source_revision_changed_during_scan"
        | "classification_root_unproven"
        | "exact_root_identity_unproven" => ClassificationWorkFailure::Unavailable,
        "classification_image_limit"
        | "classification_image_decode_failed"
        | "classification_source_rejected"
        | "exact_source_size_exceeded" => ClassificationWorkFailure::Permanent,
        "classification_worker_timeout"
        | "classification_worker_exited"
        | "classification_worker_failed"
        | "exact_source_read_failed" => ClassificationWorkFailure::Transient,
        _ => return ClassificationExecutionError::Blocked(error),
    };
    ClassificationExecutionError::Work { kind, error }
}

#[cfg(test)]
mod tests;
