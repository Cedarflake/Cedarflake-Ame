use std::sync::{Arc, atomic::AtomicBool};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryCoverageSummary, CategoryMergePlan, CategoryMergeSnapshot, PurposeCategoryMerge,
};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{
    ClassificationAnalysisSetup, ClassificationRecordId, ClassificationRun, Taxonomy,
};
use crate::domain::classification_configuration::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
};
use crate::ports::classification_configuration::CategoryMergeRepository;

use super::check_cancelled;

pub fn prepare_purpose_category_merge(
    operation_id: ClassificationRecordId,
    coverage: CategoryCoverageSummary,
    repository: &impl CategoryMergeRepository,
    cancelled: &Arc<AtomicBool>,
) -> Result<Option<CategoryMergePlan>, ScanError> {
    check_cancelled(cancelled)?;
    let before = repository.load_category_merge_snapshot(cancelled)?;
    let Some(merged) = select_purpose_category_merge(&before, &coverage, cancelled)? else {
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

pub(crate) fn select_purpose_category_merge(
    snapshot: &CategoryMergeSnapshot,
    coverage: &CategoryCoverageSummary,
    cancelled: &AtomicBool,
) -> Result<Option<PurposeCategoryMerge>, ScanError> {
    check_cancelled(cancelled)?;
    let before = &snapshot.categories;
    if before.configuration != coverage.snapshot.configuration
        || before.model != Some(coverage.snapshot.model)
        || before.configuration.dispatch.mode != ClassificationDispatchMode::Automatic
    {
        return Err(changed());
    }
    if !coverage.is_complete() || coverage.classified_contents == 0 {
        return Ok(None);
    }
    let rules = before
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(changed)?;
    let mut eligible = Vec::new();
    for category in rules.categories() {
        check_cancelled(cancelled)?;
        if before.protects(category) {
            continue;
        }
        let Some(basis) = snapshot.bases.get(category.id()) else {
            continue;
        };
        let origin = before.origins.get(category.id()).ok_or_else(changed)?;
        let parent = rules
            .categories()
            .iter()
            .find(|category| category.id() == basis.naming.category());
        if origin.purpose.as_ref() != Some(basis.naming.category())
            || parent
                .is_none_or(|parent| parent.semantic_revision() != basis.naming.category_revision())
            || !(2..=511).contains(&basis.minimum_members)
        {
            continue;
        }
        let support = coverage.categories.get(category.id()).ok_or_else(changed)?;
        if support.contents >= basis.minimum_members as u64
            || support.contents != support.retained_members.len() as u64
        {
            continue;
        }
        if support.retained_members.iter().any(|member| {
            member
                .alternative
                .as_ref()
                .is_none_or(|alternative| &alternative.category_id != basis.naming.category())
        }) {
            continue;
        }
        eligible.push((
            support.contents,
            PurposeCategoryMerge {
                category_id: category.id().clone(),
                retained_id: basis.naming.category().clone(),
            },
        ));
    }
    eligible.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.category_id.cmp(&right.1.category_id))
    });
    check_cancelled(cancelled)?;
    Ok(eligible.into_iter().next().map(|(_, merge)| merge))
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_changed",
        "Category merge evidence or authority changed",
    )
}

#[cfg(test)]
mod tests;
