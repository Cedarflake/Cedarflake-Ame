use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationAnalysisSetup, ClassificationRecordId, ModelCompatibility,
};
use crate::domain::classification_configuration::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
};
use crate::ports::classification_configuration::ClassificationConfigurationRepository;

pub(super) fn replace_model(
    repository: &mut impl ClassificationConfigurationRepository,
    previous: ClassificationAnalysisSetup,
    replacement: ModelCompatibility,
    session: &ClassificationRecordId,
) -> Result<ClassificationAnalysisSetup, ScanError> {
    if previous.run().model() == replacement {
        return Ok(previous);
    }
    let snapshot = repository.load_classification_configuration()?;
    if snapshot.dispatch.run_id.as_ref() != Some(previous.run().id()) {
        return Err(super::setup::mismatch());
    }
    let rules = snapshot
        .catalog
        .taxonomy
        .ok_or_else(super::setup::mismatch)?;
    if rules.revision() != previous.primary().revision() {
        return Err(super::setup::mismatch());
    }
    let operation = ClassificationRecordId::new(format!("compute-{}", session.as_str()))
        .map_err(|_| super::setup::mismatch())?;
    let setup =
        super::super::classification_candidate::prepare(operation.clone(), rules, replacement)
            .map_err(|_| super::setup::mismatch())?;
    repository.save_classification_configuration(&ClassificationConfigurationChange {
        operation_id: operation,
        expected_catalog_revision: snapshot.catalog.revision,
        expected_dispatch_revision: snapshot.dispatch.revision,
        setup: setup.clone(),
        action: ClassificationConfigurationAction::Save,
    })?;
    Ok(setup)
}

#[cfg(test)]
mod tests;
