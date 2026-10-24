use std::fs;

use crate::domain::organization_archive::OrganizationArchivePhase;
use crate::ports::organization_archive::OrganizationArchiveRepository;

use super::super::super::tests::{assert_no_scratch, fixture};
use super::*;

fn exported() -> (tempfile::TempDir, std::path::PathBuf, SqliteCatalog) {
    let (directory, source) = fixture();
    let path = directory.path().join("restore.ame-backup");
    source
        .export_organization_archive(&path, &mut |_| true)
        .unwrap();
    let target = SqliteCatalog::open(directory.path().join("restored.sqlite3")).unwrap();
    (directory, path, target)
}

fn proposal(
    catalog: &SqliteCatalog,
    path: &Path,
    operation: &str,
) -> OrganizationArchiveRestoreRequest {
    let inspection = catalog
        .inspect_organization_archive(path, &mut |_| true)
        .unwrap();
    OrganizationArchiveRestoreRequest {
        operation_id: operation.into(),
        archive_id: inspection.archive_id,
        checksum: inspection.checksum,
        expected_catalog_revision: inspection.associations.catalog_revision,
        expected_organization_revision: inspection.associations.organization_revision,
    }
}

fn state(catalog: &SqliteCatalog) -> (i64, i64, i64, i64, i64) {
    catalog.connection.query_row(
        "SELECT catalog_state.revision, organization_state.revision,
           (SELECT COUNT(*) FROM organization_records), (SELECT COUNT(*) FROM organization_operations),
           (SELECT COUNT(*) FROM organization_archive_restores)
         FROM catalog_state CROSS JOIN organization_state", [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).unwrap()
}

#[test]
fn organization_archive_restore_commits_reopens_and_replays_without_reapplying_decisions() {
    let (directory, path, mut target) = exported();
    let request = proposal(&target, &path, "apply-backup");
    let original = fs::read(&path).unwrap();
    let receipt = target
        .restore_organization_archive(&path, &request, &mut |_| true)
        .unwrap();
    assert_eq!(receipt.organization_revision, 4);
    assert_eq!(state(&target), (1, 4, 1, 2, 1));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_no_scratch(directory.path());
    target
        .connection
        .execute_batch(
            "UPDATE organization_records SET favorite = 0, favorite_revision = 5;
         UPDATE organization_state SET revision = 5; UPDATE catalog_state SET revision = 2;",
        )
        .unwrap();
    let catalog_path = target.path.clone();
    drop(target);
    let mut target = SqliteCatalog::open(catalog_path).unwrap();
    fs::rename(&path, directory.path().join("moved.ame-backup")).unwrap();
    assert_eq!(
        target
            .restore_organization_archive(&path, &request, &mut |_| true)
            .unwrap(),
        receipt
    );
    assert_eq!(state(&target), (2, 5, 1, 2, 1));
    assert!(
        !target
            .connection
            .query_row("SELECT favorite FROM organization_records", [], |row| row
                .get::<_, bool>(
                0
            ))
            .unwrap()
    );
}

#[test]
fn organization_archive_restore_rejects_stale_inspection_and_changed_payload_before_writes() {
    for changed in ["catalog", "organization", "backup"] {
        let (directory, path, mut target) = exported();
        let mut request = proposal(&target, &path, "stale-request");
        match changed {
            "catalog" => target
                .connection
                .execute_batch("UPDATE catalog_state SET revision = revision + 1")
                .unwrap(),
            "organization" => target
                .connection
                .execute_batch("UPDATE organization_state SET revision = revision + 1")
                .unwrap(),
            "backup" => request.checksum = "f".repeat(64),
            _ => unreachable!(),
        }
        let before = state(&target);
        let failure = target
            .restore_organization_archive(&path, &request, &mut |_| true)
            .unwrap_err();
        assert_eq!(
            failure.code,
            if changed == "backup" {
                "organization_archive_input_changed"
            } else {
                "organization_archive_inspection_changed"
            }
        );
        assert_eq!(state(&target), before);
        assert_no_scratch(directory.path());
    }
}

#[test]
fn organization_archive_restore_cancelled_write_rolls_back_every_phase_and_allows_retry() {
    let (directory, path, mut target) = exported();
    let request = proposal(&target, &path, "cancel-then-retry");
    for phase in 0..=4 {
        let before = state(&target);
        let failure = target
            .restore_organization_archive(&path, &request, &mut |progress| {
                progress.phase != OrganizationArchivePhase::Restoring
                    || progress.completed_units != phase
            })
            .unwrap_err();
        assert_eq!(failure.code, "organization_archive_cancelled");
        assert_eq!(state(&target), before);
        assert_no_scratch(directory.path());
    }
    target
        .restore_organization_archive(&path, &request, &mut |_| true)
        .unwrap();
    assert_eq!(state(&target), (1, 4, 1, 2, 1));
}

#[test]
fn organization_archive_restore_failure_releases_the_writer_and_staging_attachment() {
    let (directory, path, target) = exported();
    let request = proposal(&target, &path, "failed-then-retry");
    target
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_restoration BEFORE INSERT ON organization_operation_changes
         BEGIN SELECT RAISE(ABORT, 'injected restoration failure'); END;",
        )
        .unwrap();
    // Reopen after the controlled schema edit to retain the ordinary connection identity proof.
    let target_path = target.path.clone();
    drop(target);
    let mut target = SqliteCatalog::open(target_path.clone()).unwrap();
    let before = state(&target);
    assert!(
        target
            .restore_organization_archive(&path, &request, &mut |_| true)
            .is_err()
    );
    assert_eq!(state(&target), before);
    assert_no_scratch(directory.path());
    target
        .connection
        .execute_batch("DROP TRIGGER fail_restoration")
        .unwrap();
    drop(target);
    let mut target = SqliteCatalog::open(target_path).unwrap();
    target
        .restore_organization_archive(&path, &request, &mut |_| true)
        .unwrap();
}

