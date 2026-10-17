use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;
use super::{current_schema, database_error};

const TABLE_NAME: &str = "scan_directory_identities";
const TABLE_DDL: &str = "CREATE TABLE scan_directory_identities (
    scan_id TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    identity_scheme TEXT NOT NULL CHECK(identity_scheme = 'windows-file-id-128-v1'),
    identity_value TEXT NOT NULL CHECK(
        length(identity_value) = 49 AND substr(identity_value, 17, 1) = ':'
        AND substr(identity_value, 1, 16) NOT GLOB '*[^0-9a-f]*'
        AND substr(identity_value, 18, 32) NOT GLOB '*[^0-9a-f]*'
    ),
    PRIMARY KEY(scan_id, relative_path),
    FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE
)";
const INDEX_NAME: &str = "scan_directory_identities_by_identity";
const INDEX_DDL: &str = "CREATE INDEX scan_directory_identities_by_identity
    ON scan_directory_identities(scan_id, identity_value, relative_path)";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    super::repair_prerelease_v24_source_range_id_triggers(connection)?;
    super::repair_prerelease_v26_recovery_window_schema(connection)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    super::repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 34)?;
    super::repair_retired_live_gap_claims_transaction(transaction, 34)?;
    super::retired_root_authority::repair_for_schema(transaction, 34)?;
    current_schema::validate_schema_version(transaction, 34)?;
    for (kind, name, ddl) in [
        ("table", TABLE_NAME, TABLE_DDL),
        ("index", INDEX_NAME, INDEX_DDL),
    ] {
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if exists {
            if !schema_object_sql_matches(transaction, kind, name, ddl)? {
                return Err(contract_error());
            }
        } else {
            transaction.execute_batch(ddl).map_err(database_error)?;
        }
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 35; PRAGMA user_version = 35;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 35)
}

pub(super) fn validate_structure(connection: &Connection) -> Result<(), ScanError> {
    if !schema_object_sql_matches(connection, "table", TABLE_NAME, TABLE_DDL)?
        || !schema_object_sql_matches(connection, "index", INDEX_NAME, INDEX_DDL)?
    {
        return Err(contract_error());
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let orphan_exists: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM scan_directory_identities AS directory
                LEFT JOIN scan_runs AS scan ON scan.id = directory.scan_id
                WHERE scan.id IS NULL
            )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if orphan_exists {
        return Err(contract_error());
    }
    Ok(())
}

fn contract_error() -> ScanError {
    ScanError::new(
        "catalog_directory_identity_contract_invalid",
        "Retained directory identities require their canonical schema and scan ownership",
    )
}

#[cfg(test)]
mod tests;
