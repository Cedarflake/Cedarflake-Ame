use crate::domain::classification::CategoryId;

use super::*;

pub(super) fn sparse_categories(
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
