use std::collections::HashMap;

use crate::adapters::sqlite_catalog::catalog_delta::location_mutation;
use crate::domain::GalleryQuery;
use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};
use crate::domain::{
    CatalogDeltaMutation, DerivedEvidenceDisposition, IncrementalReconciliationOutcome,
    LibraryChangeId,
};
use crate::ports::exact_duplicates::GalleryDuplicateRepository;

use super::gallery_queries::{finish, ids, merged};
use super::*;

fn grouped() -> (tempfile::TempDir, SqliteCatalog) {
    let (directory, mut catalog) = fixture();
    catalog.connection.execute(
        "UPDATE asset_locations SET file_identity_scheme = 'fixture-identity', file_identity_value = location_id", [],
    ).unwrap();
    for id in ["a", "b"] {
        publish(&mut catalog, id, &"a".repeat(64));
    }
    finish(&mut catalog, "run");
    (directory, catalog)
}

fn invalidate(catalog: &mut SqliteCatalog, id: &str, changed: bool) -> Result<(), ScanError> {
    let mut location = candidate(catalog, id).location;
    location.source_generation = 0;
    if changed {
        location.source_revision.as_mut().unwrap().value = "0000000000000002".to_owned();
    }
    let transaction = catalog.connection.transaction().unwrap();
    location_mutation::apply(
        &transaction,
        &location.scan_id,
        None,
        &location.root_id,
        &CatalogDeltaMutation {
            change_id: LibraryChangeId::new(1).unwrap(),
            outcome: IncrementalReconciliationOutcome::Modified,
            evidence_disposition: DerivedEvidenceDisposition::InvalidateDerived,
            remove_location_ids: vec![location.location_id.clone()],
            upsert_location: Some(location.clone()),
            retained_preview_expectation: None,
        },
        &mut HashMap::new(),
        &mut HashMap::new(),
    )?;
    transaction
        .execute("UPDATE catalog_state SET revision = revision + 1", [])
        .unwrap();
    transaction.commit().unwrap();
    Ok(())
}

fn begin_recheck(catalog: &mut SqliteCatalog) {
    catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "recheck".into(),
            started_unix_ms: 2,
        })
        .unwrap();
}

fn publish_recheck(catalog: &mut SqliteCatalog, id: &str, digest: char) {
    let source = candidate(catalog, id);
    assert!(
        catalog
            .publish_fingerprint(
                "recheck",
                &source,
                &ContentFingerprint {
                    digest: digest.to_string().repeat(64),
                    byte_length: 20
                },
                &AtomicBool::new(false),
            )
            .unwrap()
    );
    catalog
        .publish_exact_analysis_batch("recheck", &[source.location.source_generation])
        .unwrap();
}

