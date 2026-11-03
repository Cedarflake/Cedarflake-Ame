use crate::application::classification_discovery::{
    CategoryExpansionBudget, survey_category_coverage,
};
use crate::domain::classification::discovery::ConceptId;
use crate::domain::classification::maintenance::CategoryCoverageSummary;
use crate::ports::exact_duplicates::ExactDuplicateRepository;

use super::fixture::{Evidence, assess_with_capacity};
use super::*;

fn survey(catalog: &SqliteCatalog) -> CategoryCoverageSummary {
    survey_category_coverage(catalog, 1, 2, &Arc::new(AtomicBool::new(false))).unwrap()
}

fn next_evidence(catalog: &SqliteCatalog, sequence: u8) -> Evidence {
    let mut next = evidence(catalog);
    next.discovery.candidates[0].candidate.concept = VocabularyConcept::new(
        ConceptId::from_bytes([sequence; 32]),
        format!("Category {sequence}"),
        format!("description {sequence}"),
    )
    .unwrap();
    next
}

fn automatic(
    catalog: &SqliteCatalog,
    evidence: &Evidence,
    operation: &str,
    coverage: &CategoryCoverageSummary,
) -> crate::application::classification_discovery::CategoryExpansionAssessment {
    assess_with_capacity(
        catalog,
        evidence,
        operation,
        CategoryExpansionBudget::VerifiedContents(coverage),
    )
    .unwrap()
}

fn add_source(catalog: &mut SqliteCatalog, name: &str) {
    let root = format!("root-{name}");
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        catalog,
        &format!("scan-{name}"),
        &root,
        &format!("E:\\Fixture-{name}"),
        &[(name, None, 4)],
    );
    catalog
        .connection
        .execute(
            "INSERT INTO library_root_publication_namespaces
           (root_id, root_generation, identity_scheme, identity_value, authority_kind,
            established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1',
            '0000000000000001:00000000000000000000000000000001', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state WHERE root_id = ?1",
            [&root],
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
         SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001'
         WHERE root_id = ?1",
            [&root],
        )
        .unwrap();
}

fn verify(catalog: &mut SqliteCatalog, name: &str, digest: char) {
    publish(catalog, name, digest);
    let generation = selected(catalog, name).source_generation;
    catalog
        .publish_exact_analysis_batch("organization-fixture", &[generation])
        .unwrap();
}

#[test]
fn automatic_capacity_works_before_prediction_completion_and_replays_after_restart() {
    let (directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let coverage = survey(&catalog);
    assert_eq!(coverage.classified_contents, 0);
    assert_eq!(coverage.unclassified_contents, 3);
    assert!(!coverage.is_complete());
    let plan = automatic(&catalog, &evidence(&catalog), "automatic", &coverage)
        .plan
        .unwrap();
    let saved = catalog
        .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(
        saved.catalog.taxonomy.as_ref().unwrap().categories().len(),
        2
    );
    let payload: String = catalog.connection.query_row(
        "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = 'automatic'",
        [], |row| row.get(0),
    ).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let basis = &payload["selection"]["catalog_capacity"];
    assert_eq!(basis["algorithm"], "verified-content-recurrence-v1");
    assert_eq!(basis["verified_contents"], 3);
    assert_eq!(basis["minimum_members"], 2);
    assert_eq!(payload["selection"]["maximum_generated"], 1);
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let stopped = catalog
        .change_classification_dispatch(
            saved.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    let replay = catalog
        .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(replay.dispatch, stopped);
    assert_eq!(replay.catalog, saved.catalog);
}

#[test]
fn only_new_verified_content_unlocks_capacity_while_duplicate_copies_do_not() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let first = automatic(&catalog, &evidence(&catalog), "first", &survey(&catalog))
        .plan
        .unwrap();
    catalog
        .apply_category_expansion(&first, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    let next = next_evidence(&catalog, 9);
    add_source(&mut catalog, "d");
    let unverified = survey(&catalog);
    assert!(unverified.snapshot.unverified_sources);
    assert!(
        automatic(&catalog, &next, "unknown", &unverified)
            .plan
            .is_none()
    );
    verify(&mut catalog, "d", 'a');
    let duplicate = survey(&catalog);
    assert_eq!(duplicate.unclassified_contents, 3);
    assert!(
        automatic(&catalog, &next, "duplicate", &duplicate)
            .plan
            .is_none()
    );
    add_source(&mut catalog, "e");
    verify(&mut catalog, "e", 'e');
    let expanded = survey(&catalog);
    assert_eq!(expanded.unclassified_contents, 4);
    let second = automatic(&catalog, &next, "second", &expanded)
        .plan
        .unwrap();
    catalog
        .apply_category_expansion(&second, &Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(
        catalog
            .load_classification_configuration()
            .unwrap()
            .catalog
            .taxonomy
            .unwrap()
            .categories()
            .len(),
        3
    );
}

#[test]
fn stale_catalog_capacity_rejects_publication_without_changing_the_configuration() {
    for invalidation in [
        "UPDATE catalog_state SET revision = revision + 1",
        "DELETE FROM gallery_content_fingerprints",
    ] {
        let (_directory, mut catalog) = super::super::super::fixture();
        start(&mut catalog);
        let plan = automatic(&catalog, &evidence(&catalog), "stale", &survey(&catalog))
            .plan
            .unwrap();
        let before = catalog.load_classification_configuration().unwrap();
        catalog.connection.execute_batch(invalidation).unwrap();
        assert_eq!(
            catalog
                .apply_category_expansion(&plan, &Arc::new(AtomicBool::new(false)))
                .unwrap_err()
                .code,
            "classification_changed"
        );
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert!(
            catalog
                .load_classification_setup(&id("stale"))
                .unwrap()
                .is_none()
        );
        assert!(catalog.connection.is_autocommit());
    }
}

#[test]
fn another_configuration_cannot_supply_the_automatic_budget() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    let mut coverage = survey(&catalog);
    coverage.snapshot.configuration.dispatch.revision += 1;
    let result = assess_with_capacity(
        &catalog,
        &evidence(&catalog),
        "mismatch",
        CategoryExpansionBudget::VerifiedContents(&coverage),
    );
    assert_eq!(result.err().unwrap().code, "classification_changed");
}

#[test]
fn smaller_capacity_preserves_existing_protected_categories() {
    let (_directory, mut catalog) = super::super::super::fixture();
    start(&mut catalog);
    for sequence in [8, 9] {
        let addition = assess(
            &catalog,
            &next_evidence(&catalog, sequence),
            &format!("add-{sequence}"),
        )
        .unwrap()
        .plan
        .unwrap();
        catalog
            .apply_category_expansion(&addition, &Arc::new(AtomicBool::new(false)))
            .unwrap();
    }
    catalog
        .connection
        .execute_batch(
            "UPDATE classification_managed_categories SET protected = 1;
         DELETE FROM gallery_content_fingerprints;",
        )
        .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        before.catalog.taxonomy.as_ref().unwrap().categories().len(),
        3
    );
    let coverage = survey(&catalog);
    assert_eq!(coverage.unclassified_contents, 0);
    let next = automatic(&catalog, &next_evidence(&catalog, 10), "smaller", &coverage);
    assert!(next.plan.is_none());
    assert_eq!(next.capacity_deferred.len(), 1);
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
}
