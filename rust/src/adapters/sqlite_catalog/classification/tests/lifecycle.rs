use super::*;
use crate::domain::organization_cleanup::OrganizationCleanupRequest;
use crate::ports::CatalogRepository;
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::organization_cleanup::OrganizationCleanupRepository;

#[test]
fn late_grouping_retains_human_conflict_until_explicit_whole_group_choice() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    catalog
        .change_classification(
            &id("choose-a"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    catalog
        .change_classification(
            &id("choose-c"),
            &prepare(&catalog, &["c"]),
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    assert!(
        catalog
            .prepare_classification_selection(&[selected(&catalog, "a")])
            .is_err()
    );
    let generations = [
        selected(&catalog, "a").source_generation,
        selected(&catalog, "c").source_generation,
    ];
    catalog
        .publish_exact_analysis_batch("organization-fixture", &generations)
        .unwrap();
    let merged = prepare(&catalog, &["a", "c"]);
    assert_eq!(merged.groups.len(), 1);
    assert_eq!(merged.groups[0].human, HumanClassification::Conflict);
    assert!(
        catalog
            .undo_classification(&id("old-undo"), &id("choose-a"))
            .is_err()
    );
    catalog
        .change_classification(
            &id("resolve"),
            &merged,
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["c"]).groups[0].human,
        HumanClassification::Chosen {
            category: category("photo"),
            origin: HumanDecisionOrigin::Local,
        }
    );
    catalog
        .undo_classification(&id("undo-resolution"), &id("resolve"))
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Conflict
    );
}

#[test]
fn content_publication_can_gain_an_unhandled_copy_but_old_human_target_is_stale() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let run = setup(&mut catalog);
    let evidence = predict(&mut catalog, &run, "a", "first", "drawing");
    let old = prepare(&catalog, &["a"]);
    publish(&mut catalog, "c", 'a');
    assert!(
        catalog
            .change_classification(&id("old-target"), &old, &ClassificationAction::Confirm)
            .is_err()
    );
    assert_eq!(
        prepare(&catalog, &["c"]).groups[0].prediction,
        Some(evidence.proposal())
    );
    catalog
        .change_classification(
            &id("confirm-current"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Confirm,
        )
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["c"]).groups[0].effective().review,
        ClassificationReview::Confirmed
    );
}

#[test]
fn explicit_source_removal_retains_surviving_group_but_reimport_cannot_resurrect_removed_choices() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    setup(&mut catalog);
    catalog
        .change_classification(
            &id("choose"),
            &prepare(&catalog, &["a", "b"]),
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    catalog.unregister_root("root-a").unwrap();
    assert_eq!(
        prepare(&catalog, &["c"]).groups[0].effective().category,
        Some(category("drawing"))
    );
    assert!(
        catalog
            .undo_classification(&id("partial-undo"), &id("choose"))
            .is_err()
    );
    catalog.unregister_root("root-c").unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_decisions"),
        0
    );
    assert!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_operation_targets"
        ) > 0
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "reimport",
        "root-a",
        "C:\\Fixture",
        &[("a", None, 1)],
    );
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Unreviewed
    );
}

#[test]
fn unresolved_cleanup_counts_classification_and_retires_undo_and_reimport_authority() {
    let (directory, mut catalog) = fixture();
    setup(&mut catalog);
    catalog
        .change_classification(
            &id("choose"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    catalog.connection.execute_batch("DELETE FROM asset_locations WHERE location_id = 'a'; UPDATE organization_records SET unassociated_since_unix_ms = 100;").unwrap();
    let inspection = catalog
        .inspect_organization_cleanup(Some(500), &mut |_| true)
        .unwrap();
    assert_eq!(inspection.groups, 1);
    assert_eq!(inspection.classification_groups, 1);
    assert_eq!(inspection.progress_groups, 0);
    catalog
        .apply_organization_cleanup(
            &OrganizationCleanupRequest {
                operation_id: "cleanup".into(),
                inspection,
            },
            &mut |_| true,
        )
        .unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_decisions"),
        0
    );
    assert!(
        catalog
            .undo_classification(&id("cleaned-undo"), &id("choose"))
            .is_err()
    );
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "reimport",
        "root-a",
        "C:\\Fixture",
        &[("a", None, 1)],
    );
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Unreviewed
    );
}

#[test]
fn changed_content_detaches_a_choice_and_matching_content_adopts_the_destination_group() {
    let (_directory, mut catalog) = fixture();
    for (name, digest) in [("a", 'a'), ("c", 'a'), ("b", 'b')] {
        publish(&mut catalog, name, digest);
    }
    setup(&mut catalog);
    catalog
        .change_classification(
            &id("first"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    catalog
        .change_classification(
            &id("other"),
            &prepare(&catalog, &["b"]),
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    let old = prepare(&catalog, &["c"]);
    for (digest, expected) in [('c', None), ('b', Some(category("photo")))] {
        catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'c'", []).unwrap();
        publish(&mut catalog, "c", digest);
        let generation = selected(&catalog, "c").source_generation;
        catalog
            .publish_exact_analysis_batch("organization-fixture", &[generation])
            .unwrap();
        assert_eq!(
            prepare(&catalog, &["c"]).groups[0].effective().category,
            expected
        );
        assert_eq!(
            prepare(&catalog, &["a"]).groups[0].effective().category,
            Some(category("drawing"))
        );
    }
    assert!(
        catalog
            .change_classification(
                &id("stale-content"),
                &old,
                &ClassificationAction::Correct(category("drawing"))
            )
            .is_err()
    );
}

#[test]
fn catalog_rebuild_recovers_classification_from_compatible_content_evidence() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    setup(&mut catalog);
    catalog
        .change_classification(
            &id("first"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    catalog
        .connection
        .execute("DELETE FROM asset_locations WHERE root_id = 'root-a'", [])
        .unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "rebuilt",
        "root-a",
        "C:\\Fixture",
        &[("a", None, 1)],
    );
    publish(&mut catalog, "a", 'a');
    let generation = selected(&catalog, "a").source_generation;
    catalog
        .publish_exact_analysis_batch("organization-fixture", &[generation])
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().category,
        Some(category("drawing"))
    );
}
