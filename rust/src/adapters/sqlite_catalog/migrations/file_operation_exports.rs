use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::file_operation_journal::invalid;

use super::schema_shape;

pub(super) const BATCHES: &str = "CREATE TABLE file_operation_exports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id TEXT NOT NULL UNIQUE CHECK(length(batch_id) BETWEEN 1 AND 256),
    authorized_unix_ms INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('copy', 'move')),
    rule TEXT NOT NULL CHECK(rule IN ('direct', 'category', 'album', 'date', 'source')),
    selected_files INTEGER NOT NULL CHECK(selected_files > 0),
    root_path TEXT NOT NULL CHECK(length(root_path) BETWEEN 1 AND 65536),
    identity_scheme TEXT NOT NULL CHECK(length(identity_scheme) BETWEEN 1 AND 256),
    identity_value TEXT NOT NULL CHECK(length(identity_value) BETWEEN 1 AND 4096),
    report_state TEXT NOT NULL DEFAULT 'pending' CHECK(report_state IN ('pending', 'not_needed', 'written', 'failed')),
    report_path TEXT,
    report_error_code TEXT,
    report_error_message TEXT,
    CHECK((report_state = 'written') = (report_path IS NOT NULL)),
    CHECK((report_state = 'failed') = (report_error_code IS NOT NULL AND report_error_message IS NOT NULL))
)";
const SKIPS: &str = "CREATE TABLE file_operation_export_skips (
    batch_id TEXT NOT NULL REFERENCES file_operation_exports(batch_id),
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    source_path TEXT NOT NULL CHECK(length(source_path) BETWEEN 1 AND 65536),
    reason TEXT NOT NULL CHECK(length(reason) BETWEEN 1 AND 4096),
    PRIMARY KEY(batch_id, ordinal)
) WITHOUT ROWID";
const AUTHORITY: &str = "CREATE TRIGGER file_operation_export_authority_immutable
    BEFORE UPDATE OF batch_id, authorized_unix_ms, action, rule, selected_files, root_path, identity_scheme, identity_value
    ON file_operation_exports BEGIN SELECT RAISE(ABORT, 'export authority is immutable'); END";
const RETAIN: &str = "CREATE TRIGGER file_operation_exports_retained
    BEFORE DELETE ON file_operation_exports BEGIN SELECT RAISE(ABORT, 'export history is durable'); END";
const SKIP_IMMUTABLE: &str = "CREATE TRIGGER file_operation_export_skips_immutable
    BEFORE UPDATE ON file_operation_export_skips BEGIN SELECT RAISE(ABORT, 'export skips are immutable'); END";
const SKIP_RETAIN: &str = "CREATE TRIGGER file_operation_export_skips_retained
    BEFORE DELETE ON file_operation_export_skips BEGIN SELECT RAISE(ABORT, 'export skips are durable'); END";
pub(super) const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "file_operation_exports", BATCHES),
    ("table", "file_operation_export_skips", SKIPS),
    (
        "trigger",
        "file_operation_export_authority_immutable",
        AUTHORITY,
    ),
    ("trigger", "file_operation_exports_retained", RETAIN),
    (
        "trigger",
        "file_operation_export_skips_immutable",
        SKIP_IMMUTABLE,
    ),
    (
        "trigger",
        "file_operation_export_skips_retained",
        SKIP_RETAIN,
    ),
];

pub(super) fn validate(connection: &Connection, version: i64) -> Result<(), ScanError> {
    for (kind, name, sql) in OBJECTS {
        let expanded;
        let expected = if version >= 82 && *name == "file_operation_exports" {
            expanded = super::album_year_exports::batches_schema();
            expanded.as_str()
        } else {
            sql
        };
        if !schema_shape::schema_object_sql_matches(connection, kind, name, expected)? {
            return Err(invalid(format!(
                "The export history schema is invalid: {name}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {}
