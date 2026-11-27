use crate::domain::classification::ClassificationSubject;
use crate::domain::classification_review::ClassificationReviewFilter;
use crate::ports::classification_configuration::ClassificationNavigationRepository;
use crate::ports::classification_selection::ClassificationSelectionReader;

use super::super::super::queries::{ids, publish_group, query};
use super::*;

fn start(catalog: &mut SqliteCatalog, rules: Taxonomy) -> ClassificationAnalysisSetup {
    let change = request(
        catalog,
        "start",
        rules,
        ClassificationConfigurationAction::Enable,
    );
    catalog.save_classification_configuration(&change).unwrap();
    change.setup
}

fn current_setup(catalog: &SqliteCatalog) -> ClassificationAnalysisSetup {
    let snapshot = catalog.load_classification_configuration().unwrap();
    catalog
        .load_classification_setup(&id(snapshot.catalog.active_run_id.as_deref().unwrap()))
        .unwrap()
        .unwrap()
}

fn publish_prediction(
    catalog: &mut SqliteCatalog,
    setup: &ClassificationAnalysisSetup,
    image: &str,
    prediction_id: &str,
    winner: &str,
) -> bool {
    let observed = prepare(catalog, &[image]);
    let ClassificationSubject::Exact(content) = &observed.groups[0].target.subject else {
        panic!("Expected verified fixture content");
    };
    let ranking = RankedPrediction::new(
        setup.primary().revision(),
        setup.run().model(),
        setup
            .primary()
            .categories()
            .iter()
            .map(|definition| CategoryScore {
                category_id: definition.id().clone(),
                similarity: if definition.id().as_str() == winner {
                    0.8
                } else {
                    0.2
                },
            })
            .collect(),
    )
    .unwrap();
    let evidence = PredictionEvidence::new(
        id(prediction_id),
        setup.run().clone(),
        content.clone(),
        ranking,
        None,
        ReviewRecommendation::Uncalibrated,
    )
    .unwrap();
    catalog
        .publish_classification_prediction(&selected(catalog, image), &evidence)
        .unwrap()
}

fn replace_description(catalog: &mut SqliteCatalog, operation: &str) {
    let before = catalog.load_classification_configuration().unwrap();
    let original = before.catalog.taxonomy.unwrap().categories()[0].clone();
    let replacement = CategoryDefinition::new(
        original.id().clone(),
        original.label().into(),
        vec![format!("A drawing described by {operation}")],
    )
    .unwrap();
    let change = edit(catalog, operation, Some(original), Some(replacement));
    catalog.save_classification_term(&change).unwrap();
}

fn members(catalog: &mut SqliteCatalog, category: &str) -> Vec<String> {
    ids(
        catalog,
        &query(Some(category), ClassificationReviewFilter::All),
    )
}

fn displayed(
    catalog: &SqliteCatalog,
    image: &str,
) -> crate::domain::classification_selection::ClassificationItemState {
    catalog
        .classification_items(&[selected(catalog, image)])
        .unwrap()
        .remove(0)
}

