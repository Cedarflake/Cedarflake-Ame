use crate::adapters::sqlite_catalog::SqliteCatalog;

use super::*;

fn prior() -> (tempfile::TempDir, SqliteCatalog) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    catalog
        .connection
        .execute_batch(
            "INSERT INTO organization_records VALUES
           ('canonical', NULL, 'ignored', 1, 1, 1, 1, 1, 10),
           ('alias', NULL, 'ignored', 1, 1, 1, 1, 1, 20),
           ('live', NULL, 'pending', 0, 0, 0, 1, 1, NULL);
         INSERT INTO organization_record_groups VALUES
           ('canonical', 'canonical'), ('alias', 'canonical'), ('live', 'live');
         UPDATE organization_state SET revision = 1;",
        )
        .unwrap();
    downgrade_to_v54_for_test(&catalog.connection);
    (directory, catalog)
}

fn indexed(connection: &Connection) -> Vec<(String, String)> {
    connection
        .prepare(
            "SELECT record_id, group_id FROM organization_unassociated_records ORDER BY record_id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn association_index_upgrade_preserves_decisions_timestamps_and_catalog_revision() {
    let (directory, mut catalog) = prior();
    let revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    assert_eq!(
        indexed(&catalog.connection),
        vec![
            ("alias".into(), "canonical".into()),
            ("canonical".into(), "canonical".into()),
        ]
    );
    let retained: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT revision, (SELECT revision FROM organization_state),
           (SELECT SUM(unassociated_since_unix_ms) FROM organization_records
             WHERE favorite = 1 AND disposition = 'ignored'
               AND organization_revision = 1 AND favorite_revision = 1)
         FROM catalog_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(retained, (revision, 1, 30));
    current_schema::validate_schema_version(&catalog.connection, 55).unwrap();
    drop(catalog);
    let reopened = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(indexed(&reopened.connection).len(), 2);
}

#[test]
fn association_index_tracks_loss_restoration_merges_deletion_and_rollback() {
    let (_directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    catalog.connection.execute_batch(
        "UPDATE organization_records SET unassociated_since_unix_ms = NULL WHERE record_id = 'canonical';
         UPDATE organization_record_groups SET group_id = 'live' WHERE record_id = 'alias';
         UPDATE organization_records SET unassociated_since_unix_ms = 40 WHERE record_id = 'live';",
    ).unwrap();
    assert_eq!(
        indexed(&catalog.connection),
        vec![
            ("alias".into(), "live".into()),
            ("live".into(), "live".into()),
        ]
    );
    {
        let transaction = catalog.connection.unchecked_transaction().unwrap();
        transaction
            .execute_batch(
                "UPDATE organization_records SET unassociated_since_unix_ms = NULL;
             DELETE FROM organization_records WHERE record_id = 'alias';",
            )
            .unwrap();
        assert!(indexed(&transaction).is_empty());
    }
    assert_eq!(indexed(&catalog.connection).len(), 2);
    catalog
        .connection
        .execute(
            "DELETE FROM organization_records WHERE record_id = 'alias'",
            [],
        )
        .unwrap();
    assert_eq!(
        indexed(&catalog.connection),
        vec![("live".into(), "live".into())]
    );
    validate_rows(&catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch(
            "INSERT INTO organization_records VALUES ('late', NULL, 'pending', 0, 0, 0, 1, 1, 50);
         INSERT INTO organization_record_groups VALUES ('late', 'late');",
        )
        .unwrap();
    assert_eq!(indexed(&catalog.connection).len(), 2);
    validate_rows(&catalog.connection).unwrap();
}

#[test]
fn association_index_upgrade_rolls_back_all_objects_and_accepts_retry() {
    let (_directory, mut catalog) = prior();
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_version BEFORE UPDATE ON schema_info
         BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 54).unwrap();
    for (_, name, _) in schema::OBJECTS {
        assert!(
            !catalog
                .connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                    [name],
                    |row| row.get::<_, bool>(0),
                )
                .unwrap(),
            "{name}"
        );
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER fail_version")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    assert_eq!(indexed(&catalog.connection).len(), 2);
}

#[test]
fn association_index_rejects_missing_rows_wrong_groups_and_missing_maintenance() {
    let (_directory, mut catalog) = prior();
    migrate(&mut catalog.connection).unwrap();
    catalog
        .connection
        .execute(
            "DELETE FROM organization_unassociated_records WHERE record_id = 'alias'",
            [],
        )
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        invalid().code
    );
    catalog
        .connection
        .execute(
            "INSERT INTO organization_unassociated_records VALUES ('alias', 'live')",
            [],
        )
        .unwrap();
    assert!(validate_rows(&catalog.connection).is_err());
    catalog.connection.execute("UPDATE organization_unassociated_records SET group_id = 'canonical' WHERE record_id = 'alias'", []).unwrap();
    validate_rows(&catalog.connection).unwrap();
    catalog
        .connection
        .execute_batch("DROP TRIGGER organization_unassociated_member_move")
        .unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        invalid().code
    );
}
