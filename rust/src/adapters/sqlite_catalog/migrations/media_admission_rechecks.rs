use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;
use super::{current_schema, database_error};

const TABLE: &str = "library_media_admission_rechecks";
const DDL: &str = "CREATE TABLE library_media_admission_rechecks (
    root_id TEXT NOT NULL,
    root_generation INTEGER NOT NULL CHECK(root_generation > 0),
    relative_path TEXT NOT NULL,
    location_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    file_identity_scheme TEXT,
    file_identity_value TEXT,
    state TEXT NOT NULL CHECK(state IN ('pending', 'queued', 'rejected')),
    change_id INTEGER,
    PRIMARY KEY(root_id, relative_path),
    CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL)),
    FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE,
    FOREIGN KEY(asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE SET NULL
)";
const INDEX: &str = "CREATE INDEX library_media_admission_rechecks_pending
    ON library_media_admission_rechecks(state, root_id, relative_path)
    WHERE state <> 'rejected'";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    super::terminal_inventory_repair::apply(transaction, 37)?;
    super::retired_live_gap_repair::apply(transaction, 37)?;
    super::retired_root_authority::repair_for_schema(transaction, 37)?;
    current_schema::validate_schema_version(transaction, 37)?;
    transaction.execute_batch(DDL).map_err(database_error)?;
    transaction.execute_batch(INDEX).map_err(database_error)?;
    transaction.execute_batch(
        "INSERT INTO library_media_admission_rechecks(
            root_id, root_generation, relative_path, location_id, asset_id,
            file_identity_scheme, file_identity_value, state)
         SELECT location.root_id, root.generation, location.relative_path,
                location.location_id, location.asset_id,
                location.file_identity_scheme, location.file_identity_value, 'pending'
         FROM asset_locations AS location
         JOIN library_roots AS library
           ON library.id = location.root_id AND library.active_scan_id = location.scan_id
         JOIN library_change_root_state AS root ON root.root_id = library.id
         WHERE root.is_active = 1 AND location.width = 0 AND location.height = 0
           AND location.preview_status = 'failed' AND location.preview_path = ''
           AND location.preview_issue_code IN ('image_format_unsupported', 'media_type_unsupported');
         UPDATE schema_info SET version = 38;
         PRAGMA user_version = 38;",
    ).map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 38)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_object_sql_matches(connection, "table", TABLE, DDL)?
        || !schema_object_sql_matches(
            connection,
            "index",
            "library_media_admission_rechecks_pending",
            INDEX,
        )?
    {
        return Err(ScanError::new(
            "catalog_media_admission_rechecks_invalid",
            "Legacy media rechecks require their versioned table and pending-work index",
        ));
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let invalid: bool = connection
        .query_row(
            "SELECT EXISTS (
            SELECT 1 FROM library_media_admission_rechecks AS recheck
            LEFT JOIN library_roots AS root ON root.id = recheck.root_id
            LEFT JOIN assets AS asset ON asset.id = recheck.asset_id
            LEFT JOIN library_change_queue AS work ON work.id = recheck.change_id
            WHERE root.id IS NULL OR asset.id IS NULL
              OR length(recheck.relative_path) = 0 OR length(recheck.location_id) = 0
              OR (recheck.change_id IS NOT NULL AND
                  (work.id IS NULL OR work.root_id <> recheck.root_id
                   OR work.root_generation <> recheck.root_generation))
        )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if invalid {
        return Err(ScanError::new(
            "catalog_media_admission_rechecks_invalid",
            "A media recheck must retain its asset and current queue ownership",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
