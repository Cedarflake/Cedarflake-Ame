use std::sync::{Arc, atomic::AtomicBool};

use crate::application::classification_discovery::survey_category_coverage;
use crate::domain::classification::maintenance::CategoryCoverageSummary;
use crate::ports::classification_configuration::CategoryCoverageRepository;
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::*;

fn start(catalog: &mut SqliteCatalog) -> ClassificationRun {
    for (name, digest) in [("a", 'a'), ("b", 'a'), ("c", 'c')] {
        publish(catalog, name, digest);
    }
    let generations = ["a", "b", "c"].map(|name| selected(catalog, name).source_generation);
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
    let enable = request(
        catalog,
        "enable",
        taxonomy(),
        ClassificationConfigurationAction::Enable,
    );
    catalog.save_classification_configuration(&enable).unwrap();
    enable.setup.run().clone()
}

fn survey(catalog: &SqliteCatalog, page: usize) -> CategoryCoverageSummary {
    survey_category_coverage(catalog, page, 2, &Arc::new(AtomicBool::new(false))).unwrap()
}

#[test]
fn complete_coverage_counts_exact_contents_and_reuses_compatible_predictions_after_restart() {
    let (directory, mut catalog) = fixture();
    let run = start(&mut catalog);
    predict(&mut catalog, &run, "a", "a-first", "drawing");
    predict(&mut catalog, &run, "c", "c-first", "photo");
    let before = survey(&catalog, 1);
    assert!(before.is_complete());
    assert_eq!(before.classified_contents, 2);
    assert_eq!(before.categories[&category("drawing")].contents, 1);
    assert_eq!(before.categories[&category("photo")].contents, 1);
    assert_eq!(survey(&catalog, 64).evidence, before.evidence);
    predict(&mut catalog, &run, "b", "a-later", "photo");
    let latest = survey(&catalog, 1);
    assert_eq!(latest.categories[&category("photo")].contents, 2);
    assert_ne!(latest.evidence, before.evidence);
    let current = catalog.load_classification_configuration().unwrap();
    catalog
        .change_classification_dispatch(
            current.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    let replacement = request(
        &catalog,
        "compatible",
        taxonomy(),
        ClassificationConfigurationAction::Enable,
    );
    catalog
        .save_classification_configuration(&replacement)
        .unwrap();
    assert_eq!(
        catalog
            .load_classification_configuration()
            .unwrap()
            .catalog
            .active_run_id
            .as_deref(),
        Some("compatible")
    );
    assert_eq!(survey(&catalog, 2).categories, latest.categories);
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert!(survey(&catalog, 1).is_complete());
    assert_eq!(survey(&catalog, 1).categories, latest.categories);
}

#[test]
fn missing_analysis_or_direct_source_proof_never_reports_complete_coverage() {
    let (_directory, mut catalog) = fixture();
    let run = start(&mut catalog);
    let initial = survey(&catalog, 1);
    assert!(!initial.is_complete());
    assert_eq!(initial.unclassified_contents, 2);
    predict(&mut catalog, &run, "a", "a-first", "drawing");
    assert_eq!(survey(&catalog, 2).unclassified_contents, 1);
    let generation = selected(&catalog, "c").source_generation;
    catalog
        .connection
        .execute(
            "DELETE FROM gallery_content_fingerprints WHERE source_generation = ?1",
            [i64::try_from(generation).unwrap()],
        )
        .unwrap();
    let unpublished = survey(&catalog, 2);
    assert!(unpublished.snapshot.unverified_sources);
    assert!(!unpublished.is_complete());
    assert_eq!(unpublished.unclassified_contents, 0);
    catalog
        .publish_exact_analysis_batch("organization-fixture", &[generation])
        .unwrap();
    predict(&mut catalog, &run, "c", "c-first", "photo");
    assert!(survey(&catalog, 2).is_complete());
    let source = selected(&catalog, "c");
    assert_eq!(catalog.connection.execute(
        "UPDATE asset_locations SET source_revision_token = 'changed' WHERE location_id = ?1",
        [&source.location_id],
    ).unwrap(), 1);
    let stale = survey(&catalog, 1);
    assert!(stale.snapshot.unverified_sources);
    assert!(!stale.is_complete());
    assert_eq!(stale.classified_contents, 1);
}

#[test]
fn publication_and_dispatch_changes_reject_retained_coverage_authority() {
    let (_directory, mut catalog) = fixture();
    let run = start(&mut catalog);
    let cancelled = Arc::new(AtomicBool::new(false));
    let before = catalog.begin_category_coverage(&cancelled).unwrap();
    predict(&mut catalog, &run, "a", "a-first", "drawing");
    assert_eq!(
        catalog
            .read_category_coverage(&before, None, 1, &cancelled)
            .unwrap_err()
            .code,
        "classification_changed"
    );
    assert_eq!(
        catalog
            .validate_category_coverage(&before, &cancelled)
            .unwrap_err()
            .code,
        "classification_changed"
    );
    let before_pause = catalog.begin_category_coverage(&cancelled).unwrap();
    catalog
        .change_classification_dispatch(
            before_pause.configuration.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    assert_eq!(
        catalog
            .read_category_coverage(&before_pause, None, 1, &cancelled)
            .unwrap_err()
            .code,
        "classification_changed"
    );
    assert_eq!(
        catalog
            .begin_category_coverage(&Arc::new(AtomicBool::new(true)))
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
    assert!(catalog.connection.is_autocommit());
}

#[test]
fn malformed_or_mismatched_prediction_records_are_errors_not_empty_categories() {
    let (_directory, mut catalog) = fixture();
    let run = start(&mut catalog);
    predict(&mut catalog, &run, "a", "a-first", "drawing");
    let payload: String = catalog
        .connection
        .query_row(
            "SELECT payload FROM classification_predictions WHERE prediction_id = 'a-first'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut wrong_content: serde_json::Value = serde_json::from_str(&payload).unwrap();
    wrong_content["content"]["digest"] = serde_json::Value::String("d".repeat(64));
    for invalid in ["{}".to_owned(), wrong_content.to_string()] {
        catalog.connection.execute("UPDATE classification_predictions SET payload = ?1 WHERE prediction_id = 'a-first'", [invalid]).unwrap();
        assert!(
            survey_category_coverage(&catalog, 1, 1, &Arc::new(AtomicBool::new(false))).is_err()
        );
        assert!(catalog.connection.is_autocommit());
    }
}

#[test]
fn changed_category_rules_leave_prior_predictions_unclassified() {
    let (_directory, mut catalog) = fixture();
    let run = start(&mut catalog);
    predict(&mut catalog, &run, "a", "a-first", "drawing");
    predict(&mut catalog, &run, "c", "c-first", "photo");
    let changed = Taxonomy::new(
        taxonomy()
            .categories()
            .iter()
            .map(|category| {
                CategoryDefinition::new(
                    category.id().clone(),
                    category.label().into(),
                    vec![format!("changed {}", category.label())],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let change = request(
        &catalog,
        "changed",
        changed,
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&change).unwrap();
    let report = survey(&catalog, 1);
    assert!(!report.is_complete());
    assert_eq!(report.classified_contents, 0);
    assert_eq!(report.unclassified_contents, 2);
}
