use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::{current_schema, database_error};

pub(super) const EVIDENCE_TABLE: &str = "CREATE TABLE classification_maintenance_evidence (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_operations(operation_id),
    run_id TEXT NOT NULL REFERENCES classification_runs(run_id),
    evidence_revision TEXT NOT NULL CHECK(length(evidence_revision) = 64),
    payload TEXT NOT NULL CHECK(json_valid(payload)
        AND (json_extract(payload, '$.version') IS 1
            OR (json_extract(payload, '$.version') IS 2
                AND json_extract(payload, '$.kind') IS 'equivalent_category_retirement'
                AND json_type(payload, '$.retirements') IS 'array'
                AND json_array_length(payload, '$.retirements') BETWEEN 1 AND 64)
            OR (json_extract(payload, '$.version') IS 3
                AND json_extract(payload, '$.kind') IS 'purpose_category_merge'
                AND json_type(payload, '$.merge') IS 'object'
                AND json_type(payload, '$.merge.category') IS 'text'
                AND json_type(payload, '$.merge.purpose') IS 'text'))
        AND length(CAST(payload AS BLOB)) BETWEEN 1 AND 524288)
) STRICT, WITHOUT ROWID";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 72)?;
    transaction.execute_batch(
        "ALTER TABLE classification_maintenance_evidence RENAME TO classification_maintenance_evidence_v72;"
    ).map_err(database_error)?;
    transaction
        .execute_batch(EVIDENCE_TABLE)
        .map_err(database_error)?;
    transaction.execute_batch(
        "INSERT INTO classification_maintenance_evidence SELECT * FROM classification_maintenance_evidence_v72;
         DROP TABLE classification_maintenance_evidence_v72;
         UPDATE schema_info SET version = 73; PRAGMA user_version = 73;"
    ).map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 73)
}

#[cfg(test)]
pub(super) fn downgrade_to_v72_for_test(connection: &Connection) {
    super::similar_images::downgrade_to_v73_for_test(connection);
    if super::schema_shape::schema_version(connection).unwrap() != 73 {
        return;
    }
    let incompatible: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM classification_maintenance_evidence WHERE json_extract(payload, '$.version') > 2)",
        [], |row| row.get(0),
    ).unwrap();
    assert!(!incompatible);
    connection.execute_batch(
        "ALTER TABLE classification_maintenance_evidence RENAME TO classification_maintenance_evidence_v73;"
    ).unwrap();
    connection
        .execute_batch(super::classification_retirement::EVIDENCE_TABLE)
        .unwrap();
    connection.execute_batch(
        "INSERT INTO classification_maintenance_evidence SELECT * FROM classification_maintenance_evidence_v73;
         DROP TABLE classification_maintenance_evidence_v73;
         UPDATE schema_info SET version = 72; PRAGMA user_version = 72;"
    ).unwrap();
}

#[cfg(test)]
mod tests;
