use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::super::organization::{publication, reconciliation};
use super::{current_schema, database_error};

pub(super) const GENERATION_INDEX: (&str, &str) = (
    "asset_locations_organization_generation",
    "CREATE INDEX asset_locations_organization_generation
       ON asset_locations(source_generation, root_id, scan_id)",
);

pub(super) const ROOT_GENERATION_INDEX: (&str, &str) = (
    "asset_locations_organization_root_generation",
    "CREATE INDEX asset_locations_organization_root_generation
       ON asset_locations(root_id, scan_id, source_generation)",
);

const INDEXES: &[(&str, &str)] = &[GENERATION_INDEX, ROOT_GENERATION_INDEX];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 48)?;
    for (_, ddl) in INDEXES {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    reconciliation::reconcile_known_bindings(transaction)?;
    let mut after = String::new();
    loop {
        let root: Option<String> = transaction
            .query_row(
                "SELECT id FROM library_roots WHERE id > ?1 ORDER BY id LIMIT 1",
                [&after],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?;
        let Some(root) = root else {
            break;
        };
        publication::root(transaction, &root)?;
        after = root;
    }
    // Earlier organization versions could retain independent decisions for already published
    // identical content. Normalize them once; normal startup only validates the resulting mapping.
    transaction.execute_batch(
        "UPDATE catalog_state SET revision = revision + 1 WHERE EXISTS(SELECT 1 FROM organization_records);
         UPDATE schema_info SET version = 49; PRAGMA user_version = 49;"
    ).map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 49)
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_bindings AS binding
           JOIN organization_record_groups AS member ON member.record_id = binding.record_id
           WHERE binding.digest IS NOT NULL
           GROUP BY binding.fingerprint_scheme, binding.digest, binding.byte_length
           HAVING MIN(member.group_id) <> MAX(member.group_id))
         OR EXISTS(SELECT 1 FROM organization_conflicts AS conflict
           JOIN organization_record_groups AS member ON member.record_id = conflict.record_id
           JOIN organization_records AS record ON record.record_id = member.group_id
           WHERE member.record_id <> member.group_id OR record.disposition <> 'deferred' OR record.album_id IS NOT NULL)",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid {
        return Err(ScanError::new(
            "catalog_organization_reconciliation_invalid",
            "Organization groups require coherent content bindings and deferred conflicts",
        ));
    }
    Ok(())
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (name, ddl) in INDEXES {
        if !super::schema_shape::schema_object_sql_matches(connection, "index", name, ddl)? {
            return Err(ScanError::new(
                "catalog_organization_reconciliation_invalid",
                "Organization publication requires its bounded generation indexes",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_to_v48_for_test(connection: &Connection) {
    super::organization_index_retirement::downgrade_to_v49_for_test(connection);
    if super::schema_shape::schema_version(connection).unwrap() == 49 {
        for (name, _) in INDEXES {
            connection
                .execute_batch(&format!("DROP INDEX {name}"))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 48; PRAGMA user_version = 48;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;
