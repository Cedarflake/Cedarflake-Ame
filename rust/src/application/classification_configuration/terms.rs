use std::collections::BTreeMap;

use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationAnalysisSetup, ClassificationRecordId,
};
use crate::domain::classification_configuration::{
    ClassificationCategoryInput, ClassificationSettings, ClassificationTerm,
    ClassificationTermChange, ClassificationTermsReset,
};
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_configuration::{
    ClassificationConfigurationRepository, ClassificationTermRepository,
};
use crate::ports::classification_model_package::ClassificationModelPackage;

use super::{
    StoragePaths, classification_candidate, defaults, invalid, model_package, open_catalog,
    settings, storage_paths, while_retired,
};

pub(crate) fn load() -> Result<Vec<ClassificationTerm>, ScanError> {
    load_with_storage(&storage_paths()?)
}

fn load_with_storage(storage: &StoragePaths) -> Result<Vec<ClassificationTerm>, ScanError> {
    let catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let snapshot = catalog.load_classification_configuration()?;
    let defaults = defaults::categories()?;
    let mut categories: BTreeMap<_, _> = defaults
        .values()
        .map(|category| (category.id(), (category, false)))
        .collect();
    if let Some(taxonomy) = &snapshot.catalog.taxonomy {
        categories.extend(
            taxonomy
                .categories()
                .iter()
                .map(|category| (category.id(), (category, true))),
        );
    }
    Ok(categories
        .into_values()
        .filter(|(category, _)| category.id().as_str() != "other")
        .map(|(category, is_configured)| ClassificationTerm {
            category: input(category),
            default_category: defaults.get(category.id()).map(input),
            is_custom: category.id().as_str().starts_with("custom-"),
            is_configured,
        })
        .collect())
}

pub(crate) fn save(
    operation_id: String,
    expected: Option<ClassificationCategoryInput>,
    replacement: ClassificationCategoryInput,
) -> Result<ClassificationSettings, ScanError> {
    execute(
        operation_id,
        expected.map(definition).transpose()?,
        Some(definition(replacement)?),
    )
}

pub(crate) fn remove(
    operation_id: String,
    expected: ClassificationCategoryInput,
) -> Result<ClassificationSettings, ScanError> {
    execute(operation_id, Some(definition(expected)?), None)
}

pub(crate) fn reset(
    operation_id: String,
    expected: ClassificationCategoryInput,
) -> Result<ClassificationSettings, ScanError> {
    let expected = definition(expected)?;
    let replacement = defaults::categories()?
        .get(expected.id())
        .cloned()
        .ok_or_else(|| {
            ScanError::new(
                "classification_term_invalid",
                "This category has no bundled default",
            )
        })?;
    execute(operation_id, Some(expected), Some(replacement))
}

pub(crate) fn reset_all(
    operation_id: String,
    expected: Vec<ClassificationCategoryInput>,
) -> Result<ClassificationSettings, ScanError> {
    let change = reset_change(operation_id, expected)?;
    let storage = storage_paths()?;
    if change.changes_matching() {
        while_retired(|| reset_with_storage(&storage, &change))
    } else {
        reset_with_storage(&storage, &change)
    }
}

fn reset_change(
    operation_id: String,
    expected: Vec<ClassificationCategoryInput>,
) -> Result<ClassificationTermsReset, ScanError> {
    let defaults = defaults::categories()?;
    if expected.len() > defaults.len() {
        return Err(ScanError::new(
            "classification_term_invalid",
            "Too many categories to restore",
        ));
    }
    ClassificationTermsReset::new(
        baseline(operation_id)?,
        expected
            .into_iter()
            .map(definition)
            .collect::<Result<_, _>>()?,
        defaults,
    )
}

fn reset_with_storage(
    storage: &StoragePaths,
    change: &ClassificationTermsReset,
) -> Result<ClassificationSettings, ScanError> {
    let mut catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    settings(catalog.reset_all_classification_terms(change)?, storage)
}

fn execute(
    operation_id: String,
    expected: Option<CategoryDefinition>,
    replacement: Option<CategoryDefinition>,
) -> Result<ClassificationSettings, ScanError> {
    let change = change(operation_id, expected, replacement)?;
    let storage = storage_paths()?;
    if change.changes_matching() {
        while_retired(|| save_with_storage(&storage, &change))
    } else {
        save_with_storage(&storage, &change)
    }
}

fn change(
    operation_id: String,
    expected: Option<CategoryDefinition>,
    replacement: Option<CategoryDefinition>,
) -> Result<ClassificationTermChange, ScanError> {
    let baseline = baseline(operation_id)?;
    let bundled_default = match replacement.as_ref() {
        Some(category) => defaults::categories()?.get(category.id()).cloned(),
        None => None,
    };
    Ok(ClassificationTermChange {
        expected,
        replacement,
        bundled_default,
        baseline,
    })
}

fn baseline(operation_id: String) -> Result<ClassificationAnalysisSetup, ScanError> {
    classification_candidate::prepare(
        ClassificationRecordId::new(operation_id).map_err(invalid)?,
        classification_candidate::initial_categories().map_err(invalid)?,
        model_package().compatibility(),
    )
    .map_err(invalid)
}

fn save_with_storage(
    storage: &StoragePaths,
    change: &ClassificationTermChange,
) -> Result<ClassificationSettings, ScanError> {
    let mut catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    settings(catalog.save_classification_term(change)?, storage)
}

fn definition(category: ClassificationCategoryInput) -> Result<CategoryDefinition, ScanError> {
    CategoryDefinition::new(
        CategoryId::new(category.id).map_err(invalid)?,
        category.label,
        category.descriptions,
    )
    .map_err(invalid)
}

fn input(category: &CategoryDefinition) -> ClassificationCategoryInput {
    ClassificationCategoryInput {
        id: category.id().as_str().into(),
        label: category.label().into(),
        descriptions: category.descriptions().to_vec(),
    }
}

#[cfg(test)]
mod tests;
