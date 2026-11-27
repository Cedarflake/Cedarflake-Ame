use super::*;
use crate::adapters::sqlite_catalog::migrations::current_schema;

#[test]
fn current_contract_requires_retained_indexes_and_rejects_reintroduced_index() {
    for (modification, error) in [
        (
            GENERATION_INDEX.1,
            "catalog_organization_index_contract_invalid",
        ),
        (
            "DROP INDEX asset_locations_organization_root_generation",
            "catalog_organization_index_contract_invalid",
        ),
        (
            "DROP INDEX asset_locations_exact_source",
            "catalog_exact_analysis_contract_invalid",
        ),
    ] {
        let connection = current_catalog();

        connection.execute_batch(modification).unwrap();
        assert_eq!(
            current_schema::validate_current_schema_contract(&connection)
                .unwrap_err()
                .code,
            error,
            "{modification}"
        );
    }
}

#[test]
fn publication_retains_indexed_generation_and_root_keyset_reads() {
    let connection = current_catalog();

    for (query, access_path, key_predicate) in [
        (
            "SELECT asset_id FROM asset_locations WHERE source_generation = 7",
            "SEARCH asset_locations USING INDEX",
            "(source_generation=?)",
        ),
        (
            "SELECT source_generation FROM asset_locations
             WHERE root_id = 'root' AND scan_id = 'scan' AND source_generation > 7
             ORDER BY source_generation LIMIT 256",
            "SEARCH asset_locations USING COVERING INDEX",
            "(root_id=? AND scan_id=? AND source_generation>?)",
        ),
    ] {
        let plan = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {query}"))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
            .join("\n");
        assert!(plan.contains(access_path), "{plan}");
        assert!(plan.contains(key_predicate), "{plan}");
        assert!(!plan.contains("SCAN asset_locations"), "{plan}");
        assert!(!plan.contains("USE TEMP B-TREE"), "{plan}");
    }
}

fn current_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    super::super::initialize_schema(&mut connection).unwrap();
    connection
}
