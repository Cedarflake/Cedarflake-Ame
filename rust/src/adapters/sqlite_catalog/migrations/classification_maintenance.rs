use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, schema_shape};

const OBJECTS: &[(&str, &str)] = &[
    ("classification_maintenance_evidence", super::classification_merge::EVIDENCE_TABLE),
    ("classification_managed_categories", "CREATE TABLE classification_managed_categories (
        category_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_categories(category_id),
        concept_id TEXT NOT NULL CHECK(length(concept_id) = 64 AND concept_id NOT GLOB '*[^0-9a-f]*'),
        purpose_category_id TEXT REFERENCES classification_categories(category_id),
        operation_id TEXT NOT NULL REFERENCES classification_operations(operation_id),
        admitted_label TEXT NOT NULL CHECK(length(admitted_label) BETWEEN 1 AND 80),
        admitted_semantic_revision TEXT NOT NULL CHECK(length(admitted_semantic_revision) = 64),
        protected INTEGER NOT NULL CHECK(protected IN (0, 1)),
        FOREIGN KEY(category_id, admitted_semantic_revision)
            REFERENCES classification_category_versions(category_id, semantic_revision)
    ) STRICT, WITHOUT ROWID"),
];

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (name, statement) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, "table", name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid_row: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_maintenance_evidence'))
         OR EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_managed_categories'))
         OR EXISTS(SELECT 1 FROM classification_managed_categories AS origin
             JOIN classification_operations AS operation USING(operation_id)
             WHERE operation.operation_kind <> 'configure')
         OR EXISTS(SELECT 1 FROM classification_maintenance_evidence AS evidence
             JOIN classification_operations AS operation USING(operation_id)
             WHERE operation.operation_kind <> 'configure' OR operation.restored <> 0)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_row {
        return Err(invalid());
    }
    super::super::classification::maintenance::validate_origins(connection)
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_maintenance_invalid",
        "Managed category origins or evidence are inconsistent",
    )
}
