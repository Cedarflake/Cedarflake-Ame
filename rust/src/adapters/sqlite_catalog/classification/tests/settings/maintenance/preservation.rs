use std::sync::{Arc, atomic::Ordering};

use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::*;

#[test]
fn category_addition_preserves_human_choices_and_independent_favorite_changes() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let observed = super::super::super::prepare(&catalog, &["a"]);
    catalog
        .change_classification(
            &id("correct"),
            &observed,
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    let human = super::super::super::prepare(&catalog, &["a"]).groups[0]
        .human
        .clone();
    let plan = plan(&catalog, &evidence(&catalog));
    let selection = catalog
        .prepare_organization_selection(&[selected(&catalog, "a")])
        .unwrap();
    catalog
        .set_organization_favorite("favorite", &selection, true)
        .unwrap();
    catalog
        .apply_category_expansion(&plan, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        super::super::super::prepare(&catalog, &["a"]).groups[0].human,
        human
    );
    assert!(
        catalog
            .inspect_organization_selection(&[selected(&catalog, "a")])
            .unwrap()
            .decisions[0]
            .favorite
    );
}

#[test]
fn cancellation_after_run_insert_rolls_back_the_entire_expansion() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let plan = plan(&catalog, &evidence(&catalog));
    let before = catalog.load_classification_configuration().unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancellation = cancelled.clone();
    catalog
        .connection
        .update_hook(Some(
            move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                if table == "classification_runs" {
                    cancellation.store(true, Ordering::Release);
                }
            },
        ))
        .unwrap();
    assert_eq!(
        catalog
            .apply_category_expansion(&plan, &cancelled)
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
    assert!(cancelled.load(Ordering::Acquire));
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    assert!(
        catalog
            .load_category_expansion_snapshot(plan.additions())
            .unwrap()
            .known_categories
            .is_empty()
    );
    assert!(
        catalog
            .load_classification_setup(&id("expand"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn another_current_verified_copy_keeps_comparison_evidence_valid() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let plan = plan(&catalog, &evidence(&catalog));
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-d",
        "root-d",
        "E:\\Fixture",
        &[("d", None, 4)],
    );
    catalog.connection.execute_batch(
        "INSERT INTO library_root_publication_namespaces
             (root_id, root_generation, identity_scheme, identity_value, authority_kind,
              established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1',
             '0000000000000001:00000000000000000000000000000001', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state WHERE root_id = 'root-d';
         UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001'
         WHERE location_id = 'd';"
    ).unwrap();
    publish(&mut catalog, "d", 'c');
    let generation = selected(&catalog, "d").source_generation;
    catalog
        .publish_exact_analysis_batch("organization-fixture", &[generation])
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "UPDATE library_change_root_state SET is_active = 0 WHERE root_id = 'root-c'",
        )
        .unwrap();
    assert!(
        catalog
            .apply_category_expansion(&plan, &AtomicBool::new(false))
            .is_ok()
    );
}

#[test]
fn full_taxonomy_defers_additions_without_dropping_existing_categories() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let evidence = evidence(&catalog);
    let mut categories = edited_rules("Drawing", "baseline").categories().to_vec();
    categories.extend((1..256).map(|index| {
        CategoryDefinition::new(
            category(&format!("manual-{index}")),
            format!("Collection {index}"),
            vec!["baseline".into()],
        )
        .unwrap()
    }));
    let save = request(
        &catalog,
        "full",
        Taxonomy::new(categories).unwrap(),
        ClassificationConfigurationAction::Save,
    );
    let before = catalog.save_classification_configuration(&save).unwrap();
    let outcome = assess(&catalog, &evidence, "at-capacity").unwrap();
    assert!(outcome.plan.is_none());
    assert_eq!(outcome.capacity_deferred.len(), 1);
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
}

#[test]
fn old_encoder_evidence_is_rejected_after_model_replacement() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let evidence = evidence(&catalog);
    let mut save = request(
        &catalog,
        "model-change",
        edited_rules("Drawing", "baseline"),
        ClassificationConfigurationAction::Save,
    );
    let model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([91; 32]),
        ClassificationRevision::from_bytes([92; 32]),
        ClassificationRevision::from_bytes([93; 32]),
        2,
    )
    .unwrap();
    let rules = save.setup.primary().clone();
    let run = ClassificationRun::new(id("model-change"), model, rules.revision(), None).unwrap();
    save.setup = ClassificationAnalysisSetup::new(run, rules, None).unwrap();
    let before = catalog.save_classification_configuration(&save).unwrap();
    assert_eq!(
        assess(&catalog, &evidence, "old-model").err().unwrap().code,
        "classification_changed"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
}
