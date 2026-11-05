use rusqlite::params;

use crate::adapters::sqlite_catalog::organization::tests::fixture;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::similar_images::{SIMILARITY_SCHEME, SimilarContentKey, group_id};

use super::*;

#[test]
fn preference_upgrade_rolls_back_and_preserves_current_and_retired_choices() {
    let (directory, mut catalog) = fixture();
    downgrade_for_test(&catalog.connection);
    let content = SimilarContentKey {
        scheme: EXACT_FINGERPRINT_SCHEME.into(),
        digest: "a".repeat(64),
        byte_length: 20,
    };
    for scheme in [SIMILARITY_SCHEME, super::super::confirmation::LEGACY_SCHEME] {
        let group = group_id(scheme, &content).unwrap();
        catalog
            .connection
            .execute(
                "INSERT INTO similarity_groups VALUES (?1, ?2, ?3, ?4, 20)",
                params![group, scheme, content.scheme, content.digest],
            )
            .unwrap();
        catalog
            .connection
            .execute(
                "INSERT INTO similarity_decisions VALUES (?1, ?2, ?3, 20, 1, 7)",
                params![group, content.scheme, content.digest],
            )
            .unwrap();
    }
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_preferences BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 76 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    current_schema::validate_schema_version(&catalog.connection, 75).unwrap();
    assert_eq!(catalog.connection.query_row(
        "SELECT COUNT(*) FROM similarity_decisions WHERE preferred_digest = ?1 AND revision = 7",
        [&content.digest], |row| row.get::<_, i64>(0),
    ).unwrap(), 2);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_preferences")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    drop(catalog);
    let catalog = crate::adapters::sqlite_catalog::SqliteCatalog::open(
        directory.path().join("catalog.sqlite3"),
    )
    .unwrap();
    assert_eq!(
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM similarity_preferences AS p
         JOIN similarity_decisions AS d USING(group_id)
         WHERE p.fingerprint_scheme = ?1 AND p.digest = ?2 AND p.byte_length = 20
           AND d.ignored = 1 AND d.revision = 7",
                params![content.scheme, content.digest],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        2
    );
}
