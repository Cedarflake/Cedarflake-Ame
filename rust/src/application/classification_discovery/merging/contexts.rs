use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::maintenance::merging_rules::{
    compatible_context_target, sparse_categories,
};
use crate::domain::classification::maintenance::{
    CategoryCoverageSummary, CategoryMergeSnapshot, PurposeCategoryMerge,
};

use super::super::{NamingContextCatalog, check_cancelled};
use super::changed;

pub(super) fn select_context_category_merge(
    snapshot: &CategoryMergeSnapshot,
    coverage: &CategoryCoverageSummary,
    contexts: &NamingContextCatalog,
    cancelled: &AtomicBool,
) -> Result<Option<PurposeCategoryMerge>, ScanError> {
    for category_id in sparse_categories(snapshot, coverage, cancelled)? {
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
