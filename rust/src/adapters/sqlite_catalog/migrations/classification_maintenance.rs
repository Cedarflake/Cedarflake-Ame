use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

pub(super) const EVIDENCE_TABLE_V71: &str = "CREATE TABLE classification_maintenance_evidence (
        operation_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_operations(operation_id),
        run_id TEXT NOT NULL REFERENCES classification_runs(run_id),
        evidence_revision TEXT NOT NULL CHECK(length(evidence_revision) = 64),
        payload TEXT NOT NULL CHECK(json_valid(payload) AND json_extract(payload, '$.version') IS 1
            AND length(CAST(payload AS BLOB)) BETWEEN 1 AND 524288)
    ) STRICT, WITHOUT ROWID";

const OBJECTS: &[(&str, &str)] = &[
    ("classification_maintenance_evidence", EVIDENCE_TABLE_V71),
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

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 70)?;
    for (_, statement) in OBJECTS {
        transaction
            .execute_batch(statement)
            .map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 71; PRAGMA user_version = 71;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 71)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    let version = schema_shape::schema_version(connection)?;
    for (name, statement) in OBJECTS {
        let statement = if *name == "classification_maintenance_evidence" && version >= 72 {
            super::classification_retirement::EVIDENCE_TABLE
        } else {
            statement
        };
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

#[cfg(test)]
pub(super) fn downgrade_to_v70_for_test(connection: &Connection) {
    super::classification_retirement::downgrade_to_v71_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() != 71 {
        return;
    }
    connection
        .execute_batch(
            "DROP TABLE classification_managed_categories;
         DROP TABLE classification_maintenance_evidence;
         UPDATE schema_info SET version = 70; PRAGMA user_version = 70;",
        )
        .unwrap();
}

#[cfg(test)]
mod tests;
