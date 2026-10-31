use crate::adapters::SqliteCatalog;
use crate::adapters::sqlite_catalog::organization::tests::fixture;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::*;

#[test]
fn publication_upgrade_preserves_dispatch_and_recovers_published_roots() {
    let (directory, mut catalog) = fixture();
    let state = catalog.classification_dispatch_state().unwrap();
    downgrade_to_v68_for_test(&catalog.connection);
    migrate(&mut catalog.connection).unwrap();
    validate(&catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
    assert_eq!(catalog.classification_dispatch_state().unwrap(), state);
    let rows: u32 = catalog.connection.query_row(
        "SELECT count(*) FROM classification_source_publications WHERE after_source_generation = 0",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(rows, 2);
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.classification_dispatch_state().unwrap(), state);
}

#[test]
fn failed_publication_upgrade_rolls_back_schema_and_retries() {
    let (_directory, mut catalog) = fixture();
    downgrade_to_v68_for_test(&catalog.connection);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_publication_upgrade BEFORE UPDATE OF version ON schema_info
         WHEN NEW.version = 69 BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    assert!(migrate(&mut catalog.connection).is_err());
    assert_eq!(
        schema_shape::schema_version(&catalog.connection).unwrap(),
        68
    );
    for (_, name, _) in OBJECTS {
        let exists: bool = catalog
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists, "{name}");
    }
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_publication_upgrade")
        .unwrap();
    migrate(&mut catalog.connection).unwrap();
    validate_rows(&catalog.connection).unwrap();
}

#[test]
fn publication_pages_seek_with_the_root_source_index() {
    let (_directory, catalog) = fixture();
    let mut statement = catalog
        .connection
        .prepare(
            "EXPLAIN QUERY PLAN SELECT DISTINCT source_generation FROM asset_locations
         WHERE root_id = ?1 AND scan_id = ?2 AND source_generation > ?3 AND source_generation <= ?4
         ORDER BY source_generation LIMIT ?5",
        )
        .unwrap();
    let plan = statement
        .query_map(rusqlite::params!["root-a", "scan-a", 0, 100, 33], |row| {
            row.get::<_, String>(3)
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
        .join("\n");
    assert!(
        plan.contains("USING COVERING INDEX asset_locations_classification_publication"),
        "{plan}"
    );
    assert!(!plan.contains("USE TEMP B-TREE"), "{plan}");
}