#[test]
fn old_cards_remain_folded_while_new_confirmed_copies_join_and_changed_content_splits() {
    let (_directory, mut catalog) = grouped();
    let visible_revision = catalog
        .load_snapshot(10, &merged(), "visible", None, None, None)
        .unwrap()
        .revision;
    for id in ["a", "b"] {
        invalidate(&mut catalog, id, false).unwrap();
    }
    assert_eq!(ids(&mut catalog, &merged()), ["a", "c"]);
    let annotations = catalog
        .load_gallery_duplicates(visible_revision, &["a".into()])
        .unwrap();
    assert!(annotations.revision > visible_revision);
    assert_eq!(annotations.annotations[0].pending_location_count, 2);
    assert!(
        catalog
            .load_duplicate_groups(None)
            .unwrap()
            .groups
            .is_empty()
    );
    assert_eq!(catalog.load_fingerprint_candidates("").unwrap().len(), 3);
    let members = catalog
        .load_gallery_duplicate_locations(&"a".repeat(64), 20, "")
        .unwrap();
    assert_eq!(
        (members.location_count, members.pending_location_count),
        (2, 2)
    );
    assert_eq!(members.members.len(), 2);

    let source = candidate(&catalog, "a").location;
    assert!(
        catalog
            .set_duplicate_preference(
                &ExactDuplicateReviewKey {
                    digest: "a".repeat(64),
                    byte_length: 20
                },
                Some(&ExactDuplicatePreference {
                    location_id: source.location_id,
                    asset_id: source.asset_id,
                    source_generation: source.source_generation,
                }),
            )
            .is_err()
    );

    begin_recheck(&mut catalog);
    publish_recheck(&mut catalog, "c", 'a');
    assert_eq!(ids(&mut catalog, &merged()), ["a"]);
    let members = catalog
        .load_gallery_duplicate_locations(&"a".repeat(64), 20, "")
        .unwrap();
    assert_eq!(
        (members.location_count, members.pending_location_count),
        (3, 2)
    );
    publish_recheck(&mut catalog, "a", 'b');
    assert_eq!(ids(&mut catalog, &merged()), ["a", "b"]);
    publish_recheck(&mut catalog, "b", 'a');
    assert_eq!(ids(&mut catalog, &merged()), ["a", "b"]);
    let members = catalog
        .load_gallery_duplicate_locations(&"a".repeat(64), 20, "")
        .unwrap();
    assert_eq!(
        (members.location_count, members.pending_location_count),
        (2, 0)
    );
    assert_eq!(
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM gallery_retained_fingerprints",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn changed_revision_cannot_keep_a_retained_match_even_when_size_and_timestamp_match() {
    let (_directory, mut catalog) = grouped();
    invalidate(&mut catalog, "b", true).unwrap();
    assert_eq!(ids(&mut catalog, &merged()), ["a", "b", "c"]);
    assert_eq!(
        catalog
            .load_gallery_duplicate_locations(&"a".repeat(64), 20, "")
            .unwrap()
            .location_count,
        1
    );
}

#[test]
fn repeated_invalidation_and_reopen_retain_one_pending_relationship_per_source() {
    let (directory, mut catalog) = grouped();
    invalidate(&mut catalog, "b", false).unwrap();
    invalidate(&mut catalog, "b", false).unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(ids(&mut catalog, &merged()), ["a", "c"]);
    assert_eq!(
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM gallery_retained_fingerprints",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    let members = catalog
        .load_gallery_duplicate_locations(&"a".repeat(64), 20, "a")
        .unwrap();
    assert_eq!(members.members.len(), 1);
    assert_eq!(members.members[0].location.location_id, "b");
    assert_eq!(
        (members.location_count, members.pending_location_count),
        (2, 1)
    );
}

#[test]
fn retention_failure_rolls_back_the_generation_and_published_membership() {
    let (_directory, mut catalog) = grouped();
    let generation = candidate(&catalog, "b").location.source_generation;
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_retention BEFORE INSERT ON gallery_retained_fingerprints
         BEGIN SELECT RAISE(ABORT, 'retention rejected'); END;",
        )
        .unwrap();
    assert!(invalidate(&mut catalog, "b", false).is_err());
    assert_eq!(
        candidate(&catalog, "b").location.source_generation,
        generation
    );
    assert_eq!(ids(&mut catalog, &merged()), ["a", "c"]);
}

#[test]
fn retained_hardlinks_share_one_generation_without_widening_the_gallery_scope() {
    let (_directory, mut catalog) = grouped();
    catalog.connection.execute_batch(
        "UPDATE asset_locations SET
           source_generation = (SELECT source_generation FROM asset_locations WHERE location_id = 'a'),
           modified_unix_ms = (SELECT modified_unix_ms FROM asset_locations WHERE location_id = 'a'),
           file_identity_value = 'a' WHERE location_id = 'b'",
    ).unwrap();
    invalidate(&mut catalog, "b", false).unwrap();
    assert_eq!(ids(&mut catalog, &merged()), ["a", "c"]);
    let query = GalleryQuery {
        root_id: Some("root-a".into()),
        search_text: "b".into(),
        ..merged()
    };
    let snapshot = catalog
        .load_snapshot(10, &query, "scoped", None, None, None)
        .unwrap();
    assert_eq!(snapshot.assets.len(), 1);
    assert_eq!(snapshot.assets[0].location_id, "b");
    let annotations = catalog
        .load_gallery_duplicates(snapshot.revision, &["b".into()])
        .unwrap();
    let annotation = &annotations.annotations[0];
    assert_eq!(annotation.pending_location_count, 2);
    assert_eq!(annotation.group.physical_file_count, 1);
    let location = candidate(&catalog, "a").location;
    let transaction = catalog.connection.transaction().unwrap();
    location_mutation::apply(
        &transaction,
        &location.scan_id,
        None,
        &location.root_id,
        &CatalogDeltaMutation {
            change_id: LibraryChangeId::new(2).unwrap(),
            outcome: IncrementalReconciliationOutcome::Removed,
            evidence_disposition: DerivedEvidenceDisposition::RemoveFromCurrentProjection,
            remove_location_ids: vec!["a".into()],
            upsert_location: None,
            retained_preview_expectation: None,
        },
        &mut HashMap::new(),
        &mut HashMap::new(),
    )
    .unwrap();
    transaction.commit().unwrap();
    assert_eq!(ids(&mut catalog, &merged()), ["b", "c"]);
    let removed = catalog
        .load_gallery_duplicates(snapshot.revision, &["a".into()])
        .unwrap();
    assert!(removed.annotations.is_empty());
    let members = catalog
        .load_gallery_duplicate_locations(&"a".repeat(64), 20, "")
        .unwrap();
    assert_eq!(
        (members.location_count, members.pending_location_count),
        (1, 1)
    );
    assert_eq!(members.members[0].location.location_id, "b");
}
