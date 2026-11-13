use std::collections::BTreeSet;

use crate::ports::classification_configuration::ClassificationNavigationRepository;

use super::*;

fn assert_navigation(catalog: &mut SqliteCatalog, expected: &[&str]) {
    let navigation = catalog.load_classification_navigation().unwrap();
    assert_eq!(
        navigation.populated_categories,
        expected.iter().map(|value| category(value)).collect(),
    );
    let definitions = navigation.configuration.catalog.taxonomy.unwrap();
    let from_gallery: BTreeSet<_> = definitions
        .categories()
        .iter()
        .filter(|definition| {
            let query = query(
                Some(definition.id().as_str()),
                ClassificationReviewFilter::All,
            );
            !ids(catalog, &query).is_empty()
        })
        .map(|definition| definition.id().clone())
        .collect();
    assert_eq!(navigation.populated_categories, from_gallery);
}

#[test]
fn navigation_tracks_publications_and_legacy_choices_without_removing_definitions() {
    let (directory, mut catalog) = fixture();
    let run = setup(&mut catalog);
    assert_navigation(&mut catalog, &[]);
    publish_group(&mut catalog, &["a", "c"], 'a');
    predict(&mut catalog, &run, "a", "first", "drawing");
    assert_navigation(&mut catalog, &["drawing"]);
    predict(&mut catalog, &run, "a", "replacement", "photo");
    assert_navigation(&mut catalog, &["photo"]);
    correct(&mut catalog, "choice", "c", "drawing");
    assert_navigation(&mut catalog, &["drawing"]);
    catalog
        .undo_classification(&id("undo"), &id("choice"))
        .unwrap();
    assert_navigation(&mut catalog, &["photo"]);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_navigation(&mut catalog, &["photo"]);
    assert_eq!(
        catalog
            .load_classification_navigation()
            .unwrap()
            .configuration
            .catalog
            .taxonomy
            .unwrap(),
        taxonomy()
    );
}

#[test]
fn a_late_conflict_hides_both_categories_until_local_authority_resolves_it() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    correct(&mut catalog, "a-choice", "a", "drawing");
    correct(&mut catalog, "c-choice", "c", "photo");
    assert_navigation(&mut catalog, &["drawing", "photo"]);
    publish_group(&mut catalog, &["a", "c"], 'a');
    assert_navigation(&mut catalog, &[]);
    publish_fixture_change(
        &mut catalog,
        "UPDATE classification_decisions SET choice_restored = 1 WHERE category_id = 'photo'",
        [],
    );
    assert_navigation(&mut catalog, &["drawing"]);
    correct(&mut catalog, "resolve", "c", "photo");
    assert_navigation(&mut catalog, &["photo"]);
}

#[test]
fn changed_content_and_removed_sources_stop_populating_categories() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    publish_fixture_change(
        &mut catalog,
        "UPDATE asset_locations SET file_size = 21 WHERE location_id = 'a'",
        [],
    );
    assert_navigation(&mut catalog, &["drawing"]);
    catalog.unregister_root("root-c").unwrap();
    assert_navigation(&mut catalog, &[]);
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        1
    );
}

#[test]
fn changed_rules_hide_incompatible_predictions_and_preserve_legacy_intent() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    correct(&mut catalog, "human", "b", "photo");
    assert_navigation(&mut catalog, &["drawing", "photo"]);
    let replacement = Taxonomy::new(
        taxonomy()
            .categories()
            .iter()
            .map(|definition| {
                CategoryDefinition::new(
                    definition.id().clone(),
                    definition.label().into(),
                    vec![format!("Changed purpose for {}", definition.label())],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let revision = catalog.load_classification_state().unwrap().revision;
    catalog
        .configure_classification(&id("rules"), revision, &replacement)
        .unwrap();
    assert_navigation(&mut catalog, &["photo"]);
    let revision = catalog.load_classification_state().unwrap().revision;
    catalog
        .configure_classification(
            &id("retire"),
            revision,
            &Taxonomy::new(vec![replacement.categories()[0].clone()]).unwrap(),
        )
        .unwrap();
    assert_navigation(&mut catalog, &[]);
}

#[test]
fn retained_gallery_evidence_keeps_navigation_without_granting_command_authority() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "first", "drawing");
    let old = selected(&catalog, "a");
    publish_fixture_change(
        &mut catalog,
        "UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'a'",
        [],
    );
    assert_navigation(&mut catalog, &[]);
    publish_fixture_change(
        &mut catalog,
        "INSERT INTO gallery_retained_fingerprints VALUES (?1, ?2, 'blake3-256-full-v1')",
        rusqlite::params![
            (old.source_generation + 100) as i64,
            old.source_generation as i64
        ],
    );
    assert_navigation(&mut catalog, &["drawing"]);
    assert_eq!(
        catalog
            .prepare_classification_selection(&[selected(&catalog, "a")])
            .unwrap_err()
            .code,
        "organization_evidence_pending"
    );
}

fn publish_fixture_change(
    catalog: &mut SqliteCatalog,
    sql: &str,
    parameters: impl rusqlite::Params,
) {
    let transaction = catalog.connection.transaction().unwrap();
    transaction.execute(sql, parameters).unwrap();
    operations::publish_catalog(&transaction).unwrap();
    transaction.commit().unwrap();
}
