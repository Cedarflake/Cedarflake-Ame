use super::*;

fn prior_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    downgrade_to_v44_for_test(&connection);
    connection
        .execute_batch(
            "INSERT INTO assets VALUES ('retained-asset', 123);
             UPDATE catalog_state SET revision = 9;",
        )
        .unwrap();
    connection
}

#[test]
fn retained_membership_upgrade_does_not_invent_pending_evidence_or_rescan() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    let state: (i64, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT version, (SELECT user_version FROM pragma_user_version),
                    (SELECT revision FROM catalog_state), (SELECT created_unix_ms FROM assets),
                    (SELECT COUNT(*) FROM gallery_retained_fingerprints)
             FROM schema_info",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(state, (45, 45, 9, 123, 0));
    validate(&connection).unwrap();
}

#[test]
fn conflicting_retained_membership_rolls_back_without_advancing_schema() {
    let mut connection = prior_catalog();
    connection
        .execute_batch("CREATE TABLE gallery_retained_fingerprints(unrelated TEXT)")
        .unwrap();
    assert!(migrate(&mut connection).is_err());
    assert_eq!(
        super::super::schema_shape::schema_version(&connection).unwrap(),
        44
    );
    assert_eq!(
        connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        9
    );
}

#[test]
fn altered_display_projection_is_rejected_before_current_queries() {
    let mut connection = prior_catalog();
    migrate(&mut connection).unwrap();
    connection
        .execute_batch(
            "DROP VIEW gallery_display_fingerprints;
             CREATE VIEW gallery_display_fingerprints AS SELECT 1;",
        )
        .unwrap();
    assert_eq!(
        current_schema::validate_current_schema_structure(&connection)
            .unwrap_err()
            .code,
        "catalog_retained_gallery_membership_invalid"
    );
}
