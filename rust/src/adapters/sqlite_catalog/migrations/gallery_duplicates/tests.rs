use super::*;

#[test]
fn upgrade_preserves_complete_evidence_and_admits_it_once() {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    connection.execute_batch(
        "DROP TABLE gallery_content_fingerprints;
         UPDATE schema_info SET version = 40; PRAGMA user_version = 40;
         INSERT INTO assets VALUES ('retained', 123);
         INSERT INTO exact_analysis_runs VALUES ('retained-run', 'scheme', 'engine', '1', 'full', 1);
         INSERT INTO content_fingerprints VALUES
           (7, 'scheme', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            20, NULL, NULL, NULL, 'retained-run');"
    ).unwrap();
    let before: i64 = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .unwrap();
    migrate(&mut connection).unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    let actual = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM content_fingerprints),
                (SELECT COUNT(*) FROM gallery_content_fingerprints),
                (SELECT created_unix_ms FROM assets WHERE id = 'retained'), revision
         FROM catalog_state",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(actual, (1, 1, 123, before + 1));
}

#[test]
fn malformed_projection_cannot_open_or_advance_schema() {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::migrate_schema(&mut connection).unwrap();
    connection.execute_batch("DROP TABLE gallery_content_fingerprints; CREATE TABLE gallery_content_fingerprints(unrelated TEXT)").unwrap();
    assert_eq!(
        validate(&connection).unwrap_err().code,
        "catalog_gallery_duplicates_contract_invalid"
    );
    connection
        .execute_batch("UPDATE schema_info SET version = 40; PRAGMA user_version = 40")
        .unwrap();
    assert!(migrate(&mut connection).is_err());
    assert_eq!(
        connection
            .query_row("SELECT version FROM schema_info", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        40
    );
}
