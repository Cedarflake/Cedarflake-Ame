use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{CategoryNaming, VocabularyConcept};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{CategoryDefinition, CategoryId, Taxonomy};

use super::{
    CategoryCoverageSummary, CategoryMergeSnapshot, PurposeCategoryMerge, check_cancelled,
};

pub(crate) fn select_purpose_category_merge(
    snapshot: &CategoryMergeSnapshot,
    coverage: &CategoryCoverageSummary,
    cancelled: &AtomicBool,
) -> Result<Option<PurposeCategoryMerge>, ScanError> {
    for category in sparse_categories(snapshot, coverage, cancelled)? {
        check_cancelled(cancelled)?;
        let basis = &snapshot.bases[&category];
        let support = &coverage.categories[&category];
        if support.retained_members.iter().any(|member| {
            member
                .alternative
                .as_ref()
                .is_none_or(|alternative| &alternative.category_id != basis.naming.category())
        }) {
            continue;
        }
        return Ok(Some(PurposeCategoryMerge {
            category_id: category,
            retained_id: basis.naming.category().clone(),
            context: None,
        }));
    }
    check_cancelled(cancelled)?;
    Ok(None)
}

pub(crate) fn validate_category_merge(
    snapshot: &CategoryMergeSnapshot,
    coverage: &CategoryCoverageSummary,
    merged: &PurposeCategoryMerge,
    cancelled: &AtomicBool,
) -> Result<bool, ScanError> {
    if merged.context.is_none() {
        return Ok(
            select_purpose_category_merge(snapshot, coverage, cancelled)?.as_ref() == Some(merged),
        );
    }
    let eligible = sparse_categories(snapshot, coverage, cancelled)?;
    if !eligible.contains(&merged.category_id) {
        return Ok(false);
    }
    check_cancelled(cancelled)?;
    compatible_context_target(snapshot, coverage, merged)
}

pub(crate) fn compatible_context_target(
    snapshot: &CategoryMergeSnapshot,
    coverage: &CategoryCoverageSummary,
    merged: &PurposeCategoryMerge,
) -> Result<bool, ScanError> {
    let context = merged.context.as_ref().ok_or_else(changed)?;
    let Some(source) = snapshot.bases.get(&merged.category_id) else {
        return Ok(false);
    };
    let Some(target) = snapshot.bases.get(&merged.retained_id) else {
        return Ok(false);
    };
    if source.naming != target.naming || !(2..=511).contains(&target.minimum_members) {
        return Ok(false);
    }
    let origins = &snapshot.categories.origins;
    let source_origin = origins.get(&merged.category_id).ok_or_else(changed)?;
    let target_origin = origins.get(&merged.retained_id).ok_or_else(changed)?;
    if context.narrower.id() != source_origin.concept_id
        || context.broader.id() != target_origin.concept_id
        || source_origin.purpose != target_origin.purpose
        || context.narrower.id() == context.broader.id()
        || !(2..=5).contains(&context.path.len())
        || context.path.first() != Some(&context.narrower.id())
        || context.path.last() != Some(&context.broader.id())
    {
        return Ok(false);
    }
    let support = coverage
        .categories
        .get(&merged.category_id)
        .ok_or_else(changed)?;
    let target_support = coverage
        .categories
        .get(&merged.retained_id)
        .ok_or_else(changed)?;
    if support.contents == 0
        || target_support.contents < target.minimum_members as u64
        || support.retained_members.iter().any(|member| {
            member
                .alternative
                .as_ref()
                .is_none_or(|alternative| alternative.category_id != merged.retained_id)
        })
    {
        return Ok(false);
    }
    let rules = snapshot
        .categories
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(changed)?;
    let naming = CategoryNaming::WithinCategory(source.naming.clone());
    Ok(matches_definition(
        rules,
        &merged.category_id,
        &naming.project(&context.narrower),
    )? && matches_definition(
        rules,
        &merged.retained_id,
        &naming.project(&context.broader),
    )?)
}

fn matches_definition(
    rules: &Taxonomy,
    id: &CategoryId,
    concept: &VocabularyConcept,
) -> Result<bool, ScanError> {
    let Some(current) = rules
        .categories()
        .iter()
        .find(|category| category.id() == id)
    else {
        return Ok(false);
    };
    let expected = CategoryDefinition::new(
        id.clone(),
        concept.label().into(),
        vec![concept.description().into()],
    )
    .map_err(|_| changed())?;
    Ok(current.semantic_revision() == expected.semantic_revision())
}

pub(crate) fn sparse_categories(
    snapshot: &CategoryMergeSnapshot,
    coverage: &CategoryCoverageSummary,
    cancelled: &AtomicBool,
) -> Result<Vec<CategoryId>, ScanError> {
    check_cancelled(cancelled)?;
    let before = &snapshot.categories;
    if before.configuration != coverage.snapshot.configuration
        || before.model != Some(coverage.snapshot.model)
        || before.configuration.dispatch.mode != ClassificationDispatchMode::Automatic
    {
        return Err(changed());
    }
    if !coverage.is_complete() || coverage.classified_contents == 0 {
        return Ok(Vec::new());
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
        if support.contents < basis.minimum_members as u64
            && support.contents == support.retained_members.len() as u64
        {
            eligible.push(category.id().clone());
        }
    }
    eligible.sort_by(|left, right| {
        coverage.categories[left]
            .contents
            .cmp(&coverage.categories[right].contents)
            .then_with(|| left.cmp(right))
    });
    check_cancelled(cancelled)?;
    Ok(eligible)
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_changed",
        "Category merge evidence or authority changed",
    )
}

#[cfg(test)]
#[path = "../../../../test_support/classification_merge_fixture.rs"]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
