use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::adapters::classification_runtime::LocalClassificationEncoderFactory;
use crate::adapters::{SqliteClassificationFeatureCache, SqliteSemanticFeatureStore};
use crate::domain::ScanError;
use crate::domain::classification::ClassificationAnalysisSetup;
use crate::domain::classification::work::{
    ClassificationAnalysisResult, ClassificationVerificationResult, ClassificationWorkFailure,
    ClassificationWorkLease, ClassificationWorkTarget,
};
use crate::ports::classification_execution::{
    ClassificationExecutionError, ClassificationExecutionOutcome, ClassificationWorkExecutor,
};

use super::super::analysis_scheduling::{AnalysisScheduler, Priority, ScheduledEncoding};
use super::super::classification_analysis::analyze_classification_content;
use super::super::classification_screening::ScreeningClassifier;
use super::super::classification_verification::verify_classification_source;
use super::super::image_feature_reuse::SharedImageFeatures;
use super::{ClassificationRuntimePaths, setup};

type NativeEncoderResidency = ScheduledEncoding<LocalClassificationEncoderFactory>;

type NativeFeatureRepository =
    SharedImageFeatures<SqliteClassificationFeatureCache, SqliteSemanticFeatureStore>;

pub(super) struct FeatureResources<'a> {
    pub cache: &'a mut NativeFeatureRepository,
    pub residency: &'a mut NativeEncoderResidency,
}

pub(super) struct NativeExecutor {
    database: PathBuf,
    cache_directory: PathBuf,
    cache: Option<NativeFeatureRepository>,
    setup: ClassificationAnalysisSetup,
    classifier: Option<ScreeningClassifier>,
    residency: NativeEncoderResidency,
    yielded: bool,
}

impl NativeExecutor {
    pub(super) fn new(
        paths: &ClassificationRuntimePaths,
        setup: ClassificationAnalysisSetup,
        residency: NativeEncoderResidency,
    ) -> Self {
        Self {
            database: paths.database.clone(),
            cache_directory: paths.feature_cache_directory.clone(),
            cache: None,
            setup,
            classifier: None,
            residency,
            yielded: false,
        }
    }

    pub(super) fn scheduler(&self) -> Arc<AnalysisScheduler<LocalClassificationEncoderFactory>> {
        self.residency.scheduler()
    }

    pub(super) fn search_executor(&self) -> Self {
        Self {
            database: self.database.clone(),
            cache_directory: self.cache_directory.clone(),
            cache: None,
            setup: self.setup.clone(),
            classifier: None,
            residency: self.scheduler().client(Priority::Foreground),
            yielded: false,
        }
    }

    pub(super) fn take_yielded(&mut self) -> bool {
        std::mem::take(&mut self.yielded)
    }
    pub(super) fn mark_yielded(&mut self) {
        self.yielded = true;
    }

    pub(super) fn database(&self) -> &std::path::Path {
        &self.database
    }

    pub(super) fn taxonomy(&self) -> &crate::domain::classification::Taxonomy {
        self.setup.primary()
    }

    pub(super) fn feature_resources(&mut self) -> Result<FeatureResources<'_>, ScanError> {
        if self.cache.is_none() {
            self.cache = Some(self.open_features()?);
        }
        Ok(FeatureResources {
            cache: self.cache.as_mut().expect("the feature cache is owned"),
            residency: &mut self.residency,
        })
    }

    fn analyze(
        &mut self,
        lease: &ClassificationWorkLease,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationAnalysisResult, ScanError> {
        if self.cache.is_none() {
            self.cache = Some(self.open_features()?);
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
    fn open_features(&self) -> Result<NativeFeatureRepository, ScanError> {
        Ok(SharedImageFeatures::new(
            SqliteClassificationFeatureCache::open(&self.cache_directory)?,
            SqliteSemanticFeatureStore::open(
                &self.cache_directory.with_file_name("semantic-features"),
            )?,
        ))
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
            Err(error) if error.code == "analysis_yielded" => {
                self.yielded = true;
                Ok(ClassificationExecutionOutcome::Cancelled)
            }
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
        Some(Duration::from_secs(2))
    }
}

pub(super) fn execution_error(error: ScanError) -> ClassificationExecutionError {
    let kind = match error.code.as_str() {
        "viewer_source_guard_failed"
        | "preview_source_missing"
        | "preview_root_unavailable"
        | "preview_source_open_failed"
        | "classification_source_changed"
        | "semantic_source_changed"
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
