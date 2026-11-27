use crate::domain::classification::{CategoryDefinition, CategoryId, ClassificationRecordId};
use crate::domain::classification_configuration::{
    ClassificationCategoryInput, ClassificationSettings, ClassificationTerm,
    ClassificationTermChange,
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
    let taxonomy = match snapshot.catalog.taxonomy {
        Some(taxonomy) => taxonomy,
        None => classification_candidate::initial_categories().map_err(invalid)?,
    };
    Ok(taxonomy
        .categories()
        .iter()
        .filter(|category| category.id().as_str() != "other")
        .map(|category| ClassificationTerm {
            category: input(category),
            default_category: defaults.get(category.id()).map(input),
            is_custom: category.id().as_str().starts_with("custom-"),
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
    let operation = ClassificationRecordId::new(operation_id).map_err(invalid)?;
    let baseline = classification_candidate::prepare(
        operation,
        classification_candidate::initial_categories().map_err(invalid)?,
        model_package().compatibility(),
    )
    .map_err(invalid)?;
    Ok(ClassificationTermChange {
        expected,
        replacement,
        baseline,
    })
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