#[test]
fn organization_archive_restore_rejects_identifier_reuse_and_revision_overflow() {
    let (directory, path, mut target) = exported();
    let request = proposal(&target, &path, "same-operation");
    target
        .restore_organization_archive(&path, &request, &mut |_| true)
        .unwrap();
    let before = state(&target);
    let mut different = request;
    different.checksum = "1".repeat(64);
    assert_eq!(
        target
            .restore_organization_archive(&path, &different, &mut |_| true)
            .unwrap_err()
            .code,
        "organization_archive_operation_conflict"
    );
    assert_eq!(state(&target), before);
    target
        .connection
        .execute(
            "UPDATE organization_state SET revision = ?1",
            [i64::MAX - 1],
        )
        .unwrap();
    let full = proposal(&target, &path, "no-revision-space");
    let before = state(&target);
    assert_eq!(
        target
            .restore_organization_archive(&path, &full, &mut |_| true)
            .unwrap_err()
            .code,
        "organization_archive_revision_capacity"
    );
    assert_eq!(state(&target), before);
    assert_no_scratch(directory.path());
}

#[test]
fn organization_archive_restore_can_explicitly_recover_cleaned_records_without_reviving_old_undo() {
    let (directory, path, mut target) = exported();
    let request = proposal(&target, &path, "first-restore");
    let first = target
        .restore_organization_archive(&path, &request, &mut |_| true)
        .unwrap();
    target
        .connection
        .execute_batch("DELETE FROM organization_records")
        .unwrap();
    let request = proposal(&target, &path, "restore-after-cleanup");
    let next = target
        .restore_organization_archive(&path, &request, &mut |_| true)
        .unwrap();
    assert!(next.organization_revision > first.organization_revision);
    let revisions: (i64, i64) = target.connection.query_row(
        "SELECT record.organization_revision, change.applied_revision FROM organization_records AS record
         JOIN organization_operation_changes AS change USING(record_id)", [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert!(revisions.0 > revisions.1);
    assert_no_scratch(directory.path());
}
