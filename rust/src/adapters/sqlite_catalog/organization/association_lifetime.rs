use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::domain::ScanError;

use super::super::{database_error, unix_time_ms};

const WINDOW: usize = 256;

pub(super) fn restore_generation(
    connection: &Connection,
    source_generation: i64,
) -> Result<(), ScanError> {
    let members = unassociated_members(connection)?;
    connection.execute(
        &format!("UPDATE organization_records SET unassociated_since_unix_ms = NULL
         WHERE unassociated_since_unix_ms IS NOT NULL AND record_id IN (
           SELECT related.record_id FROM asset_locations AS location
           CROSS JOIN organization_bindings AS binding ON binding.asset_id = location.asset_id
             AND binding.source_generation = location.source_generation
           JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
           JOIN organization_record_groups AS own ON own.record_id = binding.record_id
           JOIN {members} AS related ON related.group_id = own.group_id
           WHERE location.source_generation = ?1
             AND location.file_size = binding.byte_length
             AND location.file_identity_scheme IS binding.file_identity_scheme
             AND location.file_identity_value IS binding.file_identity_value
             AND location.source_revision_token IS binding.source_revision_token)"),
        [source_generation],
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn restore_group(connection: &Connection, record_id: &str) -> Result<(), ScanError> {
    let members = unassociated_members(connection)?;
    connection
        .prepare_cached(&format!(
            "UPDATE organization_records SET unassociated_since_unix_ms = NULL
         WHERE unassociated_since_unix_ms IS NOT NULL AND record_id IN (
           SELECT related.record_id FROM organization_record_groups AS own
           JOIN {members} AS related ON related.group_id = own.group_id
           WHERE own.record_id = ?1)"
        ))
        .map_err(database_error)?
        .execute([record_id])
        .map_err(database_error)?;
    Ok(())
}

pub(in crate::adapters::sqlite_catalog) fn location_group(
    connection: &Connection,
    scan_id: &str,
    root_id: &str,
    location_id: &str,
) -> Result<Option<String>, ScanError> {
    connection
        .query_row(
            "SELECT member.group_id FROM asset_locations AS location
             JOIN organization_bindings AS binding
               ON binding.asset_id = location.asset_id
                 AND binding.source_generation = location.source_generation
             JOIN organization_record_groups AS member ON member.record_id = binding.record_id
             WHERE location.scan_id = ?1 AND location.root_id = ?2 AND location.location_id = ?3",
            params![scan_id, root_id, location_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)
}

pub(in crate::adapters::sqlite_catalog) fn refresh_root(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_association_root (
           group_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_association_root;",
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT OR IGNORE INTO temp.organization_association_root
         SELECT member.group_id FROM organization_sources AS source
         JOIN organization_record_groups AS member ON member.record_id = source.record_id
         WHERE source.root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    let mut after = String::new();
    loop {
        let groups = connection
            .prepare(
                "SELECT group_id FROM temp.organization_association_root
                 WHERE group_id > ?1 ORDER BY group_id LIMIT ?2",
            )
            .map_err(database_error)?
            .query_map(params![after, WINDOW as i64], |row| row.get::<_, String>(0))
            .map_err(database_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database_error)?;
        let Some(last) = groups.last() else {
            return Ok(());
        };
        after = last.clone();
        refresh_groups(connection, &groups, unix_time_ms())?;
    }
}

pub(in crate::adapters::sqlite_catalog) fn refresh_groups(
    connection: &Connection,
    groups: &[String],
    observed_unix_ms: i64,
) -> Result<(), ScanError> {
    if groups.is_empty() {
        return Ok(());
    }
    if groups.len() > WINDOW || observed_unix_ms < 0 {
        return Err(ScanError::new(
            "organization_association_window_invalid",
            "The association update exceeds its bounded window",
        ));
    }
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_association_window (
               group_id TEXT PRIMARY KEY NOT NULL, associated INTEGER NOT NULL DEFAULT 0
             ) WITHOUT ROWID;
             DELETE FROM temp.organization_association_window;",
        )
        .map_err(database_error)?;
    let placeholders = vec!["(?)"; groups.len()].join(", ");
    connection
        .execute(
            &format!(
                "INSERT INTO temp.organization_association_window(group_id) VALUES {placeholders}"
            ),
            params_from_iter(groups),
        )
        .map_err(database_error)?;
    connection
        .execute_batch(
            "UPDATE temp.organization_association_window AS selected SET associated = EXISTS(
               SELECT 1 FROM organization_record_groups AS member
               CROSS JOIN organization_bindings AS binding ON binding.record_id = member.record_id
               CROSS JOIN asset_locations AS location
                 ON location.asset_id = binding.asset_id
                   AND location.source_generation = binding.source_generation
               JOIN library_roots AS root ON root.id = location.root_id
                 AND root.active_scan_id = location.scan_id
               WHERE member.group_id = selected.group_id
                 AND location.file_size = binding.byte_length
                 AND location.file_identity_scheme IS binding.file_identity_scheme
                 AND location.file_identity_value IS binding.file_identity_value
                 AND location.source_revision_token IS binding.source_revision_token
             );",
        )
        .map_err(database_error)?;
    let members = unassociated_members(connection)?;
    connection
        .execute(
            &format!(
                "UPDATE organization_records SET unassociated_since_unix_ms = NULL
         WHERE unassociated_since_unix_ms IS NOT NULL AND record_id IN (
           SELECT member.record_id FROM temp.organization_association_window AS selected
           CROSS JOIN {members} AS member USING(group_id)
           WHERE selected.associated = 1)"
            ),
            [],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "UPDATE organization_records SET unassociated_since_unix_ms = ?1
         WHERE unassociated_since_unix_ms IS NULL AND record_id IN (
           SELECT member.record_id FROM temp.organization_association_window AS selected
           CROSS JOIN organization_record_groups AS member USING(group_id)
           WHERE selected.associated = 0)",
            [observed_unix_ms],
        )
        .map_err(database_error)?;
    Ok(())
}

fn unassociated_members(connection: &Connection) -> Result<&'static str, ScanError> {
    let version: i64 = connection
        .prepare_cached("SELECT version FROM schema_info")
        .map_err(database_error)?
        .query_row([], |row| row.get(0))
        .map_err(database_error)?;
    // Historical organization migrations publish before the lookup is installed. Current
    // catalogs must use the maintained lookup; a missing current table remains an error.
    Ok(if version < 55 {
        "organization_record_groups"
    } else {
        "organization_unassociated_records"
    })
}
