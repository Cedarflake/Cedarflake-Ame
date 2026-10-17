use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::super::MAX_SCAN_CATCH_UP_LINEAGE;
use super::current_schema::ContractValidationDepth;
use super::schema_shape::{schema_version, table_columns_match};
use super::{database_error, record_current_schema_row_audit};

pub(super) fn validate_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let state_columns_match = table_columns_match(
        connection,
        "library_change_catch_up_state",
        &[
            ("volume_id", "TEXT", false, 1),
            ("journal_id", "TEXT", true, 0),
            ("next_usn", "TEXT", true, 0),
            ("root_set_fingerprint", "TEXT", true, 0),
            ("catalog_revision", "INTEGER", true, 0),
            ("updated_unix_ms", "INTEGER", true, 0),
        ],
    )?;
    let mut handoff_columns = vec![
        ("catch_up_source", "TEXT", true, 1),
        ("catch_up_watermark", "TEXT", true, 2),
        ("file_identity_scheme", "TEXT", true, 3),
        ("file_identity_value", "TEXT", true, 4),
        ("asset_id", "TEXT", true, 0),
        ("source_location_id", "TEXT", true, 0),
        ("root_id", "TEXT", true, 0),
        ("absolute_path", "TEXT", true, 0),
        ("relative_path", "TEXT", true, 0),
        ("preview_path", "TEXT", true, 0),
        ("file_size", "INTEGER", true, 0),
        ("created_unix_ms", "INTEGER", false, 0),
        ("modified_unix_ms", "INTEGER", true, 0),
        ("width", "INTEGER", true, 0),
        ("height", "INTEGER", true, 0),
        ("preview_status", "TEXT", true, 0),
        ("preview_issue_code", "TEXT", false, 0),
        ("preview_issue_message", "TEXT", false, 0),
        ("metadata_engine_id", "TEXT", true, 0),
        ("metadata_engine_version", "TEXT", true, 0),
        ("capture_local_time", "TEXT", false, 0),
        ("capture_offset_minutes", "INTEGER", false, 0),
        ("capture_time_source", "TEXT", false, 0),
        ("capture_raw_value", "TEXT", false, 0),
        ("updated_unix_ms", "INTEGER", true, 0),
    ];
    if schema_version(connection)? >= 31 {
        handoff_columns.push(("source_revision_token", "TEXT", false, 0));
        handoff_columns.push(("source_generation", "INTEGER", false, 0));
    }
    let handoff_columns_match = table_columns_match(
        connection,
        "library_change_catch_up_handoffs",
        &handoff_columns,
    )?;
    let (
        has_table,
        has_marker,
        has_path_index,
        has_handoff_table,
        has_asset_index,
        has_preview_index,
        has_lineage_table,
        has_lineage_index,
        has_lineage_foreign_key,
        has_scan_lineage_marker,
        has_scan_lineage_table,
        has_scan_lineage_index,
        has_scan_lineage_foreign_key,
    ) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_state'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM pragma_index_list('asset_locations')
                 WHERE name = 'asset_locations_root_relative'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info('asset_locations_root_relative')
                 ORDER BY seqno
                )) = 'root_id,relative_path,scan_id,location_id',
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_handoffs')
               AND (SELECT COUNT(*) FROM pragma_table_info('library_change_catch_up_handoffs')
                 WHERE name IN (
                   'catch_up_source', 'catch_up_watermark', 'file_identity_scheme',
                   'file_identity_value', 'asset_id', 'source_location_id', 'root_id',
                   'absolute_path', 'relative_path', 'preview_path', 'file_size',
                   'created_unix_ms', 'modified_unix_ms', 'width', 'height',
                   'preview_status', 'preview_issue_code', 'preview_issue_message',
                   'metadata_engine_id', 'metadata_engine_version', 'capture_local_time',
                   'capture_offset_minutes', 'capture_time_source', 'capture_raw_value',
                   'updated_unix_ms', 'source_revision_token', 'source_generation'
                 )) IN (25, 27)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_table_info('library_change_catch_up_handoffs')
                 WHERE pk > 0 ORDER BY pk
               )) = 'catch_up_source,catch_up_watermark,file_identity_scheme,file_identity_value',
               EXISTS(SELECT 1 FROM pragma_index_list('library_change_catch_up_handoffs')
                 WHERE name = 'library_change_catch_up_handoffs_asset'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info('library_change_catch_up_handoffs_asset')
                 ORDER BY seqno
               )) = 'asset_id,catch_up_source,catch_up_watermark',
               EXISTS(SELECT 1 FROM pragma_index_list('library_change_catch_up_handoffs')
                 WHERE name = 'library_change_catch_up_handoffs_preview'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info('library_change_catch_up_handoffs_preview')
                 ORDER BY seqno
               )) = 'preview_path,preview_status,catch_up_source,catch_up_watermark',
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_queue_catch_up_lineage')
               AND (SELECT COUNT(*) FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               )) = 4
               AND (SELECT COUNT(*) FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name IN (
                 'change_id', 'catch_up_source', 'catch_up_watermark', 'enrolled_unix_ms'
               )) = 4
               AND EXISTS(SELECT 1 FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'change_id' AND upper(type) = 'INTEGER'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 1)
               AND EXISTS(SELECT 1 FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'catch_up_source' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 2)
               AND EXISTS(SELECT 1 FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'catch_up_watermark' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 3)
               AND EXISTS(SELECT 1 FROM pragma_table_info(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'enrolled_unix_ms' AND upper(type) = 'INTEGER'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_table_info('library_change_queue_catch_up_lineage')
                 WHERE pk > 0 ORDER BY pk
               )) = 'change_id,catch_up_source,catch_up_watermark',
               EXISTS(SELECT 1 FROM pragma_index_list(
                 'library_change_queue_catch_up_lineage'
               ) WHERE name = 'library_change_queue_catch_up_lineage_evidence'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info(
                   'library_change_queue_catch_up_lineage_evidence'
                 ) ORDER BY seqno
               )) = 'catch_up_source,catch_up_watermark,change_id',
               (SELECT COUNT(*) FROM pragma_foreign_key_list(
                 'library_change_queue_catch_up_lineage'
               )) = 1
               AND EXISTS(SELECT 1 FROM pragma_foreign_key_list(
                 'library_change_queue_catch_up_lineage'
               ) WHERE \"table\" = 'library_change_queue' AND \"from\" = 'change_id'
                   AND \"to\" = 'id' AND on_update = 'NO ACTION'
                   AND on_delete = 'CASCADE' AND \"match\" = 'NONE'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_catch_up_lineage_complete'),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'scan_run_catch_up_lineage')
               AND (SELECT COUNT(*) FROM pragma_table_info('scan_run_catch_up_lineage')) = 4
               AND (SELECT COUNT(*) FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name IN (
                   'scan_id', 'catch_up_source', 'catch_up_watermark', 'enrolled_unix_ms'
                 )) = 4
               AND EXISTS(SELECT 1 FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name = 'scan_id' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 1)
               AND EXISTS(SELECT 1 FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name = 'catch_up_source' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 2)
               AND EXISTS(SELECT 1 FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name = 'catch_up_watermark' AND upper(type) = 'TEXT'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 3)
               AND EXISTS(SELECT 1 FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE name = 'enrolled_unix_ms' AND upper(type) = 'INTEGER'
                   AND \"notnull\" = 1 AND dflt_value IS NULL AND pk = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_table_info('scan_run_catch_up_lineage')
                 WHERE pk > 0 ORDER BY pk
               )) = 'scan_id,catch_up_source,catch_up_watermark',
               EXISTS(SELECT 1 FROM pragma_index_list('scan_run_catch_up_lineage')
                 WHERE name = 'scan_run_catch_up_lineage_evidence'
                   AND \"unique\" = 0 AND partial = 0)
               AND (SELECT group_concat(name, ',') FROM (
                 SELECT name FROM pragma_index_info('scan_run_catch_up_lineage_evidence')
                 ORDER BY seqno
               )) = 'catch_up_source,catch_up_watermark,scan_id',
               (SELECT COUNT(*) FROM pragma_foreign_key_list('scan_run_catch_up_lineage')) = 1
               AND EXISTS(SELECT 1 FROM pragma_foreign_key_list(
                 'scan_run_catch_up_lineage'
               ) WHERE \"table\" = 'scan_runs' AND \"from\" = 'scan_id'
                   AND \"to\" = 'id' AND on_update = 'NO ACTION'
                   AND on_delete = 'CASCADE' AND \"match\" = 'NONE')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, bool>(5)?,
                    row.get::<_, bool>(6)?,
                    row.get::<_, bool>(7)?,
                    row.get::<_, bool>(8)?,
                    row.get::<_, bool>(9)?,
                    row.get::<_, bool>(10)?,
                    row.get::<_, bool>(11)?,
                    row.get::<_, bool>(12)?,
                ))
            },
        )
        .map_err(database_error)?;
    if !state_columns_match
        || !handoff_columns_match
        || !has_table
        || !has_marker
        || !has_path_index
        || !has_handoff_table
        || !has_asset_index
        || !has_preview_index
        || !has_lineage_table
        || !has_lineage_index
        || !has_lineage_foreign_key
        || !has_scan_lineage_marker
        || !has_scan_lineage_table
        || !has_scan_lineage_index
        || !has_scan_lineage_foreign_key
    {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its downtime catch-up checkpoint authority",
        ));
    }
    let has_scan_handoff_marker = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
             WHERE name = 'scan_handoff_batch_complete')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_scan_handoff_marker {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its normalized scan handoff contract",
        ));
    }
    let marker_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1
                    AND scan_catch_up_lineage_complete = 1
                    AND scan_handoff_batch_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !marker_complete {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its downtime catch-up checkpoint authority",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();
    let invalid_lineage = connection
        .query_row(
            "SELECT
               EXISTS(
                 SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                 LEFT JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                 WHERE changes.id IS NULL
               )
               OR EXISTS(
                 SELECT 1 FROM library_change_queue AS changes
                 WHERE (
                   (changes.catch_up_source IS NULL) <> (changes.catch_up_watermark IS NULL)
                 ) OR (
                   changes.catch_up_source IS NOT NULL
                   AND NOT EXISTS (
                     SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                     WHERE lineage.change_id = changes.id
                       AND lineage.catch_up_source = changes.catch_up_source
                       AND lineage.catch_up_watermark = changes.catch_up_watermark
                   )
                 ) OR (
                   changes.catch_up_source IS NULL
                   AND EXISTS (
                     SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                     WHERE lineage.change_id = changes.id
                   )
                 ) OR (
                   SELECT COUNT(*) FROM library_change_queue_catch_up_lineage AS lineage
                   WHERE lineage.change_id = changes.id
                 ) > 64
               )
               OR EXISTS(
                 SELECT 1 FROM scan_run_catch_up_lineage AS lineage
                 LEFT JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                 WHERE scans.id IS NULL
                    OR scans.status NOT IN ('running', 'paused')
               )
               OR EXISTS(
                 SELECT 1 FROM scan_run_catch_up_lineage
                 GROUP BY scan_id HAVING COUNT(*) > ?1
               )
               OR EXISTS(
                 SELECT 1
                 FROM scan_runs AS scans
                 JOIN library_change_queue AS changes
                   ON changes.root_id = scans.root_id
                  AND changes.root_generation = scans.root_generation_at_start
                  AND changes.id <= scans.change_queue_high_watermark
                 JOIN library_change_queue_catch_up_lineage AS lineage
                   ON lineage.change_id = changes.id
                 WHERE scans.status IN ('running', 'paused')
                   AND changes.status IN ('pending', 'leased', 'retry_wait')
                   AND NOT EXISTS (
                     SELECT 1 FROM scan_run_catch_up_lineage AS frozen
                     WHERE frozen.scan_id = scans.id
                       AND frozen.catch_up_source = lineage.catch_up_source
                       AND frozen.catch_up_watermark = lineage.catch_up_watermark
                   )
               )
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
                 'library_change_queue_catch_up_lineage'
               ))
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
                 'scan_run_catch_up_lineage'
               ))",
            [MAX_SCAN_CATCH_UP_LINEAGE],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_lineage {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its bounded queue-to-watermark lineage",
        ));
    }
    Ok(())
}

pub(super) fn validate_prerelease_authority(connection: &Connection) -> Result<(), ScanError> {
    let has_marker = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('library_change_queue_contract')
               WHERE name = 'change_catch_up_complete'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !has_marker {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its downtime catch-up checkpoint authority",
        ));
    }
    let marker_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !marker_complete {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its downtime catch-up checkpoint authority",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
