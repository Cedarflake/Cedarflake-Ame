use super::*;

#[derive(Debug, PartialEq, Eq)]
struct PreservedRecord {
    record_id: String,
    group_id: String,
    album_id: Option<String>,
    disposition: String,
    favorite: bool,
    organization_revision: i64,
    favorite_revision: i64,
}

fn prior_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    downgrade_to_v47_for_test(&connection);
    connection.execute_batch(
        "UPDATE organization_state SET revision = 3;
         INSERT INTO organization_albums VALUES ('album', '保留相册', 1, 1, 1);
         INSERT INTO organization_records VALUES ('first', 'album', 'organized', 1, 2, 3, 1, 3, NULL);
         INSERT INTO organization_records VALUES ('second', NULL, 'deferred', 0, 2, 0, 1, 2, NULL);
         INSERT INTO organization_operations VALUES ('favorite', printf('%064d', 1), 'favorite', 3, 1, 3);
         INSERT INTO organization_operation_changes VALUES ('favorite', 'first', 'favorite', NULL, NULL, 0, 3);"
    ).unwrap();
    connection
}

#[test]
fn upgrade_preserves_decisions_and_establishes_complete_group_and_undo_authority() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    current_schema::validate_schema_version(&connection, 48).unwrap();
    let records: Vec<PreservedRecord> = connection.prepare(
        "SELECT record.record_id, member.group_id, record.album_id, record.disposition,
           record.favorite, record.organization_revision, record.favorite_revision
         FROM organization_records AS record JOIN organization_record_groups AS member USING(record_id)
         ORDER BY record.record_id"
    ).unwrap().query_map([], |row| Ok(PreservedRecord {
        record_id: row.get(0)?, group_id: row.get(1)?, album_id: row.get(2)?,
        disposition: row.get(3)?, favorite: row.get(4)?, organization_revision: row.get(5)?, favorite_revision: row.get(6)?
    })).unwrap().map(Result::unwrap).collect();
    assert_eq!(
        records,
        vec![
            PreservedRecord {
                record_id: "first".into(),
                group_id: "first".into(),
                album_id: Some("album".into()),
                disposition: "organized".into(),
                favorite: true,
                organization_revision: 2,
                favorite_revision: 3,
            },
            PreservedRecord {
                record_id: "second".into(),
                group_id: "second".into(),
                album_id: None,
                disposition: "deferred".into(),
                favorite: false,
                organization_revision: 2,
                favorite_revision: 0,
            },
        ]
    );
    let target: (String, String, String) = connection
        .query_row("SELECT * FROM organization_operation_targets", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(
        target,
        ("favorite".into(), "first".into(), "favorite".into())
    );
    super::super::migrate_schema(&mut connection).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM organization_operation_targets",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn a_conflicting_transition_schema_rolls_back_without_altering_existing_records() {
    let mut connection = prior_catalog();
    connection
        .execute_batch("CREATE TABLE organization_album_retirements(invalid TEXT)")
        .unwrap();
    assert!(migrate(&mut connection).is_err());
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        47
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM organization_records WHERE favorite = 1",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'organization_record_groups'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn cyclic_groups_missing_targets_and_retired_membership_fail_validation() {
    for mutation in [
        "UPDATE organization_record_groups SET group_id = CASE record_id WHEN 'first' THEN 'second' ELSE 'first' END",
        "DELETE FROM organization_operation_targets",
        "DELETE FROM organization_operation_changes",
        "INSERT INTO organization_album_retirements VALUES ('album', 3, 3)",
        "INSERT INTO organization_operation_conflicts VALUES ('favorite', 'first', 'album', 4)",
    ] {
        let mut connection = prior_catalog();
        migrate(&mut connection).unwrap();
        connection.execute_batch(mutation).unwrap();
        assert_eq!(
            validate_rows(&connection).unwrap_err().code,
            "catalog_organization_transitions_invalid",
            "{mutation}"
        );
    }
}

#[test]
fn removed_records_leave_a_missing_target_receipt_for_atomic_undo_rejection() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    connection
        .execute(
            "DELETE FROM organization_records WHERE record_id = 'first'",
            [],
        )
        .unwrap();
    current_schema::validate_schema_version(&connection, 48).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM organization_operation_changes",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM organization_operation_targets",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}
