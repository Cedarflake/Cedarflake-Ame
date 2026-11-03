use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

#[cfg(test)]
use super::{classification_maintenance, schema_shape};
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
                AND json_array_length(payload, '$.retirements') BETWEEN 1 AND 64))
        AND length(CAST(payload AS BLOB)) BETWEEN 1 AND 524288)
) STRICT, WITHOUT ROWID";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 71)?;
    transaction.execute_batch(
        "ALTER TABLE classification_maintenance_evidence RENAME TO classification_maintenance_evidence_v71;",
    ).map_err(database_error)?;
    transaction
        .execute_batch(EVIDENCE_TABLE)
        .map_err(database_error)?;
    transaction.execute_batch(
        "INSERT INTO classification_maintenance_evidence SELECT * FROM classification_maintenance_evidence_v71;
         DROP TABLE classification_maintenance_evidence_v71;
         UPDATE schema_info SET version = 72; PRAGMA user_version = 72;",
    ).map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 72)
}

#[cfg(test)]
pub(super) fn downgrade_to_v71_for_test(connection: &Connection) {
    super::classification_merge::downgrade_to_v72_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() != 72 {
        return;
    }
    let incompatible: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM classification_maintenance_evidence WHERE json_extract(payload, '$.version') <> 1)",
        [], |row| row.get(0),
    ).unwrap();
    assert!(!incompatible);
    connection.execute_batch(
        "ALTER TABLE classification_maintenance_evidence RENAME TO classification_maintenance_evidence_v72;",
    ).unwrap();
    connection
        .execute_batch(classification_maintenance::EVIDENCE_TABLE_V71)
        .unwrap();
    connection.execute_batch(
        "INSERT INTO classification_maintenance_evidence SELECT * FROM classification_maintenance_evidence_v72;
         DROP TABLE classification_maintenance_evidence_v72;
         UPDATE schema_info SET version = 71; PRAGMA user_version = 71;",
    ).unwrap();
}

#[cfg(test)]
mod tests;
