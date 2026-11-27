use crate::adapters::sqlite_catalog::organization::tests::fixture;

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
