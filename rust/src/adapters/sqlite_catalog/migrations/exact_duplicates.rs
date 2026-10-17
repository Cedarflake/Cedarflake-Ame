use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_object_sql_matches};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "exact_analysis_runs",
        "CREATE TABLE exact_analysis_runs (
        run_id TEXT PRIMARY KEY NOT NULL,
        scheme TEXT NOT NULL,
        engine_id TEXT NOT NULL,
        engine_version TEXT NOT NULL,
        parameters TEXT NOT NULL,
        started_unix_ms INTEGER NOT NULL
    )",
    ),
    (
        "table",
        "exact_analysis_outcomes",
        "CREATE TABLE exact_analysis_outcomes (
        run_id TEXT PRIMARY KEY NOT NULL REFERENCES exact_analysis_runs(run_id),
        phase TEXT NOT NULL CHECK(phase IN ('completed', 'cancelled', 'failed')),
        checked_files INTEGER NOT NULL CHECK(checked_files >= 0),
        fingerprinted_files INTEGER NOT NULL CHECK(fingerprinted_files >= 0),
        skipped_files INTEGER NOT NULL CHECK(skipped_files >= 0),
        bytes_read INTEGER NOT NULL CHECK(bytes_read >= 0),
        issue_code TEXT,
        finished_unix_ms INTEGER NOT NULL
    )",
    ),
    (
        "table",
        "content_fingerprints",
        "CREATE TABLE content_fingerprints (
        source_generation INTEGER NOT NULL CHECK(source_generation > 0),
        scheme TEXT NOT NULL,
        digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        byte_length INTEGER NOT NULL CHECK(byte_length > 0),
        file_identity_scheme TEXT,
        file_identity_value TEXT,
        source_revision_token TEXT,
        run_id TEXT NOT NULL REFERENCES exact_analysis_runs(run_id),
        PRIMARY KEY(source_generation, scheme),
        CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL))
    )",
    ),
    (
        "index",
        "content_fingerprints_content",
        "CREATE INDEX content_fingerprints_content
        ON content_fingerprints(scheme, digest, byte_length, source_generation)",
    ),
    (
        "index",
        "asset_locations_exact_size",
        "CREATE INDEX asset_locations_exact_size
        ON asset_locations(file_size, location_id, root_id, scan_id)",
    ),
    (
        "index",
        "asset_locations_exact_source",
        "CREATE INDEX asset_locations_exact_source
        ON asset_locations(source_generation, location_id, root_id, scan_id)",
    ),
];

pub(super) const LEGACY_CURSOR_INDEX: (&str, &str, &str) = (
    "index",
    "asset_locations_exact_cursor",
    "CREATE INDEX asset_locations_exact_cursor
        ON asset_locations(location_id, root_id, scan_id, file_size, source_generation)",
);

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 39)?;
    for (kind, name, ddl) in OBJECTS.iter().chain(std::iter::once(&LEGACY_CURSOR_INDEX)) {
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if exists {
            require_object(transaction, kind, name, ddl)?;
        } else {
            transaction.execute_batch(ddl).map_err(database_error)?;
        }
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 40; PRAGMA user_version = 40;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 40)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in OBJECTS {
        require_object(connection, kind, name, ddl)?;
    }
    Ok(())
}

pub(super) fn validate_legacy_cursor(connection: &Connection) -> Result<(), ScanError> {
    let (kind, name, ddl) = LEGACY_CURSOR_INDEX;
    require_object(connection, kind, name, ddl)
}

fn require_object(
    connection: &Connection,
    kind: &str,
    name: &str,
    ddl: &str,
) -> Result<(), ScanError> {
    if schema_object_sql_matches(connection, kind, name, ddl)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_exact_analysis_contract_invalid",
        "Content analysis requires its versioned evidence tables and bounded query indexes",
    ))
}

#[cfg(test)]
pub(crate) fn downgrade_to_v39_for_test(connection: &Connection) {
    let version: i64 = connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("exact analysis fixture version");
    if !matches!(version, 40..=super::SCHEMA_VERSION) {
        return;
    }
    let transaction = connection
        .unchecked_transaction()
        .expect("exact analysis fixture downgrade");
    if version >= 42 {
        transaction
            .execute_batch("DROP TABLE exact_duplicate_decisions")
            .expect("remove test-only duplicate choices");
    }
    if version >= 41 {
        transaction
            .execute_batch("DROP TABLE gallery_content_fingerprints")
            .expect("remove published gallery evidence");
    }
    if version <= 42 {
        transaction
            .execute_batch("DROP INDEX asset_locations_exact_cursor")
            .expect("remove legacy exact candidate cursor");
    }
    for (kind, name, _) in OBJECTS.iter().rev() {
        transaction
            .execute_batch(&format!("DROP {kind} {name}"))
            .expect("remove exact analysis fixture schema");
    }
    transaction
        .execute_batch("UPDATE schema_info SET version = 39; PRAGMA user_version = 39;")
        .expect("restore v39 fixture version");
    transaction.commit().expect("publish v39 fixture");
}

#[cfg(test)]
mod tests;
