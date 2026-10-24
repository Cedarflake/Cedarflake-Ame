use std::sync::atomic::AtomicBool;

use rusqlite::params;

use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};
use crate::domain::exact_duplicates::{ContentFingerprint, ExactAnalysisRun, FingerprintCandidate};
use crate::ports::exact_duplicate_review::ExactDuplicateReviewRepository;
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

use super::*;

mod admission;
mod lifecycle;
mod publication;
mod queries;
mod reconciliation;
mod transitions;
mod undo;

fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-a",
        "root-a",
        "C:\\Fixture",
        &[("a", None, 1), ("b", None, 2)],
    );
    super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-c",
        "root-c",
        "D:\\Fixture",
        &[("c", None, 3)],
    );
    catalog.connection.execute_batch(
        "INSERT INTO library_root_publication_namespaces
           (root_id, root_generation, identity_scheme, identity_value, authority_kind,
            established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1',
            '0000000000000001:00000000000000000000000000000001', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state;
         UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001';"
    ).unwrap();
    catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "organization-fixture".into(),
            started_unix_ms: 1,
        })
        .unwrap();
    (directory, catalog)
}

fn publish(catalog: &mut SqliteCatalog, id: &str, digest: char) {
    let location = catalog.load_active_location(id).unwrap().unwrap();
    let root = catalog
        .load_incremental_catalog_root(&location.root_id)
        .unwrap()
        .unwrap();
    assert!(
        catalog
            .publish_fingerprint(
                "organization-fixture",
                &FingerprintCandidate { location, root },
                &ContentFingerprint {
                    digest: digest.to_string().repeat(64),
                    byte_length: 20
                },
                &AtomicBool::new(false)
            )
            .unwrap()
    );
}

fn selected(catalog: &SqliteCatalog, id: &str) -> OrganizationSelection {
    let location = catalog.load_active_location(id).unwrap().unwrap();
    OrganizationSelection {
        asset_id: location.asset_id,
        location_id: location.location_id,
        source_generation: location.source_generation,
    }
}

fn prepare(catalog: &SqliteCatalog, ids: &[&str]) -> OrganizationPreparedSelection {
    catalog
        .prepare_organization_selection(
            &ids.iter()
                .map(|id| selected(catalog, id))
                .collect::<Vec<_>>(),
        )
        .unwrap()
}

fn scalar(catalog: &SqliteCatalog, sql: &str) -> u64 {
    catalog
        .connection
        .query_row(sql, [], |row| {
            super::super::exact_duplicates::read_unsigned(row, 0)
        })
        .unwrap()
}

#[test]
fn folded_unfolded_and_cross_root_selections_target_the_same_whole_group() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    let folded = prepare(&catalog, &["a"]);
    let scoped = prepare(&catalog, &["c"]);
    let unfolded = prepare(&catalog, &["c", "a", "c"]);
    assert_eq!(folded.groups, scoped.groups);
    assert_eq!(scoped.groups, unfolded.groups);
    assert_eq!(unfolded.groups.len(), 1);
    assert_eq!(unfolded.groups[0].location_count, 2);
    assert_eq!(unfolded.groups[0].representative.location_id, "a");
    assert_eq!(unfolded.selections.len(), 2);
    let receipt = catalog
        .set_organization_favorite("favorite-whole-group", &scoped, true)
        .unwrap();
    assert_eq!(receipt.changed_groups, 1);
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_sources"),
        2
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_bindings"),
        2
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE favorite = 1 AND disposition = 'pending' AND album_id IS NULL"
        ),
        1
    );
}

#[test]
fn favorite_survives_reopen_and_exact_retry_without_replaying_or_repurposing_the_operation() {
    let (directory, mut catalog) = fixture();
    let prepared = prepare(&catalog, &["b"]);
    let receipt = catalog
        .set_organization_favorite("durable-favorite", &prepared, true)
        .unwrap();
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE favorite = 1"
        ),
        1
    );
    assert_eq!(
        catalog
            .set_organization_favorite("durable-favorite", &prepared, true)
            .unwrap(),
        receipt
    );
    assert_eq!(
        catalog
            .set_organization_favorite("durable-favorite", &prepared, false)
            .unwrap_err()
            .code,
        "organization_operation_conflict"
    );
    let current = prepare(&catalog, &["b"]);
    let no_change = catalog
        .set_organization_favorite("same-favorite", &current, true)
        .unwrap();
    assert_eq!(no_change.changed_groups, 0);
    assert_eq!(
        no_change.organization_revision,
        receipt.organization_revision
    );
}

