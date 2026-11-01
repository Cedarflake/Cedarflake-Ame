use crate::domain::classification::discovery::{CategoryNaming, VocabularyConcept};
use crate::domain::classification::{CategoryDefinition, CategoryId};

use super::*;

pub(super) fn select_context_category_merge(
    snapshot: &CategoryMergeSnapshot,
    coverage: &CategoryCoverageSummary,
    contexts: &NamingContextCatalog,
    cancelled: &AtomicBool,
) -> Result<Option<PurposeCategoryMerge>, ScanError> {
    for category_id in eligibility::sparse_categories(snapshot, coverage, cancelled)? {
        check_cancelled(cancelled)?;
        let support = &coverage.categories[&category_id];
        let Some(alternative) = support
            .retained_members
            .first()
            .and_then(|member| member.alternative.as_ref())
        else {
            continue;
        };
        let origins = &snapshot.categories.origins;
        let Some(target) = origins.get(&alternative.category_id) else {
            continue;
        };
        let source = origins.get(&category_id).ok_or_else(changed)?;
        let Some(context) =
            contexts.broader_merge_context(source.concept_id, target.concept_id, cancelled)?
        else {
            continue;
        };
        let merged = PurposeCategoryMerge {
            category_id,
            retained_id: alternative.category_id.clone(),
            context: Some(context),
        };
        if compatible_context_target(snapshot, coverage, &merged)? {
            return Ok(Some(merged));
        }
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
    let eligible = eligibility::sparse_categories(snapshot, coverage, cancelled)?;
    if !eligible.contains(&merged.category_id) {
        return Ok(false);
    }
    check_cancelled(cancelled)?;
    compatible_context_target(snapshot, coverage, merged)
}

fn compatible_context_target(
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
