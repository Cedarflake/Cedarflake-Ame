use std::fs;

use rusqlite::Connection;
use serde_json::Value;

use super::*;

pub(super) fn fixture() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-a",
        "root-a",
        "C:\\UnavailableArchiveFixture",
        &[("a", None, 1)],
    );
    catalog.connection.execute_batch(
        "UPDATE organization_state SET revision = 3;
         UPDATE organization_history_state SET started_revision = 3;
         INSERT INTO organization_albums VALUES ('album-a', '风景🌲', 1, 1, 1);
         INSERT INTO organization_albums VALUES ('album-b', '旧相册', 2, 1, 2);
         INSERT INTO organization_records VALUES ('record-a', NULL, 'deferred', 1, 3, 2, 1, 3, NULL);
         INSERT INTO organization_record_groups VALUES ('record-a', 'record-a');
         INSERT INTO organization_sources VALUES ('root-a', 'record-a');
         INSERT INTO organization_conflicts VALUES ('record-a', 'album-a', 1);
         INSERT INTO organization_conflicts VALUES ('record-a', 'album-b', 2);
         INSERT INTO organization_album_retirements VALUES ('album-b', 3, 3);
         INSERT INTO organization_bindings
           SELECT 'record-a', asset_id, source_generation, file_size,
                  file_identity_scheme, file_identity_value, source_revision_token, NULL, NULL
           FROM asset_locations;
         INSERT INTO organization_operations VALUES ('operation-a', printf('%064d', 0), 'organization', 3, 1, 3);
         INSERT INTO organization_operation_changes VALUES ('operation-a', 'record-a', 'organization', NULL, 'pending', NULL, 3);
         INSERT INTO organization_operation_targets VALUES ('operation-a', 'record-a', 'organization');
         INSERT INTO organization_operation_conflicts VALUES ('operation-a', 'record-a', 'album-a', 1);
         INSERT INTO organization_album_operation_changes VALUES ('operation-a', 'album-b', '旧相册', 0, 3);
         INSERT INTO organization_review_scopes VALUES ('bookmark', 'library', NULL, NULL, NULL, 1, '', 'pending');
         INSERT INTO organization_review_scopes VALUES ('active', 'library', NULL, NULL, NULL, 1, '', 'pending');
         INSERT INTO organization_review_session VALUES (1, 'active', 'live-session', 1, NULL);
         INSERT INTO organization_review_bookmark
           SELECT 1, 'bookmark', 'record-a', admission_id, 'root-a', 'a', 0, 3
           FROM organization_admissions LIMIT 1;
         INSERT INTO organization_review_operations VALUES ('operation-a', 'old-session', 1, 2);
         INSERT INTO exact_duplicate_decisions VALUES ('blake3-256-full-v1', printf('%064d', 0), 20, 'a', 'asset-a', 1, 3);"
    ).unwrap();
    (directory, catalog)
}

fn entries(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

pub(super) fn assert_no_scratch(directory: &Path) {
    for entry in fs::read_dir(directory).unwrap() {
        assert!(
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".ame-")
        );
    }
}

#[test]
fn organization_archive_exports_durable_families_without_original_reads() {
    let (directory, catalog) = fixture();
    let destination = directory.path().join("整理记录.ame-backup");
    let receipt = catalog
        .export_organization_archive(&destination, &mut |_| true)
        .unwrap();
    let records = entries(&destination);
    for kind in records::KINDS {
        assert!(
            records.iter().any(|record| record["kind"] == kind.name),
            "missing {}",
            kind.name
        );
    }
    assert_eq!(receipt.record_count as usize, records.len() - 2);
    assert_eq!(
        receipt.byte_length,
        fs::metadata(&destination).unwrap().len()
    );
    assert_eq!(records[0]["header"]["archive_id"], receipt.archive_id);
    assert!(
        records
            .iter()
            .any(|record| record["kind"] == "album" && record["values"][1] == "风景🌲")
    );
    let content = fs::read_to_string(destination).unwrap();
    assert!(!content.contains("live-session"));
    assert!(!content.contains("preview_path"));
    assert_no_scratch(directory.path());
}

#[test]
fn organization_archive_snapshot_survives_concurrent_decisions() {
    let (directory, catalog) = fixture();
    let live = Connection::open(&catalog.path).unwrap();
    let destination = directory.path().join("snapshot.ame-backup");
    let mut changed = false;
    catalog
        .export_organization_archive(&destination, &mut |progress| {
            if progress.phase == OrganizationArchivePhase::Writing && !changed {
                live.execute_batch(
                    "UPDATE organization_albums SET name = 'Later';
                UPDATE organization_state SET revision = 4;",
                )
                .unwrap();
                changed = true;
            }
            true
        })
        .unwrap();
    let records = entries(&destination);
    assert_eq!(records[0]["header"]["organization_revision"], 3);
    assert!(records.iter().any(|record| record["values"][1] == "风景🌲"));
    assert!(!fs::read_to_string(destination).unwrap().contains("Later"));
    assert_eq!(
        live.query_row("SELECT revision FROM organization_state", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        4
    );
}

#[test]
fn organization_archive_cancels_each_phase_without_publishing_or_retaining_scratch() {
    for phase in [
        OrganizationArchivePhase::Snapshot,
        OrganizationArchivePhase::Writing,
        OrganizationArchivePhase::Verifying,
    ] {
        let (directory, catalog) = fixture();
        let destination = directory.path().join("cancel.ame-backup");
        let failure = catalog
            .export_organization_archive(&destination, &mut |progress| progress.phase != phase)
            .unwrap_err();
        assert_eq!(failure.code, "organization_archive_cancelled");
        assert!(!destination.exists());
        assert_no_scratch(directory.path());
        assert_eq!(
            catalog
                .connection
                .query_row("SELECT revision FROM organization_state", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            3
        );
    }
}

#[test]
fn organization_archive_preserves_existing_and_concurrently_created_destinations() {
    let (directory, catalog) = fixture();
    let existing = directory.path().join("existing.ame-backup");
    fs::write(&existing, "keep existing").unwrap();
    assert!(
        catalog
            .export_organization_archive(&existing, &mut |_| true)
            .is_err()
    );
    assert_eq!(fs::read_to_string(existing).unwrap(), "keep existing");
    let raced = directory.path().join("raced.ame-backup");
    let mut created = false;
    assert!(
        catalog
            .export_organization_archive(&raced, &mut |progress| {
                if progress.phase == OrganizationArchivePhase::Verifying && !created {
                    fs::write(&raced, "created concurrently").unwrap();
                    created = true;
                }
                true
            })
            .is_err()
    );
    assert_eq!(fs::read_to_string(raced).unwrap(), "created concurrently");
    assert_no_scratch(directory.path());
}

#[test]
fn organization_archive_empty_catalog_is_complete() {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("empty.sqlite3")).unwrap();
    let destination = directory.path().join("empty.ame-backup");
    let receipt = catalog
        .export_organization_archive(&destination, &mut |_| true)
        .unwrap();
    assert_eq!(receipt.record_count, 0);
    assert_eq!(entries(&destination).len(), 2);
    assert_no_scratch(directory.path());
}
