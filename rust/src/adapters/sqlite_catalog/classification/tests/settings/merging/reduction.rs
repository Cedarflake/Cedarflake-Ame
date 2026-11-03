use crate::application::classification_discovery::prepare_category_reduction;
use crate::domain::classification::maintenance::CategoryMaintenancePlan;
use crate::ports::classification_configuration::CategoryRetirementRepository;

use super::*;

#[test]
fn reduction_retires_equivalent_definitions_before_requiring_content_coverage() {
    let (_directory, mut catalog) = super::super::super::fixture();
    super::super::retirement::seed(&mut catalog, 3);
    let cancelled = Arc::new(AtomicBool::new(false));
    let before = catalog.load_classification_configuration().unwrap();
    let Some(CategoryMaintenancePlan::Retirement(plan)) =
        prepare_category_reduction(id("reduce"), &catalog, None, &cancelled).unwrap()
    else {
        panic!("expected equivalent-category retirement");
    };
    assert_eq!(plan.retirements().len(), 2);
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    let saved = catalog
        .apply_category_retirement(&plan, &cancelled)
        .unwrap();
    assert_eq!(saved.catalog.taxonomy.unwrap().categories().len(), 2);
    assert!(
        prepare_category_reduction(id("again"), &catalog, None, &cancelled)
            .unwrap()
            .is_none()
    );
}

#[test]
fn reduction_retains_both_sparse_members_and_the_original_publication_guards() {
    let (_directory, mut catalog) = super::super::super::fixture();
    let child = sparse(&mut catalog);
    let cancelled = Arc::new(AtomicBool::new(false));
    let Some(CategoryMaintenancePlan::Merge(plan)) =
        prepare_category_reduction(id("reduce"), &catalog, None, &cancelled).unwrap()
    else {
        panic!("expected a two-member purpose merge");
    };
    assert_eq!(plan.category_id(), &child);
    assert_eq!(plan.retained_id(), &category("drawing"));
    let before = catalog.load_classification_configuration().unwrap();
    catalog
        .change_classification_dispatch(
            before.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    assert_eq!(
        catalog
            .apply_category_merge(&plan, &cancelled)
            .unwrap_err()
            .code,
        "classification_changed"
    );
    assert!(
        catalog
            .load_classification_setup(&id("reduce"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn reduction_preserves_recurrent_incomplete_and_manually_protected_categories() {
    let (_directory, mut catalog) = super::super::super::fixture();
    let (run, child) = start(&mut catalog);
    let cancelled = Arc::new(AtomicBool::new(false));
    assert!(
        prepare_category_reduction(id("recurrent"), &catalog, None, &cancelled)
            .unwrap()
            .is_none()
    );
    predict_child(&mut catalog, &run, &child, "c", "new-purpose", false);
    catalog
        .connection
        .execute(
            "UPDATE classification_managed_categories SET protected = 1",
            [],
        )
        .unwrap();
    assert!(
        prepare_category_reduction(id("protected"), &catalog, None, &cancelled)
            .unwrap()
            .is_none()
    );
    catalog
        .connection
        .execute(
            "UPDATE classification_managed_categories SET protected = 0",
            [],
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_revision_token = 'changed' WHERE location_id = ?1",
            [&selected(&catalog, "b").location_id],
        )
        .unwrap();
    assert!(!survey(&catalog).is_complete());
    assert!(
        prepare_category_reduction(id("incomplete"), &catalog, None, &cancelled)
            .unwrap()
            .is_none()
    );
}

#[test]
fn reduction_rejects_cancelled_and_paused_preparation() {
    let (_directory, mut catalog) = super::super::super::fixture();
    sparse(&mut catalog);
    let cancelled = Arc::new(AtomicBool::new(true));
    assert_eq!(
        prepare_category_reduction(id("cancelled"), &catalog, None, &cancelled)
            .err()
            .unwrap()
            .code,
        "classification_cancelled"
    );
    cancelled.store(false, Ordering::Release);
    let before = catalog.load_classification_configuration().unwrap();
    catalog
        .change_classification_dispatch(
            before.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    assert_eq!(
        prepare_category_reduction(id("paused"), &catalog, None, &cancelled)
            .err()
            .unwrap()
            .code,
        "classification_changed"
    );
    assert!(
        catalog
            .load_classification_setup(&id("paused"))
            .unwrap()
            .is_none()
    );
}
