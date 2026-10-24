use crate::domain::organization_cleanup::{OrganizationCleanupPhase, OrganizationCleanupRequest};
use crate::ports::organization_archive::OrganizationArchiveRepository;
use crate::ports::organization_cleanup::OrganizationCleanupRepository;

use super::*;

fn orphaned_favorite() -> (tempfile::TempDir, SqliteCatalog) {
    let (directory, mut catalog) = fixture();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "DELETE FROM asset_locations WHERE location_id = 'a';
         UPDATE organization_records SET unassociated_since_unix_ms = 100;",
        )
        .unwrap();
    (directory, catalog)
}

fn request(catalog: &mut SqliteCatalog, cutoff: Option<i64>) -> OrganizationCleanupRequest {
    OrganizationCleanupRequest {
        operation_id: "cleanup".into(),
        inspection: catalog
            .inspect_organization_cleanup(cutoff, &mut |_| true)
            .unwrap(),
    }
}

#[test]
fn cleanup_retires_decisions_and_queue_authority_without_resurrection() {
    let (_directory, mut catalog) = orphaned_favorite();
    let request = request(&mut catalog, Some(500));
    assert_eq!(request.inspection.groups, 1);
    assert_eq!(request.inspection.favorite_groups, 1);
    assert_eq!(request.inspection.oldest_unassociated_unix_ms, Some(100));
    let receipt = catalog
        .apply_organization_cleanup(&request, &mut |_| true)
        .unwrap();
    assert_eq!(receipt.removed_records, 1);
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_bindings"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operation_targets"
        ),
        1
    );
    assert_eq!(
        catalog
            .apply_organization_cleanup(&request, &mut |_| true)
            .unwrap(),
        receipt
    );

    super::super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-return",
        "root-a",
        "C:\\Fixture",
        &[("a", None, 1)],
    );
    let current = catalog
        .inspect_organization_selection(&[selected(&catalog, "a")])
        .unwrap();
    assert!(!current.decisions[0].favorite);
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
}

#[test]
fn a_reassociated_group_survives_even_without_a_revision_change() {
    let (_directory, mut catalog) = fixture();
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    catalog.connection.execute_batch(
        "UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'a';
         UPDATE organization_records SET unassociated_since_unix_ms = 100;",
    ).unwrap();
    let request = request(&mut catalog, Some(500));
    assert_eq!(request.inspection.groups, 1);
    catalog.connection.execute(
        "UPDATE asset_locations SET source_generation = source_generation - 100 WHERE location_id = 'a'", [],
    ).unwrap();
    let error = catalog
        .apply_organization_cleanup(&request, &mut |_| true)
        .unwrap_err();
    assert_eq!(error.code, "organization_cleanup_changed");
    assert_eq!(
        scalar(&catalog, "SELECT SUM(favorite) FROM organization_records"),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_kind = 'cleanup'"
        ),
        0
    );
    assert_eq!(
        catalog
            .inspect_organization_cleanup(None, &mut |_| true)
            .unwrap()
            .groups,
        0
    );
}

#[test]
fn cancelling_after_one_retirement_window_rolls_back_every_record() {
    let (_directory, mut catalog) = fixture();
    catalog.connection.execute_batch(
        "WITH RECURSIVE sequence(value) AS (SELECT 1 UNION ALL SELECT value + 1 FROM sequence WHERE value < 300)
         INSERT INTO organization_records
         SELECT printf('record-%04d', value), NULL, 'ignored', 1, 0, 0, 1, 1, 100 FROM sequence;
         INSERT INTO organization_record_groups SELECT record_id, record_id FROM organization_records;",
    ).unwrap();
    let request = request(&mut catalog, Some(500));
    assert_eq!(request.inspection.groups, 300);
    let error = catalog
        .apply_organization_cleanup(&request, &mut |progress| {
            progress.phase != OrganizationCleanupPhase::Applying || progress.examined_records == 0
        })
        .unwrap_err();
    assert_eq!(error.code, "organization_cleanup_cancelled");
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        300
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_kind = 'cleanup'"
        ),
        0
    );
}

#[test]
fn age_selection_preserves_recent_records_and_counts_retained_duplicate_choices() {
    let (_directory, mut catalog) = orphaned_favorite();
    catalog.connection.execute(
        "INSERT INTO organization_recovery_duplicate_decisions VALUES (?1, ?2, 20, NULL, NULL, 1, 1, 200)",
        params![crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME, "f".repeat(64)],
    ).unwrap();
    let older = request(&mut catalog, Some(150));
    assert_eq!(older.inspection.groups, 1);
    assert_eq!(older.inspection.duplicate_choices, 0);
    let both = request(&mut catalog, Some(500));
    assert_eq!(both.inspection.duplicate_choices, 1);
    assert_eq!(both.inspection.newest_unassociated_unix_ms, Some(200));
    let receipt = catalog
        .apply_organization_cleanup(&older, &mut |_| true)
        .unwrap();
    assert_eq!(receipt.removed_records, 1);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_recovery_duplicate_decisions"
        ),
        1
    );
}

