use rusqlite::{Connection, params};

use crate::adapters::sqlite_catalog::SqliteCatalog;
use crate::ports::organization_archive::OrganizationArchiveRepository;

use super::super::{staging, tests::fixture};
use super::*;

mod duplicates;
mod queue;

fn attach_archive(
    target: &SqliteCatalog,
    source: &SqliteCatalog,
    directory: &std::path::Path,
) -> (String, staging::StagedArchive) {
    let path = directory.join("source.ame-backup");
    let receipt = source
        .export_organization_archive(&path, &mut |_| true)
        .unwrap();
    let staged = staging::read(&path, directory, &mut |_| true).unwrap();
    target
        .connection
        .execute(
            "ATTACH DATABASE ?1 AS recovery",
            [staged.path().to_str().unwrap()],
        )
        .unwrap();
    target.connection.execute_batch(
        "INSERT INTO organization_operations VALUES ('restore', printf('%064d', 0), 'restore', 0, 0, 10);",
    ).unwrap();
    target
        .connection
        .execute(
            "INSERT INTO organization_archive_restores
         VALUES (?1, ?2, 'restore', 100, 3, 10)",
            params![receipt.archive_id, "0".repeat(64)],
        )
        .unwrap();
    (receipt.archive_id, staged)
}

fn apply(target: &SqliteCatalog, archive: &str) -> Result<(), ScanError> {
    let transaction = target.connection.unchecked_transaction().unwrap();
    super::apply_decisions(&transaction, archive, 100, 10, &mut |_| true)?;
    transaction
        .execute("UPDATE organization_state SET revision = 103", [])
        .unwrap();
    transaction.commit().unwrap();
    Ok(())
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn organization_archive_decision_write_retains_lineage_conflicts_and_history_without_originals() {
    let (directory, source) = fixture();
    let path = directory.path().join("target.sqlite3");
    let target = SqliteCatalog::open(path.clone()).unwrap();
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    assert_eq!(count(&target.connection, "organization_albums"), 2);
    assert_eq!(count(&target.connection, "organization_records"), 1);
    assert_eq!(count(&target.connection, "organization_conflicts"), 2);
    assert_eq!(
        count(&target.connection, "organization_operation_changes"),
        1
    );
    assert_eq!(
        count(&target.connection, "organization_album_operation_changes"),
        1
    );
    assert_eq!(
        count(&target.connection, "organization_operation_conflicts"),
        1
    );
    assert_eq!(
        count(&target.connection, "organization_review_operations"),
        1
    );
    assert_eq!(count(&target.connection, "organization_review_session"), 0);
    let state: (String, bool, i64, i64) = target.connection.query_row(
        "SELECT disposition, favorite, organization_revision, unassociated_since_unix_ms FROM organization_records",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).unwrap();
    assert_eq!(state, ("deferred".into(), true, 103, 10));
    let next: i64 = target
        .connection
        .query_row(
            "SELECT next_source_generation FROM catalog_state",
            [],
            |row| row.get(0),
        )
        .unwrap();
    apply(&target, &archive).unwrap();
    assert_eq!(count(&target.connection, "organization_bindings"), 1);
    assert_eq!(
        target
            .connection
            .query_row(
                "SELECT next_source_generation FROM catalog_state",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        next
    );
    target
        .connection
        .execute_batch("DETACH DATABASE recovery")
        .unwrap();
    drop(staged);
    drop(target);
    let reopened = SqliteCatalog::open(path).unwrap();
    assert_eq!(count(&reopened.connection, "organization_records"), 1);
}

#[test]
fn organization_archive_decision_write_preserves_existing_values_and_retired_album_authority() {
    let (directory, source) = fixture();
    let target = SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
    target
        .connection
        .execute_batch(
            "INSERT INTO organization_albums VALUES ('album-a', 'Current name', 7, 1, 7);
         INSERT INTO organization_records VALUES ('record-a', NULL, 'ignored', 0, 8, 9, 1, 9, 5);
         INSERT INTO organization_record_groups VALUES ('record-a', 'record-a');
         UPDATE organization_state SET revision = 9;",
        )
        .unwrap();
    let (archive, _staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    let state: (String, bool, i64) = target
        .connection
        .query_row(
            "SELECT disposition, favorite, organization_revision FROM organization_records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(state, ("ignored".into(), false, 8));
    assert_eq!(count(&target.connection, "organization_conflicts"), 0);
    assert_eq!(count(&target.connection, "organization_restored_fields"), 0);
    assert_eq!(
        target
            .connection
            .query_row(
                "SELECT name FROM organization_albums WHERE album_id = 'album-a'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "Current name"
    );
}

#[test]
fn organization_archive_decision_write_failure_rolls_back_identities_and_all_decisions() {
    let (directory, source) = fixture();
    let target = SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
    let (archive, _staged) = attach_archive(&target, &source, directory.path());
    target
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_history BEFORE INSERT ON organization_operation_changes
         BEGIN SELECT RAISE(ABORT, 'injected history failure'); END;",
        )
        .unwrap();
    let before: i64 = target
        .connection
        .query_row(
            "SELECT next_source_generation FROM catalog_state",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(apply(&target, &archive).is_err());
    for table in [
        "organization_albums",
        "organization_records",
        "organization_bindings",
        "organization_archive_generations",
        "organization_recovery_roots",
    ] {
        assert_eq!(count(&target.connection, table), 0, "{table}");
    }
    assert_eq!(
        target
            .connection
            .query_row(
                "SELECT next_source_generation FROM catalog_state",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        before
    );
    target
        .connection
        .execute_batch("DROP TRIGGER fail_history")
        .unwrap();
    apply(&target, &archive).unwrap();
    assert_eq!(count(&target.connection, "organization_records"), 1);
}

#[test]
fn organization_archive_decision_write_repeated_after_explicit_cleanup_restores_paired_history() {
    let (directory, source) = fixture();
    let target = SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    target
        .connection
        .execute(
            "DELETE FROM organization_records WHERE record_id = 'record-a'",
            [],
        )
        .unwrap();
    assert_eq!(
        count(&target.connection, "organization_operation_changes"),
        0
    );
    assert_eq!(
        count(&target.connection, "organization_operation_targets"),
        1
    );
    apply(&target, &archive).unwrap();
    assert_eq!(
        count(&target.connection, "organization_operation_changes"),
        1
    );
    assert_eq!(
        count(&target.connection, "organization_operation_targets"),
        1
    );
    target
        .connection
        .execute_batch("DETACH DATABASE recovery")
        .unwrap();
    drop(staged);
    let exported = directory.path().join("restored.ame-backup");
    target
        .export_organization_archive(&exported, &mut |_| true)
        .unwrap();
    let inspected = target
        .inspect_organization_archive(&exported, &mut |_| true)
        .unwrap();
    assert_eq!(inspected.groups, 1);
    assert_eq!(inspected.favorite_groups, 1);
}

#[test]
fn organization_archive_decision_write_rejects_history_identity_collisions_atomically() {
    let (directory, source) = fixture();
    let target = SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
    let (archive, _staged) = attach_archive(&target, &source, directory.path());
    target.connection.execute_batch(
        "INSERT INTO organization_operations VALUES ('operation-a', printf('%064d', 1), 'organization', 2, 1, 3);
         UPDATE organization_state SET revision = 2;",
    ).unwrap();
    assert_eq!(
        apply(&target, &archive).unwrap_err().code,
        "organization_archive_history_conflict"
    );
    assert_eq!(count(&target.connection, "organization_records"), 0);
    assert_eq!(count(&target.connection, "organization_albums"), 0);
    assert_eq!(
        count(&target.connection, "organization_archive_generations"),
        0
    );
}
