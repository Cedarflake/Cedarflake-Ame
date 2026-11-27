use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;

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

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in OBJECTS {
        require_object(connection, kind, name, ddl)?;
    }
    Ok(())
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
