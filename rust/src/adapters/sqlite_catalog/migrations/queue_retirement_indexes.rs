use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;

const INDEXES: &[(&str, &str)] = &[
    (
        "library_change_queue_superseded_by",
        "CREATE INDEX library_change_queue_superseded_by
         ON library_change_queue(superseded_by_change_id)",
    ),
    (
        "library_media_admission_rechecks_change",
        "CREATE INDEX library_media_admission_rechecks_change
         ON library_media_admission_rechecks(change_id)",
    ),
    (
        "library_media_admission_rechecks_asset",
        "CREATE INDEX library_media_admission_rechecks_asset
         ON library_media_admission_rechecks(asset_id)",
    ),
];

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (name, ddl) in INDEXES {
        require_index(connection, name, ddl)?;
    }
    Ok(())
}

fn require_index(connection: &Connection, name: &str, ddl: &str) -> Result<(), ScanError> {
    if schema_object_sql_matches(connection, "index", name, ddl)? {
        return Ok(());
    }
    Err(ScanError::new(
        "catalog_queue_retirement_index_invalid",
        "Queue and media retirement require their bounded ownership indexes",
    ))
}

#[cfg(test)]
mod tests;
