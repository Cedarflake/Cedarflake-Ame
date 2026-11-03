use std::sync::{Arc, atomic::AtomicBool};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::maintenance::CategoryMaintenancePlan;
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::ports::classification_configuration::{
    CategoryMergeRepository, CategoryRetirementRepository,
};

use super::{
    NamingContextCatalog, check_cancelled, prepare_category_merge,
    prepare_equivalent_category_retirement, survey_category_coverage,
};

pub fn prepare_category_reduction(
    operation_id: ClassificationRecordId,
    repository: &(impl CategoryRetirementRepository + CategoryMergeRepository),
    contexts: Option<&NamingContextCatalog>,
    cancelled: &Arc<AtomicBool>,
) -> Result<Option<CategoryMaintenancePlan>, ScanError> {
    check_cancelled(cancelled)?;
    let snapshot = repository.load_category_retirement_snapshot()?;
    if snapshot.configuration.dispatch.mode != ClassificationDispatchMode::Automatic {
        return Err(ScanError::new(
            "classification_changed",
            "Category maintenance requires the current automatic configuration",
        ));
    }
    let Some(taxonomy) = &snapshot.configuration.catalog.taxonomy else {
        return Ok(None);
    };
    if taxonomy
        .categories()
        .iter()
        .all(|category| snapshot.protects(category))
    {
        return Ok(None);
    }
    if let Some(plan) =
        prepare_equivalent_category_retirement(operation_id.clone(), repository, cancelled)?
    {
        return Ok(Some(CategoryMaintenancePlan::Retirement(Box::new(plan))));
    }
    let coverage = survey_category_coverage(repository, 64, 64, cancelled)?;
    let plan = prepare_category_merge(operation_id, coverage, contexts, repository, cancelled)?;
    Ok(plan.map(|plan| CategoryMaintenancePlan::Merge(Box::new(plan))))
}
