use std::sync::{Arc, atomic::AtomicBool};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::merging_rules::select_purpose_category_merge;
use crate::domain::classification::maintenance::{CategoryCoverageSummary, CategoryMergePlan};
use crate::domain::classification::{
    ClassificationAnalysisSetup, ClassificationRecordId, ClassificationRun, Taxonomy,
};
use crate::domain::classification_configuration::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
};
use crate::ports::classification_configuration::CategoryMergeRepository;

use super::{NamingContextCatalog, check_cancelled};

mod contexts;

pub fn prepare_purpose_category_merge(
    operation_id: ClassificationRecordId,
    coverage: CategoryCoverageSummary,
    repository: &impl CategoryMergeRepository,
    cancelled: &Arc<AtomicBool>,
) -> Result<Option<CategoryMergePlan>, ScanError> {
    prepare_category_merge(operation_id, coverage, None, repository, cancelled)
}

pub fn prepare_category_merge(
    operation_id: ClassificationRecordId,
    coverage: CategoryCoverageSummary,
    contexts: Option<&NamingContextCatalog>,
    repository: &impl CategoryMergeRepository,
    cancelled: &Arc<AtomicBool>,
) -> Result<Option<CategoryMergePlan>, ScanError> {
    check_cancelled(cancelled)?;
    let before = repository.load_category_merge_snapshot(cancelled)?;
    let mut selected = select_purpose_category_merge(&before, &coverage, cancelled)?;
    if selected.is_none()
        && let Some(contexts) = contexts
    {
        selected =
            contexts::select_context_category_merge(&before, &coverage, contexts, cancelled)?;
    }
    let Some(merged) = selected else {
        return Ok(None);
    };
    let current = before
        .categories
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(changed)?;
    let rules = Taxonomy::new(
        current
            .categories()
            .iter()
            .filter(|category| category.id() != &merged.category_id)
            .cloned()
            .collect(),
    )
    .map_err(|_| changed())?;
    let run = ClassificationRun::new(
        operation_id.clone(),
        coverage.snapshot.model,
        rules.revision(),
        None,
    )
    .map_err(|_| changed())?;
    let configuration = ClassificationConfigurationChange {
        operation_id,
        expected_catalog_revision: before.categories.configuration.catalog.revision,
        expected_dispatch_revision: before.categories.configuration.dispatch.revision,
        setup: ClassificationAnalysisSetup::new(run, rules, None).map_err(|_| changed())?,
        action: ClassificationConfigurationAction::Save,
    };
    repository.validate_category_coverage(&coverage.snapshot, cancelled)?;
    check_cancelled(cancelled)?;
    Ok(Some(CategoryMergePlan {
        before,
        coverage,
        configuration,
        merged,
    }))
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_changed",
        "Category merge evidence or authority changed",
    )
}

#[cfg(test)]
mod tests;
