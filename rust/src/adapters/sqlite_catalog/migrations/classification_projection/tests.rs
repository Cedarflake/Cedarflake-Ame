use crate::adapters::sqlite_catalog::organization::tests::fixture;

use super::*;

#[test]
fn altered_projection_is_rejected_by_schema_validation() {
    let (_directory, catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "DROP VIEW classification_human_projection;
         CREATE VIEW classification_human_projection AS SELECT NULL AS category_id;",
        )
        .unwrap();
    assert_eq!(
        validate(&catalog.connection).unwrap_err().code,
        "classification_projection_invalid"
    );
}
