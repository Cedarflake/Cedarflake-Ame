use crate::adapters::sqlite_catalog::organization::tests::fixture;

use super::*;

#[test]
fn similarity_validation_checks_owned_foreign_keys_without_claiming_other_tables() {
    let (_directory, catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
         CREATE TABLE unrelated_parent(id INTEGER PRIMARY KEY);
         CREATE TABLE unrelated_child(parent_id INTEGER REFERENCES unrelated_parent(id));
         INSERT INTO unrelated_child VALUES (1);",
        )
        .unwrap();
    validate_rows(&catalog.connection).unwrap();

    catalog
        .connection
        .execute_batch(
            "INSERT INTO similarity_variants(evidence_id, variant, word0, word1, word2, word3)
         VALUES (1234567, 0, 1, 2, 3, 4);",
        )
        .unwrap();
    assert_eq!(
        validate_rows(&catalog.connection).unwrap_err().code,
        "catalog_similarity_contract_invalid"
    );
}
