use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::OrganizationGroupKey;

use super::super::{database_error, sqlite_integer};

mod decision;

pub(super) struct ResolvedRecord {
    pub(super) record_id: String,
    pub(super) merged: bool,
    pub(super) has_content_binding: bool,
}

pub(super) fn resolve_members(
    connection: &Connection,
    key: &OrganizationGroupKey,
) -> Result<Option<ResolvedRecord>, ScanError> {
    prepare_groups(connection)?;
    // Temporary membership sets have no persistent planner statistics. Keep their bounded rows
    // ahead of durable history so each published copy uses point lookups rather than history scans.
    connection.execute(
        "INSERT OR IGNORE INTO temp.organization_merge_groups
         SELECT mapping.group_id FROM temp.organization_command_members AS member
         CROSS JOIN organization_bindings AS binding
           ON member.asset_id = binding.asset_id AND member.source_generation = binding.source_generation
         CROSS JOIN organization_record_groups AS mapping ON mapping.record_id = binding.record_id", [],
    ).map_err(database_error)?;
    let preferred: Option<String> = match key {
        OrganizationGroupKey::Confirmed { digest, byte_length } => connection.query_row(
            "SELECT mapping.group_id FROM organization_bindings AS binding
             JOIN organization_record_groups AS mapping ON mapping.record_id = binding.record_id
             WHERE binding.fingerprint_scheme = ?1 AND binding.digest = ?2 AND binding.byte_length = ?3
             LIMIT 1",
            params![EXACT_FINGERPRINT_SCHEME, digest, sqlite_integer(*byte_length, "content byte length")?],
            |row| row.get(0),
        ).optional().map_err(database_error)?,
        OrganizationGroupKey::Provisional { .. } => None,
    };
    if let Some(record_id) = &preferred {
        connection
            .execute(
                "INSERT OR IGNORE INTO temp.organization_merge_groups VALUES (?1)",
                [record_id],
            )
            .map_err(database_error)?;
    }
    merge(connection, preferred.as_deref())
}

pub(super) fn prepare_groups(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_merge_groups (
           group_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_merge_groups;",
        )
        .map_err(database_error)
}

pub(super) fn merge(
    connection: &Connection,
    preferred: Option<&str>,
) -> Result<Option<ResolvedRecord>, ScanError> {
    let record_id: Option<String> = connection
        .query_row(
            "SELECT record.record_id FROM temp.organization_merge_groups AS selected
         CROSS JOIN organization_records AS record ON selected.group_id = record.record_id
         ORDER BY record.record_id = ?1 DESC, record.created_unix_ms, record.record_id LIMIT 1",
            [preferred],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(record_id) = record_id else {
        return Ok(None);
    };
    let multiple: bool = connection
        .query_row(
            "SELECT COUNT(*) > 1 FROM temp.organization_merge_groups",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if multiple {
        decision::merge(connection, &record_id)?;
        connection.execute(
            "UPDATE organization_record_groups SET group_id = ?1
             WHERE group_id <> ?1 AND group_id IN (SELECT group_id FROM temp.organization_merge_groups)",
            [&record_id],
        ).map_err(database_error)?;
    }
    Ok(Some(ResolvedRecord {
        record_id,
        merged: multiple,
        has_content_binding: preferred.is_some(),
    }))
}

pub(in crate::adapters::sqlite_catalog) fn reconcile_known_bindings(
    connection: &Connection,
) -> Result<(), ScanError> {
    let mut after: Option<(String, String, i64)> = None;
    loop {
        let next: Option<(String, String, i64)> = connection
            .query_row(
                "SELECT fingerprint_scheme, digest, byte_length FROM organization_bindings
             WHERE digest IS NOT NULL AND (fingerprint_scheme, digest, byte_length) > (?1, ?2, ?3)
             ORDER BY fingerprint_scheme, digest, byte_length LIMIT 1",
                params![
                    after.as_ref().map_or("", |key| key.0.as_str()),
                    after.as_ref().map_or("", |key| key.1.as_str()),
                    after.as_ref().map_or(-1, |key| key.2)
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(database_error)?;
        let Some(key) = next else {
            break;
        };
        prepare_groups(connection)?;
        connection.execute(
            "INSERT OR IGNORE INTO temp.organization_merge_groups
             SELECT mapping.group_id FROM organization_bindings AS binding
             JOIN organization_record_groups AS mapping ON mapping.record_id = binding.record_id
             WHERE binding.fingerprint_scheme = ?1 AND binding.digest = ?2 AND binding.byte_length = ?3",
            params![key.0, key.1, key.2],
        ).map_err(database_error)?;
        merge(connection, None)?;
        after = Some(key);
    }
    Ok(())
}
