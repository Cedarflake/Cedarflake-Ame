use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryRetirementPlan, CategoryRetirementSnapshot, EquivalentCategoryRetirement,
};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{
    CategoryDefinition, ClassificationAnalysisSetup, ClassificationRecordId, ClassificationRun,
    Taxonomy,
};
use crate::domain::classification_configuration::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
};
use crate::ports::classification_configuration::CategoryRetirementRepository;

use super::check_cancelled;

pub fn prepare_equivalent_category_retirement(
    operation_id: ClassificationRecordId,
    repository: &impl CategoryRetirementRepository,
    cancelled: &AtomicBool,
) -> Result<Option<CategoryRetirementPlan>, ScanError> {
    check_cancelled(cancelled)?;
    let before = repository.load_category_retirement_snapshot()?;
    if before.configuration.dispatch.mode != ClassificationDispatchMode::Automatic {
        return Err(changed());
    }
    let model = before.model.ok_or_else(changed)?;
    let current = before
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(changed)?;
    let retirements = equivalent_retirements(&before, cancelled)?;
    if retirements.is_empty() {
        return Ok(None);
    }
    let retired: BTreeSet<_> = retirements.iter().map(|entry| &entry.category_id).collect();
    let rules = Taxonomy::new(
        current
            .categories()
            .iter()
            .filter(|category| !retired.contains(category.id()))
            .cloned()
            .collect(),
    )
    .map_err(|_| changed())?;
    let run = ClassificationRun::new(operation_id.clone(), model, rules.revision(), None)
        .map_err(|_| changed())?;
    let configuration = ClassificationConfigurationChange {
        operation_id,
        expected_catalog_revision: before.configuration.catalog.revision,
        expected_dispatch_revision: before.configuration.dispatch.revision,
        setup: ClassificationAnalysisSetup::new(run, rules, None).map_err(|_| changed())?,
        action: ClassificationConfigurationAction::Save,
    };
    check_cancelled(cancelled)?;
    Ok(Some(CategoryRetirementPlan {
        before,
        configuration,
        retirements,
    }))
}

pub(crate) fn equivalent_retirements(
    snapshot: &CategoryRetirementSnapshot,
    cancelled: &AtomicBool,
) -> Result<Vec<EquivalentCategoryRetirement>, ScanError> {
    let rules = snapshot
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(changed)?;
    let mut groups: BTreeMap<_, Vec<&CategoryDefinition>> = BTreeMap::new();
    for category in rules.categories() {
        check_cancelled(cancelled)?;
        let purpose = snapshot
            .origins
            .get(category.id())
            .and_then(|origin| origin.purpose.as_ref());
        groups
            .entry((purpose, category.descriptions()))
            .or_default()
            .push(category);
    }
    let mut retirements = Vec::new();
    for group in groups.values() {
        check_cancelled(cancelled)?;
        let Some(retained) = group
            .iter()
            .min_by_key(|category| (!snapshot.protects(category), category.id()))
        else {
            continue;
        };
        for category in group {
            if category.id() == retained.id() || snapshot.protects(category) {
                continue;
            }
            retirements.push(EquivalentCategoryRetirement {
                category_id: category.id().clone(),
                retained_id: retained.id().clone(),
            });
        }
    }
    retirements.sort_by(|left, right| left.category_id.cmp(&right.category_id));
    retirements.truncate(64);
    Ok(retirements)
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_changed",
        "The category configuration or maintenance authority changed",
    )
}
