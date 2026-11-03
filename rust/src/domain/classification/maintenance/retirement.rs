use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;

use super::{ManagedCategoryOrigin, check_cancelled};
use crate::domain::classification::{CategoryDefinition, CategoryId, ModelCompatibility};
use crate::domain::classification_configuration::{
    ClassificationConfigurationChange, ClassificationConfigurationSnapshot,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryRetirementSnapshot {
    pub configuration: ClassificationConfigurationSnapshot,
    pub model: Option<ModelCompatibility>,
    pub origins: BTreeMap<CategoryId, ManagedCategoryOrigin>,
    pub human_categories: BTreeSet<CategoryId>,
}

impl CategoryRetirementSnapshot {
    pub(crate) fn protects(&self, category: &CategoryDefinition) -> bool {
        self.human_categories.contains(category.id())
            || self
                .origins
                .values()
                .any(|origin| origin.purpose.as_ref() == Some(category.id()))
            || self.origins.get(category.id()).is_none_or(|origin| {
                origin.protected
                    || origin.admitted_label != category.label()
                    || origin.admitted_semantic_revision != category.semantic_revision()
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquivalentCategoryRetirement {
    pub category_id: CategoryId,
    pub retained_id: CategoryId,
}

pub struct CategoryRetirementPlan {
    pub(crate) before: CategoryRetirementSnapshot,
    pub(crate) configuration: ClassificationConfigurationChange,
    pub(crate) retirements: Vec<EquivalentCategoryRetirement>,
}

impl CategoryRetirementPlan {
    pub fn retirements(&self) -> &[EquivalentCategoryRetirement] {
        &self.retirements
    }

    pub fn configuration(&self) -> &ClassificationConfigurationChange {
        &self.configuration
    }
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
