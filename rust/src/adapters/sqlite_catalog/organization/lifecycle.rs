use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::database_error;

pub(in crate::adapters::sqlite_catalog) fn forget_source(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_removed_source_groups (
           group_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_removed_source_groups;",
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT OR IGNORE INTO temp.organization_removed_source_groups
         SELECT member.group_id FROM organization_sources AS source
         JOIN organization_record_groups AS member ON member.record_id = source.record_id
         WHERE source.root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    let removed = connection
        .execute(
            "DELETE FROM organization_sources WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "DELETE FROM temp.organization_removed_source_groups AS removed
         WHERE EXISTS (
             SELECT 1 FROM organization_record_groups AS surviving
             JOIN organization_sources AS source ON source.record_id = surviving.record_id
             JOIN library_roots AS root ON root.id = source.root_id
             WHERE surviving.group_id = removed.group_id
         )",
            [],
        )
        .map_err(database_error)?;
    connection.execute(
        "DELETE FROM organization_records WHERE record_id IN (
           SELECT member.record_id FROM organization_record_groups AS member
           JOIN temp.organization_removed_source_groups AS removed ON removed.group_id = member.group_id
         )", [],
    ).map_err(database_error)?;
    connection
        .execute("DELETE FROM temp.organization_removed_source_groups", [])
        .map_err(database_error)?;
    if removed > 0 {
        connection
            .execute(
                "UPDATE organization_state SET revision = revision + 1 WHERE singleton = 1",
                [],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

pub(in crate::adapters::sqlite_catalog) fn transfer_source(
    connection: &Connection,
    previous_root: &str,
    destination_root: &str,
) -> Result<(), ScanError> {
    let destination_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?1)",
            [destination_root],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if previous_root == destination_root || !destination_exists {
        return Err(ScanError::new(
            "organization_source_changed",
            "The replacement library source is no longer available",
        ));
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO organization_sources(root_id, record_id)
         SELECT ?2, record_id FROM organization_sources WHERE root_id = ?1",
            params![previous_root, destination_root],
        )
        .map_err(database_error)?;
    Ok(())
}
