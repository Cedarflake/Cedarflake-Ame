use std::collections::VecDeque;

use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{ClassificationContent, ClassificationRecordId};
use crate::domain::semantic_results::{SEMANTIC_CANDIDATE_WINDOW, SemanticIndexStatus};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::classification_features::ClassificationFeatureRepository;
use crate::ports::classification_work::ClassificationWorkRepository;
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::semantic_catalog::{SemanticCandidate, SemanticCatalog};
use crate::ports::semantic_features::SemanticFeatureRepository;

use super::super::classification_dispatch::{Admission, ClassificationDispatchSignals};
use super::super::semantic_requests::SearchRequests;
use super::executor::NativeExecutor;
use super::semantic::SemanticExecution;

#[derive(Default)]
pub(super) struct SemanticIndexPass {
    cursor: String,
    pending: VecDeque<SemanticCandidate>,
    completed_revision: Option<u64>,
    status: SemanticIndexStatus,
}

impl SemanticIndexPass {
    pub(super) fn step(
        &mut self,
        catalog: &mut (impl SemanticCatalog + ExactDuplicateRepository + ClassificationWorkRepository),
        executor: &mut NativeExecutor,
        execution: &mut SemanticExecution,
        signals: &ClassificationDispatchSignals,
        requests: &SearchRequests,
    ) -> Result<bool, ScanError> {
        if self.status.blocked {
            return Ok(false);
        }
        let automatic =
            catalog.classification_dispatch_state()?.mode == ClassificationDispatchMode::Automatic;
        let Admission::Ready(active) = signals.admission(automatic)? else {
            self.status.indexing = false;
            requests.publish_index(self.status.clone())?;
            return Ok(false);
        };
        let result = self.advance(
            catalog,
            executor,
            execution,
            active.catalog_revision,
            &active.cancelled,
        );
        let progress = match result {
            Ok(progress) => progress,
            Err(_) if active.cancelled.load(std::sync::atomic::Ordering::Acquire) => true,
            Err(error) => {
                self.status.issue_code = Some(error.code);
                self.status.indexing = false;
                self.status.blocked = true;
                false
            }
        };
        requests.publish_index(self.status.clone())?;
        Ok(progress)
    }

    fn advance(
        &mut self,
        catalog: &mut (impl SemanticCatalog + ExactDuplicateRepository),
        executor: &mut NativeExecutor,
        execution: &mut SemanticExecution,
        revision: u64,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<bool, ScanError> {
        let resources = executor.feature_resources()?;
        let storage = resources.cache.semantic().storage()?;
        self.status.feature_generation = storage.generation;
        self.status.budget_reached = storage.budget_reached;
        if storage.budget_reached {
            self.status.indexing = false;
            return Ok(false);
        }
        if self.pending.is_empty() {
            if self.completed_revision == Some(revision) {
                return Ok(false);
            }
            if self.completed_revision.take().is_some() {
                self.cursor.clear();
            }
            let window = catalog.semantic_candidates(&GalleryQuery::default(), &self.cursor)?;
            self.status.catalog_revision = window.revision;
            if window.candidates.is_empty() {
                execution.finish(catalog)?;
                self.completed_revision = Some(revision);
                self.status.indexing = false;
                return Ok(false);
            }
            self.pending = window.candidates.into();
        }
        self.status.indexing = true;
        let model = resources.residency.compatibility();
        // A bounded cache-only pass promotes old compatible vectors without reading media.
        for _ in 0..SEMANTIC_CANDIDATE_WINDOW {
            super::super::semantic_retrieval::check_cancelled(cancelled)?;
            let Some(candidate) = self.pending.front() else {
                return Ok(true);
            };
            if let Some(content) = &candidate.content {
                let content = ClassificationContent {
                    scheme: ClassificationRecordId::new(content.scheme().to_owned()).map_err(
                        |error| ScanError::new("semantic_content_invalid", error.to_string()),
                    )?,
                    digest: *content.digest(),
                    byte_length: content.byte_length(),
                };
                if resources
                    .cache
                    .load_image_feature(model, &content)?
                    .is_some()
                {
                    self.cursor = self
                        .pending
                        .pop_front()
                        .expect("the candidate is present")
                        .source
                        .location_id;
                    continue;
                }
            }
            break;
        }
        let Some(candidate) = self.pending.front() else {
            return Ok(true);
        };
        let result = execution.image(catalog, executor, &candidate.source, cancelled);
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(true);
        }
        if let Err(error) = result {
            if error.code == "analysis_yielded" {
                executor.mark_yielded();
                return Ok(false);
            }
            match super::executor::execution_error(error) {
                crate::ports::classification_execution::ClassificationExecutionError::Work {
                    error,
                    ..
                } => {
                    self.status.issue_code = Some(error.code);
                }
                crate::ports::classification_execution::ClassificationExecutionError::Blocked(
                    error,
                ) => return Err(error),
            }
        }
        self.cursor = self
            .pending
            .pop_front()
            .expect("the candidate is present")
            .source
            .location_id;
        Ok(true)
    }
}
use crate::ports::image_feature_encoding::ImageFeatureEncoding;
