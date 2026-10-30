use crate::domain::classification::{
    CategoryDefinition, CategoryId, CategoryScore, ClassificationReview, ClassificationRevision,
    HumanClassification, HumanDecisionOrigin, ModelCompatibility, RankedPrediction,
    ReviewRecommendation,
};
use crate::ports::organization::OrganizationRepository;

use super::super::organization::tests::{fixture, publish, selected};
use super::*;

mod archives;
mod authority;
mod lifecycle;
mod review;

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

fn category(value: &str) -> CategoryId {
    CategoryId::new(value.into()).unwrap()
}

fn taxonomy() -> Taxonomy {
    Taxonomy::new(
        ["drawing", "photo"]
            .map(|value| {
                CategoryDefinition::new(
                    category(value),
                    value.into(),
                    vec![format!("An image used as a {value}")],
                )
                .unwrap()
            })
            .into(),
    )
    .unwrap()
}

fn setup(catalog: &mut SqliteCatalog) -> ClassificationRun {
    let taxonomy = taxonomy();
    catalog
        .configure_classification(&id("configure"), 0, &taxonomy)
        .unwrap();
    let run = ClassificationRun::new(
        id("run"),
        ModelCompatibility::new(
            ClassificationRevision::from_bytes([1; 32]),
            ClassificationRevision::from_bytes([2; 32]),
            ClassificationRevision::from_bytes([3; 32]),
            2,
        )
        .unwrap(),
        taxonomy.revision(),
        None,
    )
    .unwrap();
    catalog.begin_classification_run(&run, None).unwrap();
    run
}

fn prepare(catalog: &SqliteCatalog, ids: &[&str]) -> PreparedClassificationSelection {
    catalog
        .prepare_classification_selection(
            &ids.iter()
                .map(|name| selected(catalog, name))
                .collect::<Vec<_>>(),
        )
        .unwrap()
}

fn predict(
    catalog: &mut SqliteCatalog,
    run: &ClassificationRun,
    image: &str,
    prediction_id: &str,
    winner: &str,
) -> PredictionEvidence {
    let observed = prepare(catalog, &[image]);
    let crate::domain::classification::ClassificationSubject::Exact(content) =
        &observed.groups[0].target.subject
    else {
        panic!("Expected exact fixture content")
    };
    let prediction = PredictionEvidence::new(
        id(prediction_id),
        run.clone(),
        content.clone(),
        RankedPrediction::new(
            run.primary_rules(),
            run.model(),
            ["drawing", "photo"]
                .map(|name| CategoryScore {
                    category_id: category(name),
                    similarity: if name == winner { 0.8 } else { 0.2 },
                })
                .into(),
        )
        .unwrap(),
        None,
        ReviewRecommendation::Uncalibrated,
    )
    .unwrap();
    assert!(
        catalog
            .publish_classification_prediction(&selected(catalog, image), &prediction)
            .unwrap()
    );
    prediction
}

fn scalar(catalog: &SqliteCatalog, query: &str) -> i64 {
    catalog
        .connection
        .query_row(query, [], |row| row.get(0))
        .unwrap()
}

#[test]
fn confirmation_survives_restart_without_prediction_cache_and_keeps_r4_state() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    let prepared = prepare(&catalog, &["c"]);
    let r4_revision = scalar(&catalog, "SELECT revision FROM organization_state");
    let receipt = catalog
        .change_classification(&id("confirm"), &prepared, &ClassificationAction::Confirm)
        .unwrap();
    assert_eq!(receipt.changed_groups, 1);
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM organization_state"),
        r4_revision
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM organization_operations"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM organization_records WHERE favorite = 1 OR disposition <> 'pending' OR album_id IS NOT NULL"
        ),
        0
    );
    catalog
        .connection
        .execute("DELETE FROM classification_predictions", [])
        .unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let current = prepare(&catalog, &["a", "c"]);
    assert_eq!(current.groups.len(), 1);
    assert_eq!(
        current.groups[0].effective().review,
        ClassificationReview::Confirmed
    );
    assert_eq!(
        current.groups[0].effective().category,
        Some(category("drawing"))
    );
    assert!(current.groups[0].prediction.is_none());
    assert_eq!(
        catalog
            .change_classification(&id("confirm"), &prepared, &ClassificationAction::Confirm)
            .unwrap(),
        receipt
    );
    assert!(
        catalog
            .change_classification(&id("confirm"), &prepared, &ClassificationAction::Defer)
            .is_err()
    );
}

#[test]
fn display_only_rename_retains_prediction_but_changed_descriptions_retire_it() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    let old = prepare(&catalog, &["a"]);
    let revision = catalog.load_classification_state().unwrap().revision;
    let renamed = Taxonomy::new(
        taxonomy()
            .categories()
            .iter()
            .map(|value| value.renamed(format!("Label {}", value.label())).unwrap())
            .collect(),
    )
    .unwrap();
    catalog
        .configure_classification(&id("rename"), revision, &renamed)
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].prediction,
        old.groups[0].prediction
    );
    let revision = catalog.load_classification_state().unwrap().revision;
    let changed = Taxonomy::new(
        renamed
            .categories()
            .iter()
            .map(|value| {
                CategoryDefinition::new(
                    value.id().clone(),
                    value.label().into(),
                    vec![format!("Another description for {}", value.label())],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    catalog
        .configure_classification(&id("descriptions"), revision, &changed)
        .unwrap();
    assert!(prepare(&catalog, &["a"]).groups[0].prediction.is_none());
    assert!(
        catalog
            .change_classification(&id("stale-confirm"), &old, &ClassificationAction::Confirm)
            .is_err()
    );
}