#[test]
fn replacing_content_or_membership_cannot_authorize_an_old_proposal_without_a_revision_bump() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let prepared = prepare(&catalog, &["a"]);
    publish(&mut catalog, "c", 'a');
    assert_eq!(
        catalog
            .set_organization_favorite("stale-members", &prepared, true)
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    let prepared = prepare(&catalog, &["a"]);
    catalog.connection.execute("UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000002' WHERE location_id = 'a'", []).unwrap();
    assert_eq!(
        catalog
            .set_organization_favorite("stale-content", &prepared, true)
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_operations"),
        0
    );
}

#[test]
fn current_representative_preference_is_global_and_invalidates_a_prepared_write() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    let before = prepare(&catalog, &["a"]);
    let preferred = selected(&catalog, "c");
    catalog
        .set_duplicate_preference(
            &ExactDuplicateReviewKey {
                digest: "a".repeat(64),
                byte_length: 20,
            },
            Some(&ExactDuplicatePreference {
                location_id: preferred.location_id,
                asset_id: preferred.asset_id,
                source_generation: preferred.source_generation,
            }),
        )
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0]
            .representative
            .location_id,
        "c"
    );
    assert_eq!(
        catalog
            .set_organization_favorite("stale-representative", &before, true)
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
}

#[test]
fn pending_display_evidence_cannot_authorize_a_partial_group_or_a_singleton() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    let old = selected(&catalog, "c");
    catalog.connection.execute("INSERT INTO gallery_content_fingerprints SELECT source_generation, scheme FROM content_fingerprints WHERE source_generation = ?1", [old.source_generation as i64]).unwrap();
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'c'", []).unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO gallery_retained_fingerprints VALUES (?1, ?2, 'blake3-256-full-v1')",
            params![
                (old.source_generation + 100) as i64,
                old.source_generation as i64
            ],
        )
        .unwrap();
    for id in ["a", "c"] {
        assert_eq!(
            catalog
                .prepare_organization_selection(&[selected(&catalog, id)])
                .unwrap_err()
                .code,
            "organization_evidence_pending"
        );
    }
}

#[test]
fn failed_batch_preserves_all_prior_favorites_progress_and_receipts() {
    let (_directory, mut catalog) = fixture();
    let prepared = prepare(&catalog, &["a", "b"]);
    catalog.connection.execute_batch(
        "CREATE TRIGGER fail_second_organization_write BEFORE UPDATE OF favorite ON organization_records
         WHEN (SELECT COUNT(*) FROM organization_records WHERE favorite = 1) = 1
         BEGIN SELECT RAISE(ABORT, 'injected save failure'); END;"
    ).unwrap();
    assert!(
        catalog
            .set_organization_favorite("failed-batch", &prepared, true)
            .is_err()
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_operations"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM organization_state"),
        prepared.organization_revision
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_second_organization_write")
        .unwrap();
    assert_eq!(
        catalog
            .set_organization_favorite("failed-batch", &prepared, true)
            .unwrap()
            .changed_groups,
        2
    );
}

#[test]
fn favorite_does_not_change_album_progress_or_duplicate_ignore() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("first", &prepared, true)
        .unwrap();
    catalog.connection.execute_batch(
        "INSERT INTO organization_albums VALUES ('album', 'Landscapes', 1, 1, 1);
         UPDATE organization_records SET album_id = 'album', disposition = 'organized', organization_revision = 9;
         INSERT INTO exact_duplicate_decisions VALUES ('blake3-256-full-v1',
           'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 20, NULL, NULL, 1, 1);"
    ).unwrap();
    let current = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("unfavorite", &current, false)
        .unwrap();
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE album_id = 'album' AND disposition = 'organized' AND organization_revision = 9 AND favorite = 0"
        ),
        1
    );
    assert_eq!(
        scalar(&catalog, "SELECT ignored FROM exact_duplicate_decisions"),
        1
    );
}

#[test]
fn selection_limits_and_tampered_group_authority_are_rejected_before_writes() {
    let (_directory, mut catalog) = fixture();
    let selected = selected(&catalog, "a");
    for selections in [vec![], vec![selected; 257]] {
        assert_eq!(
            catalog
                .prepare_organization_selection(&selections)
                .unwrap_err()
                .code,
            "organization_selection_invalid"
        );
    }
    let mut prepared = prepare(&catalog, &["a"]);
    prepared.groups[0].location_count += 1;
    assert_eq!(
        catalog
            .set_organization_favorite("tampered", &prepared, true)
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
}

#[test]
fn replacing_a_generation_at_the_same_path_rejects_the_old_singleton() {
    let (_directory, mut catalog) = fixture();
    let old = prepare(&catalog, &["a"]);
    catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'a'", []).unwrap();
    assert_eq!(
        catalog
            .set_organization_favorite("old-generation", &old, true)
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    let current = prepare(&catalog, &["a"]);
    assert_ne!(old.groups[0].key, current.groups[0].key);
    assert_eq!(
        catalog
            .set_organization_favorite("new-generation", &current, true)
            .unwrap()
            .changed_groups,
        1
    );
}