#[test]
fn paused_term_edits_retain_display_across_reopen_and_partial_rematching_without_command_authority()
{
    let (directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let initial = start(&mut catalog, taxonomy());
    assert!(publish_prediction(
        &mut catalog,
        &initial,
        "a",
        "a-first",
        "drawing"
    ));
    assert!(publish_prediction(
        &mut catalog,
        &initial,
        "b",
        "b-first",
        "drawing"
    ));
    let old_selection = prepare(&catalog, &["a"]);
    let dispatch = catalog
        .load_classification_configuration()
        .unwrap()
        .dispatch;
    catalog
        .change_classification_dispatch(dispatch.revision, &ClassificationDispatchCommand::Pause)
        .unwrap();
    replace_description(&mut catalog, "edit-one");
    assert_eq!(members(&mut catalog, "drawing"), ["a", "b"]);
    assert_eq!(
        catalog
            .load_classification_navigation()
            .unwrap()
            .populated_categories,
        [category("drawing")].into()
    );
    let mut indexed = query(Some("drawing"), ClassificationReviewFilter::All);
    indexed.root_id = Some("root-a".into());
    assert_eq!(ids(&mut catalog, &indexed), ["a", "b"]);
    assert_eq!(
        displayed(&catalog, "a").category_id.as_deref(),
        Some("drawing")
    );
    assert!(!displayed(&catalog, "a").can_confirm);
    assert!(
        prepare(&catalog, &["a"]).groups[0]
            .compatible_prediction()
            .is_none()
    );
    assert!(
        catalog
            .change_classification(
                &id("stale-confirm"),
                &old_selection,
                &ClassificationAction::Confirm
            )
            .is_err()
    );
    let command_projection =
        crate::adapters::sqlite_catalog::gallery::classification::command_projection("locations");
    let command_category: Option<String> = catalog
        .connection
        .query_row(
            &format!(
                "SELECT ({command_projection} SELECT category_id FROM effective)
          FROM asset_locations AS locations WHERE location_id = 'a'"
            ),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(command_category, None);
    assert!(!publish_prediction(
        &mut catalog,
        &initial,
        "a",
        "late-old-run",
        "photo"
    ));
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        catalog
            .load_classification_configuration()
            .unwrap()
            .dispatch
            .mode,
        ClassificationDispatchMode::Paused
    );
    assert_eq!(members(&mut catalog, "drawing"), ["a", "b"]);
    let replacement = current_setup(&catalog);
    assert!(publish_prediction(
        &mut catalog,
        &replacement,
        "a",
        "a-current",
        "photo"
    ));
    assert_eq!(members(&mut catalog, "drawing"), ["b"]);
    assert_eq!(members(&mut catalog, "photo"), ["a"]);
    assert!(displayed(&catalog, "a").can_confirm);
    replace_description(&mut catalog, "edit-two");
    assert_eq!(members(&mut catalog, "drawing"), ["b"]);
    assert_eq!(members(&mut catalog, "photo"), ["a"]);
    assert!(!displayed(&catalog, "a").can_confirm);
    let transaction = catalog.connection.transaction().unwrap();
    transaction
        .execute(
            "UPDATE asset_locations SET file_size = 21 WHERE location_id = 'b'",
            [],
        )
        .unwrap();
    operations::publish_catalog(&transaction).unwrap();
    transaction.commit().unwrap();
    assert!(members(&mut catalog, "drawing").is_empty());
    assert_eq!(members(&mut catalog, "photo"), ["a"]);
}

#[test]
fn deletion_readdition_and_encoder_replacement_do_not_reauthorize_retired_display() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    let custom = custom();
    let mut categories = taxonomy().categories().to_vec();
    categories.push(custom.clone());
    let initial = start(&mut catalog, Taxonomy::new(categories).unwrap());
    assert!(publish_prediction(
        &mut catalog,
        &initial,
        "a",
        "custom-first",
        "custom-travel"
    ));
    let delete = edit(&catalog, "delete-custom", Some(custom.clone()), None);
    catalog.save_classification_term(&delete).unwrap();
    assert!(members(&mut catalog, "custom-travel").is_empty());
    let add = edit(&catalog, "readd-custom", None, Some(custom));
    catalog.save_classification_term(&add).unwrap();
    assert_eq!(
        current_setup(&catalog).run().configuration(),
        initial.run().configuration()
    );
    assert!(members(&mut catalog, "custom-travel").is_empty());
    assert_eq!(displayed(&catalog, "a").category_id, None);
    assert!(!displayed(&catalog, "a").can_confirm);
    let replacement = current_setup(&catalog);
    assert!(publish_prediction(
        &mut catalog,
        &replacement,
        "a",
        "custom-current",
        "custom-travel"
    ));
    replace_description(&mut catalog, "retain-custom");
    assert_eq!(members(&mut catalog, "custom-travel"), ["a"]);
    let observed = prepare(&catalog, &["b"]);
    catalog
        .change_classification(
            &id("human-photo"),
            &observed,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    let rules = before.catalog.taxonomy.unwrap();
    let mut change = request(
        &catalog,
        "other-encoder",
        rules.clone(),
        ClassificationConfigurationAction::Save,
    );
    let model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([9; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        2,
    )
    .unwrap();
    let run = ClassificationRun::new(id("other-encoder"), model, rules.revision(), None).unwrap();
    change.setup = ClassificationAnalysisSetup::new(run, rules, None).unwrap();
    catalog.save_classification_configuration(&change).unwrap();
    assert!(members(&mut catalog, "custom-travel").is_empty());
    assert_eq!(displayed(&catalog, "a").category_id, None);
    assert_eq!(members(&mut catalog, "photo"), ["b"]);
    assert!(!publish_prediction(
        &mut catalog,
        &replacement,
        "a",
        "late-incompatible",
        "custom-travel"
    ));
}

#[test]
fn reset_batch_retains_results_and_conflict_rolls_back_display_state() {
    let defaults = taxonomy()
        .categories()
        .iter()
        .map(|category| (category.id().clone(), category.clone()))
        .collect();
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    let initial = start(&mut catalog, taxonomy());
    assert!(publish_prediction(
        &mut catalog,
        &initial,
        "a",
        "first",
        "drawing"
    ));
    replace_description(&mut catalog, "edit-drawing");
    let before = catalog.load_classification_configuration().unwrap();
    let expected = before.catalog.taxonomy.as_ref().unwrap().categories()[0].clone();
    let baseline = request(
        &catalog,
        "reset-display",
        taxonomy(),
        ClassificationConfigurationAction::Save,
    )
    .setup;
    let reset = crate::domain::classification_configuration::ClassificationTermsReset::new(
        baseline,
        vec![expected.clone()],
        &defaults,
    )
    .unwrap();
    replace_description(&mut catalog, "concurrent-edit");
    let generation = scalar(
        &catalog,
        "SELECT generation FROM classification_display_state",
    );
    assert_eq!(
        catalog
            .reset_all_classification_terms(&reset)
            .unwrap_err()
            .code,
        "classification_term_changed"
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT generation FROM classification_display_state"
        ),
        generation
    );
    assert_eq!(members(&mut catalog, "drawing"), ["a"]);
    let current = catalog.load_classification_configuration().unwrap();
    let expected = current.catalog.taxonomy.unwrap().categories()[0].clone();
    let baseline = request(
        &catalog,
        "reset-display-current",
        taxonomy(),
        ClassificationConfigurationAction::Save,
    )
    .setup;
    let reset = crate::domain::classification_configuration::ClassificationTermsReset::new(
        baseline,
        vec![expected],
        &defaults,
    )
    .unwrap();
    catalog.reset_all_classification_terms(&reset).unwrap();
    assert_eq!(members(&mut catalog, "drawing"), ["a"]);
    assert_eq!(
        displayed(&catalog, "a").category_id.as_deref(),
        Some("drawing")
    );
}

#[test]
fn schema_upgrade_backfills_only_current_admitted_results_before_term_edit() {
    let (directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    publish_group(&mut catalog, &["b"], 'b');
    let initial = start(&mut catalog, taxonomy());
    assert!(publish_prediction(
        &mut catalog,
        &initial,
        "a",
        "old",
        "drawing"
    ));
    let mut next = taxonomy().categories().to_vec();
    next[0] = CategoryDefinition::new(
        category("drawing"),
        "drawing".into(),
        vec!["Changed drawing description".into()],
    )
    .unwrap();
    let change = request(
        &catalog,
        "different-rules",
        Taxonomy::new(next).unwrap(),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&change).unwrap();
    assert!(publish_prediction(
        &mut catalog,
        &change.setup,
        "b",
        "current",
        "photo"
    ));
    let snapshot = catalog.load_classification_configuration().unwrap();
    catalog
        .connection
        .execute_batch(
            "DROP VIEW classification_display_projection;
         DROP VIEW classification_current_display_projection;
         DROP VIEW classification_retained_display_projection;
         DROP TABLE classification_display_predictions;
         DROP TABLE classification_display_category_floors;
         DROP TABLE classification_display_state;
         UPDATE schema_info SET version = 83;
         PRAGMA user_version = 83;",
        )
        .unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        catalog.load_classification_configuration().unwrap(),
        snapshot
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM classification_predictions"),
        2
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM classification_display_predictions"
        ),
        1
    );
    assert!(members(&mut catalog, "drawing").is_empty());
    assert_eq!(members(&mut catalog, "photo"), ["b"]);
    replace_description(&mut catalog, "after-upgrade");
    assert!(members(&mut catalog, "drawing").is_empty());
    assert_eq!(members(&mut catalog, "photo"), ["b"]);
}
