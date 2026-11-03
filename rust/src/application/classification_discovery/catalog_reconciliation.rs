use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryCatalogReconciliationPlan, ManagedCategoryCatalog, catalog_retirements,
};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{
    ClassificationAnalysisSetup, ClassificationRecordId, ClassificationRun, Taxonomy,
};
use crate::domain::classification_configuration::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
};
use crate::ports::classification_configuration::CategoryRetirementRepository;

use super::check_cancelled;

pub fn prepare_category_catalog_reconciliation(
    operation_id: ClassificationRecordId,
    catalog: &ManagedCategoryCatalog,
    repository: &impl CategoryRetirementRepository,
    cancelled: &AtomicBool,
) -> Result<Option<CategoryCatalogReconciliationPlan>, ScanError> {
    check_cancelled(cancelled)?;
    let before = repository.load_category_retirement_snapshot()?;
    if before.configuration.dispatch.mode != ClassificationDispatchMode::Automatic {
        return Err(changed());
    }
    let retired = catalog_retirements(&before, catalog, cancelled)?;
    if retired.is_empty() {
        return Ok(None);
    }
    let model = before.model.ok_or_else(changed)?;
    let current = before
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(changed)?;
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
    Ok(Some(CategoryCatalogReconciliationPlan {
        before,
        catalog: catalog.clone(),
        configuration,
        retired,
    }))
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_changed",
        "The category catalog authority changed",
    )
}