#[test]
fn a_configured_source_without_a_published_catalog_is_not_cleanup_authority() {
    let (_directory, mut catalog) = orphaned_favorite();
    catalog
        .connection
        .execute(
            "UPDATE library_roots SET active_scan_id = NULL WHERE id = 'root-a'",
            [],
        )
        .unwrap();
    assert_eq!(request(&mut catalog, None).inspection.groups, 0);
    assert_eq!(
        scalar(&catalog, "SELECT SUM(favorite) FROM organization_records"),
        1
    );
}

#[test]
fn failure_and_changed_request_preserve_receipt_and_decisions() {
    let (_directory, mut catalog) = orphaned_favorite();
    let mut request = request(&mut catalog, Some(500));
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER cleanup_failure BEFORE DELETE ON organization_records
         BEGIN SELECT RAISE(ABORT, 'controlled cleanup failure'); END;",
        )
        .unwrap();
    assert!(
        catalog
            .apply_organization_cleanup(&request, &mut |_| true)
            .is_err()
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        1
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER cleanup_failure;")
        .unwrap();
    catalog
        .apply_organization_cleanup(&request, &mut |_| true)
        .unwrap();
    request.inspection.cutoff_unix_ms += 1;
    assert_eq!(
        catalog
            .apply_organization_cleanup(&request, &mut |_| true)
            .unwrap_err()
            .code,
        "organization_operation_conflict"
    );
}

#[test]
fn explicit_backup_application_can_restore_cleaned_records() {
    let (directory, mut catalog) = orphaned_favorite();
    let path = directory.path().join("before-cleanup.ame-backup");
    catalog
        .export_organization_archive(&path, &mut |_| true)
        .unwrap();
    let cleanup = request(&mut catalog, None);
    catalog
        .apply_organization_cleanup(&cleanup, &mut |_| true)
        .unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    let inspected = catalog
        .inspect_organization_archive(&path, &mut |_| true)
        .unwrap();
    catalog
        .restore_organization_archive(
            &path,
            &crate::domain::organization_archive::OrganizationArchiveRestoreRequest {
                operation_id: "restore-cleaned-records".into(),
                archive_id: inspected.archive_id,
                checksum: inspected.checksum,
                expected_catalog_revision: inspected.associations.catalog_revision,
                expected_organization_revision: inspected.associations.organization_revision,
            },
            &mut |_| true,
        )
        .unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT SUM(favorite) FROM organization_records"),
        1
    );
}

#[test]
fn cleanup_removes_all_aliases_once_and_preserves_album_definitions() {
    let (_directory, mut catalog) = fixture();
    catalog.connection.execute_batch(
        "INSERT INTO organization_albums VALUES ('album', 'Retained album', 1, 1, 1);
         INSERT INTO organization_records VALUES ('canonical', 'album', 'organized', 1, 1, 1, 1, 1, 100);
         INSERT INTO organization_records VALUES ('alias', 'album', 'organized', 1, 1, 1, 1, 1, 100);
         INSERT INTO organization_record_groups VALUES ('canonical', 'canonical'), ('alias', 'canonical');",
    ).unwrap();
    let prepared = request(&mut catalog, Some(500));
    assert_eq!(prepared.inspection.groups, 1);
    assert_eq!(prepared.inspection.album_groups, 1);
    assert_eq!(prepared.inspection.progress_groups, 1);
    assert_eq!(
        catalog
            .apply_organization_cleanup(&prepared, &mut |_| true)
            .unwrap()
            .removed_records,
        1
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_albums"),
        1
    );
}

#[test]
fn cleanup_preserves_generation_history_owned_by_a_recent_record() {
    let (_directory, mut catalog) = orphaned_favorite();
    let generation = scalar(
        &catalog,
        "SELECT source_generation FROM organization_bindings LIMIT 1",
    );
    catalog.connection.execute_batch(
        "INSERT INTO organization_records VALUES ('recent', NULL, 'ignored', 0, 0, 0, 1, 1, 1000);
         INSERT INTO organization_record_groups VALUES ('recent', 'recent');
         INSERT INTO organization_bindings
         SELECT 'recent', 'another-retained-asset', source_generation, byte_length,
           file_identity_scheme, file_identity_value, source_revision_token, fingerprint_scheme, digest
         FROM organization_bindings LIMIT 1;",
    ).unwrap();
    let prepared = request(&mut catalog, Some(500));
    assert_eq!(prepared.inspection.groups, 1);
    catalog
        .apply_organization_cleanup(&prepared, &mut |_| true)
        .unwrap();
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        1
    );
    let retained: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM organization_admissions WHERE source_generation = ?1",
            [generation as i64],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained, 1);
}

#[test]
fn inspecting_retained_choices_has_cancellation_between_windows() {
    let (_directory, mut catalog) = fixture();
    catalog.connection.execute_batch(
        "WITH RECURSIVE sequence(value) AS (SELECT 1 UNION ALL SELECT value + 1 FROM sequence WHERE value < 600)
         INSERT INTO organization_recovery_duplicate_decisions
         SELECT 'blake3-256-full-v1', printf('%064x', value), 20, NULL, NULL, 1, 1, 100 FROM sequence;",
    ).unwrap();
    let error = catalog
        .inspect_organization_cleanup(Some(500), &mut |update| update.examined_records < 256)
        .unwrap_err();
    assert_eq!(error.code, "organization_cleanup_cancelled");
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_recovery_duplicate_decisions"
        ),
        600
    );
}
