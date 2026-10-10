use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_object_sql_matches};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "scan_resume_pending_locations",
        "CREATE TABLE scan_resume_pending_locations (
            scan_id TEXT NOT NULL,
            location_id TEXT NOT NULL,
            PRIMARY KEY(scan_id, location_id),
            FOREIGN KEY(scan_id, location_id)
                REFERENCES asset_locations(scan_id, location_id) ON DELETE CASCADE
        ) WITHOUT ROWID",
    ),
    (
        "trigger",
        "scan_resume_pending_insert_guard",
        "CREATE TRIGGER scan_resume_pending_insert_guard
         BEFORE INSERT ON scan_resume_pending_locations
         WHEN NOT EXISTS (
             SELECT 1 FROM scan_runs AS scan
             JOIN library_roots AS root ON root.id = scan.root_id
             JOIN library_change_root_state AS state ON state.root_id = root.id
             WHERE scan.id = NEW.scan_id AND scan.scan_owner = 'foreground'
               AND scan.status = 'running' AND root.active_scan_id IS NULL
               AND state.is_active = 1 AND state.generation = scan.root_generation_at_start
         )
         BEGIN SELECT RAISE(ABORT, 'retained import has no current resume authority'); END",
    ),
    (
        "trigger",
        "scan_resume_pending_update_guard",
        "CREATE TRIGGER scan_resume_pending_update_guard
         BEFORE UPDATE ON scan_resume_pending_locations
         BEGIN SELECT RAISE(ABORT, 'retained import membership is immutable'); END",
    ),
    (
        "trigger",
        "scan_resume_publication_guard",
        "CREATE TRIGGER scan_resume_publication_guard
         BEFORE UPDATE OF status ON scan_runs
         WHEN NEW.status = 'completed' AND EXISTS (
             SELECT 1 FROM scan_resume_pending_locations WHERE scan_id = NEW.id
         )
         BEGIN SELECT RAISE(ABORT, 'retained import namespace has not been reconciled'); END",
    ),
];

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
    super::repair_legacy_terminal_metadata_inventory_state_transaction(transaction, 32)?;
    super::repair_retired_live_gap_claims_transaction(transaction, 32)?;
    super::retired_root_authority::repair_for_schema(transaction, 32)?;
    current_schema::validate_schema_version(transaction, 32)?;
    if !super::source_revision_metadata_contract_is_complete(transaction)? {
        super::invalidate_precontract_source_revision_metadata(transaction)?;
        super::create_source_revision_metadata_contract(transaction)?;
    }
    for (_, _, ddl) in OBJECTS {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 33; PRAGMA user_version = 33;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 33)
}

pub(super) fn validate_structure(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in OBJECTS {
        if !schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid_contract());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid: bool = connection
        .query_row(
            "SELECT EXISTS (
                SELECT 1 FROM scan_resume_pending_locations AS pending
                LEFT JOIN asset_locations AS location
                  ON location.scan_id = pending.scan_id AND location.location_id = pending.location_id
                LEFT JOIN scan_runs AS scan ON scan.id = pending.scan_id
                LEFT JOIN library_roots AS root ON root.id = scan.root_id
                LEFT JOIN library_change_root_state AS state ON state.root_id = root.id
                WHERE location.location_id IS NULL OR scan.scan_owner <> 'foreground'
                   OR scan.status NOT IN ('running', 'paused') OR root.active_scan_id IS NOT NULL
                   OR state.is_active IS NOT 1 OR state.generation IS NOT scan.root_generation_at_start
            )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if invalid {
        return Err(invalid_contract());
    }
    Ok(())
}

fn invalid_contract() -> ScanError {
    ScanError::new(
        "catalog_scan_resume_inventory_unverifiable",
        "The catalog cannot prove retained first-import membership and publication isolation",
    )
}

#[cfg(test)]
pub(in crate::adapters::sqlite_catalog::migrations) fn downgrade_to_v32_for_test(
    connection: &Connection,
) {
    for (kind, name, _) in OBJECTS.iter().rev() {
        connection
            .execute_batch(&format!("DROP {kind} {name}"))
            .expect("remove v33 fixture schema");
    }
    connection
        .execute_batch("UPDATE schema_info SET version = 32; PRAGMA user_version = 32;")
        .expect("retain a v32 fixture");
}

#[cfg(test)]
mod tests;
