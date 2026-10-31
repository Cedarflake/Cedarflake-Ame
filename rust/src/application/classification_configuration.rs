use crate::adapters::ClassificationInstallationStore;
use crate::adapters::classification_runtime::{
    selected_model_compatibility, verify_local_installation,
};
use crate::domain::classification::work::ClassificationDispatchCommand;
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationError, ClassificationRecordId, Taxonomy,
};
use crate::domain::classification_configuration::{
    ClassificationCategoryInput, ClassificationConfigurationAction,
    ClassificationConfigurationChange, ClassificationConfigurationSnapshot,
    ClassificationControlAction, ClassificationSettings,
};
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_configuration::ClassificationConfigurationRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::catalog_session::open_catalog;
use super::classification_candidate;
use super::classification_desktop::installation_path;
use super::classification_runtime::while_retired;
use super::storage::{StoragePaths, storage_paths};

pub(crate) fn load() -> Result<ClassificationSettings, ScanError> {
    load_with_storage(&storage_paths()?)
}

fn load_with_storage(storage: &StoragePaths) -> Result<ClassificationSettings, ScanError> {
    let catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    settings(catalog.load_classification_configuration()?, storage)
}

pub(crate) fn initial_categories() -> Result<Vec<ClassificationCategoryInput>, ScanError> {
    classification_candidate::initial_categories()
        .map(|taxonomy| category_inputs(&taxonomy))
        .map_err(invalid)
}

pub(crate) fn save(
    operation_id: String,
    expected_catalog_revision: u64,
    expected_dispatch_revision: u64,
    categories: Vec<ClassificationCategoryInput>,
    action: ClassificationConfigurationAction,
) -> Result<ClassificationSettings, ScanError> {
    let id = ClassificationRecordId::new(operation_id).map_err(invalid)?;
    let taxonomy = Taxonomy::new(
        categories
            .into_iter()
            .map(|category| {
                CategoryDefinition::new(
                    CategoryId::new(category.id)?,
                    category.label,
                    category.descriptions,
                )
            })
            .collect::<Result<_, _>>()
            .map_err(invalid)?,
    )
    .map_err(invalid)?;
    let setup =
        classification_candidate::prepare(id.clone(), taxonomy, selected_model_compatibility())
            .map_err(invalid)?;
    let change = ClassificationConfigurationChange {
        operation_id: id,
        expected_catalog_revision,
        expected_dispatch_revision,
        setup,
        action,
    };
    let storage = storage_paths()?;
    while_retired(|| save_with_storage(&storage, &change))
}

fn save_with_storage(
    storage: &StoragePaths,
    change: &ClassificationConfigurationChange,
) -> Result<ClassificationSettings, ScanError> {
    if change.action == ClassificationConfigurationAction::Enable {
        let installation = ClassificationInstallationStore::load(&installation_path(storage))?
            .ok_or_else(|| {
                ScanError::new(
                    "classification_installation_missing",
                    "Choose the local classification model and runtime directories",
                )
            })?;
        verify_local_installation(&installation.models, &installation.runtime)?;
    }
    let mut catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    settings(catalog.save_classification_configuration(change)?, storage)
}

pub(crate) fn change_control(
    expected_revision: u64,
    action: ClassificationControlAction,
) -> Result<ClassificationSettings, ScanError> {
    let command = match action {
        ClassificationControlAction::Pause => ClassificationDispatchCommand::Pause,
        ClassificationControlAction::Continue => ClassificationDispatchCommand::Continue,
        ClassificationControlAction::Disable => ClassificationDispatchCommand::Disable,
        ClassificationControlAction::Stop => ClassificationDispatchCommand::Stop,
    };
    let storage = storage_paths()?;
    while_retired(|| {
        let mut catalog = open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
        catalog.change_classification_dispatch(expected_revision, &command)?;
        settings(catalog.load_classification_configuration()?, &storage)
    })
}

fn settings(
    snapshot: ClassificationConfigurationSnapshot,
    storage: &StoragePaths,
) -> Result<ClassificationSettings, ScanError> {
    let categories = snapshot
        .catalog
        .taxonomy
        .as_ref()
        .map(category_inputs)
        .unwrap_or_default();
    Ok(ClassificationSettings {
        revision: snapshot.catalog.revision,
        dispatch_revision: snapshot.dispatch.revision,
        mode: snapshot.dispatch.mode,
        categories,
        run_id: snapshot.dispatch.run_id.map(|id| id.as_str().to_owned()),
        installation_selected: ClassificationInstallationStore::load(&installation_path(storage))?
            .is_some(),
    })
}

fn category_inputs(taxonomy: &Taxonomy) -> Vec<ClassificationCategoryInput> {
    taxonomy
        .categories()
        .iter()
        .map(|category| ClassificationCategoryInput {
            id: category.id().as_str().into(),
            label: category.label().into(),
            descriptions: category.descriptions().to_vec(),
        })
        .collect()
}

fn invalid(error: ClassificationError) -> ScanError {
    ScanError::new("classification_configuration_invalid", error.to_string())
}

#[cfg(test)]
mod tests;
