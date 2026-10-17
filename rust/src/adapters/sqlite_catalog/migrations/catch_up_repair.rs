use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::super::MAX_SCAN_CATCH_UP_LINEAGE;
use super::{
    create_change_catch_up_handoff_contract, create_change_catch_up_lineage_contract,
    create_scan_handoff_batch_contract, create_scan_run_catch_up_lineage_contract, database_error,
    seed_change_catch_up_lineage,
};

pub(super) fn repair_derived_indexes(connection: &mut Connection) -> Result<(), ScanError> {
    let (
        has_table,
        has_marker,
        has_named_path_index,
        has_path_columns,
        has_handoff_table,
        has_handoff_columns,
        has_named_asset_index,
        has_named_preview_index,
        has_lineage_table,
        has_lineage_columns,
        has_named_lineage_index,
        has_obsolete_peer_index,
    ) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_state'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'index' AND name = 'asset_locations_root_relative'),
                (SELECT COUNT(*) FROM pragma_table_info('asset_locations')
                  WHERE name IN ('root_id', 'relative_path', 'scan_id', 'location_id')) = 4,
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'table' AND name = 'library_change_catch_up_handoffs'),
                (SELECT COUNT(*) FROM pragma_table_info('library_change_catch_up_handoffs')
                  WHERE name IN (
                    'catch_up_source', 'catch_up_watermark', 'file_identity_scheme',
                    'file_identity_value', 'asset_id', 'source_location_id', 'root_id',
                    'absolute_path', 'relative_path', 'preview_path', 'file_size',
                    'created_unix_ms', 'modified_unix_ms', 'width', 'height',
                    'preview_status', 'preview_issue_code', 'preview_issue_message',
                    'metadata_engine_id', 'metadata_engine_version', 'capture_local_time',
                    'capture_offset_minutes', 'capture_time_source', 'capture_raw_value',
                    'updated_unix_ms'
                  )) = 25,
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'index'
                    AND name = 'library_change_catch_up_handoffs_asset'),
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'index'
                    AND name = 'library_change_catch_up_handoffs_preview'),
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'table'
                    AND name = 'library_change_queue_catch_up_lineage'),
                (SELECT COUNT(*) FROM pragma_table_info(
                  'library_change_queue_catch_up_lineage'
                ) WHERE name IN (
                  'change_id', 'catch_up_source', 'catch_up_watermark', 'enrolled_unix_ms'
                )) = 4,
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'index'
                    AND name = 'library_change_queue_catch_up_lineage_evidence'),
                EXISTS(SELECT 1 FROM sqlite_master
                  WHERE type = 'index'
                    AND name = 'library_change_queue_catch_up_peer')",
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
                ))
            },
        )
        .map_err(database_error)?;
    if !has_table || !has_marker || !has_path_columns {
        return Ok(());
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
        return Ok(());
    }
    if (has_handoff_table && !has_handoff_columns) || (has_lineage_table && !has_lineage_columns) {
        return Ok(());
    }
    let has_unseeded_lineage = if has_lineage_table {
        connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_change_queue AS changes
                   WHERE changes.catch_up_source IS NOT NULL
                     AND changes.catch_up_watermark IS NOT NULL
                     AND NOT EXISTS (
                       SELECT 1 FROM library_change_queue_catch_up_lineage AS lineage
                       WHERE lineage.change_id = changes.id
                         AND lineage.catch_up_source = changes.catch_up_source
                         AND lineage.catch_up_watermark = changes.catch_up_watermark
                     )
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        false
    };
    let has_handoff_rows = if has_handoff_table {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM library_change_catch_up_handoffs)",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        false
    };
    if has_handoff_rows && (!has_lineage_table || has_unseeded_lineage) {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove the watermark lineage of its retained handoff evidence",
        ));
    }
    if has_named_path_index
        && has_handoff_table
        && has_named_asset_index
        && has_named_preview_index
        && has_lineage_table
        && has_named_lineage_index
        && !has_obsolete_peer_index
        && !has_unseeded_lineage
    {
        return Ok(());
    }

    let transaction = connection.transaction().map_err(database_error)?;
    if !has_named_path_index {
        transaction
            .execute(
                "CREATE INDEX asset_locations_root_relative
                 ON asset_locations(root_id, relative_path, scan_id, location_id)",
                [],
            )
            .map_err(database_error)?;
    }
    if !has_handoff_table {
        create_change_catch_up_handoff_contract(&transaction)?;
    } else if has_handoff_columns {
        if !has_named_asset_index {
            transaction
                .execute(
                    "CREATE INDEX library_change_catch_up_handoffs_asset
                     ON library_change_catch_up_handoffs(
                       asset_id, catch_up_source, catch_up_watermark
                     )",
                    [],
                )
                .map_err(database_error)?;
        }
        if !has_named_preview_index {
            transaction
                .execute(
                    "CREATE INDEX library_change_catch_up_handoffs_preview
                     ON library_change_catch_up_handoffs(
                       preview_path, preview_status, catch_up_source, catch_up_watermark
                     )",
                    [],
                )
                .map_err(database_error)?;
        }
    }
    if !has_lineage_table {
        create_change_catch_up_lineage_contract(&transaction)?;
    } else if has_lineage_columns && !has_named_lineage_index {
        transaction
            .execute(
                "CREATE INDEX library_change_queue_catch_up_lineage_evidence
                 ON library_change_queue_catch_up_lineage(
                   catch_up_source, catch_up_watermark, change_id
                 )",
                [],
            )
            .map_err(database_error)?;
    }
    if !has_lineage_table || has_lineage_columns {
        seed_change_catch_up_lineage(&transaction)?;
    }
    transaction
        .execute(
            "DROP INDEX IF EXISTS library_change_queue_catch_up_peer",
            [],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn repair_scan_lineage(connection: &mut Connection) -> Result<(), ScanError> {
    let (has_state_table, has_change_marker, has_scan_marker, has_scan_lineage_table) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'library_change_catch_up_state'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_catch_up_lineage_complete'),
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'scan_run_catch_up_lineage')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            },
        )
        .map_err(database_error)?;
    if !has_state_table || !has_change_marker || has_scan_marker {
        return Ok(());
    }
    let change_marker_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !change_marker_complete {
        return Ok(());
    }
    if has_scan_lineage_table {
        return Ok(());
    }
    let has_unprovable_active_scan = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM scan_runs AS scans
               WHERE scans.status IN ('running', 'paused')
                 AND scans.change_queue_high_watermark IS NOT NULL
                 AND NOT EXISTS (
                   SELECT 1 FROM library_change_queue AS changes
                   WHERE changes.root_id = scans.root_id
                     AND changes.root_generation = scans.root_generation_at_start
                     AND changes.id <= scans.change_queue_high_watermark
                     AND changes.status IN ('pending', 'leased', 'retry_wait')
                 )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if has_unprovable_active_scan {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot reconstruct the frozen lineage of an active scan",
        ));
    }
    let transaction = connection.transaction().map_err(database_error)?;
    create_scan_run_catch_up_lineage_contract(&transaction)?;
    transaction
        .execute(
            "INSERT INTO scan_run_catch_up_lineage(
               scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             )
             SELECT scans.id, lineage.catch_up_source, lineage.catch_up_watermark,
                    MAX(lineage.enrolled_unix_ms)
             FROM scan_runs AS scans
             JOIN library_change_queue AS changes
               ON changes.root_id = scans.root_id
              AND changes.root_generation = scans.root_generation_at_start
              AND changes.id <= scans.change_queue_high_watermark
             JOIN library_change_queue_catch_up_lineage AS lineage
               ON lineage.change_id = changes.id
             WHERE scans.status IN ('running', 'paused')
               AND changes.status IN ('pending', 'leased', 'retry_wait')
             GROUP BY scans.id, lineage.catch_up_source, lineage.catch_up_watermark",
            [],
        )
        .map_err(database_error)?;
    let oversized_scan = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM scan_run_catch_up_lineage
               GROUP BY scan_id HAVING COUNT(*) > ?1
             )",
            [MAX_SCAN_CATCH_UP_LINEAGE],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if oversized_scan {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot reconstruct a bounded scan lineage",
        ));
    }
    transaction
        .execute(
            "ALTER TABLE library_change_queue_contract
             ADD COLUMN scan_catch_up_lineage_complete INTEGER NOT NULL DEFAULT 1
             CHECK(scan_catch_up_lineage_complete = 1)",
            [],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn repair_scan_handoff_batches(connection: &mut Connection) -> Result<(), ScanError> {
    let (
        has_change_marker,
        has_scan_lineage_marker,
        has_scan_handoff_marker,
        has_batches,
        has_batch_lineage,
        has_batch_items,
    ) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'change_catch_up_complete'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_catch_up_lineage_complete'),
               EXISTS(SELECT 1 FROM pragma_table_info('library_change_queue_contract')
                 WHERE name = 'scan_handoff_batch_complete'),
               EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table'
                 AND name = 'library_change_scan_handoff_batches'),
               EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table'
                 AND name = 'library_change_scan_handoff_lineage'),
               EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table'
                 AND name = 'library_change_scan_handoff_items')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, bool>(5)?,
                ))
            },
        )
        .map_err(database_error)?;
    if !has_change_marker || !has_scan_lineage_marker || has_scan_handoff_marker {
        return Ok(());
    }
    if has_batches || has_batch_lineage || has_batch_items {
        return Ok(());
    }
    let prerequisite_complete = connection
        .query_row(
            "SELECT change_catch_up_complete = 1 AND scan_catch_up_lineage_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !prerequisite_complete {
        return Ok(());
    }
    let authority_is_unprovable = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
               WHERE NOT EXISTS (
                 SELECT 1
                 FROM library_change_queue_catch_up_lineage AS lineage
                 JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                 WHERE lineage.catch_up_source = handoffs.catch_up_source
                   AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                   AND changes.status IN ('pending', 'leased', 'retry_wait')
               ) AND NOT EXISTS (
                 SELECT 1
                 FROM scan_run_catch_up_lineage AS lineage
                 JOIN scan_runs AS scans ON scans.id = lineage.scan_id
                 WHERE lineage.catch_up_source = handoffs.catch_up_source
                   AND lineage.catch_up_watermark = handoffs.catch_up_watermark
                   AND scans.status IN ('running', 'paused')
               )
             ) OR EXISTS(
               SELECT 1
               FROM scan_run_catch_up_lineage AS frozen
               JOIN scan_runs AS scans ON scans.id = frozen.scan_id
               WHERE scans.status IN ('running', 'paused')
                 AND NOT EXISTS (
                   SELECT 1
                   FROM library_change_queue AS changes
                   JOIN library_change_queue_catch_up_lineage AS lineage
                     ON lineage.change_id = changes.id
                   WHERE changes.root_id = scans.root_id
                     AND changes.root_generation = scans.root_generation_at_start
                     AND changes.id <= scans.change_queue_high_watermark
                     AND lineage.catch_up_source = frozen.catch_up_source
                     AND lineage.catch_up_watermark = frozen.catch_up_watermark
                 )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if authority_is_unprovable {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove the ownership of its prerelease handoff evidence",
        ));
    }

    let transaction = connection.transaction().map_err(database_error)?;
    create_scan_handoff_batch_contract(&transaction)?;
    transaction
        .execute(
            "ALTER TABLE library_change_queue_contract
             ADD COLUMN scan_handoff_batch_complete INTEGER NOT NULL DEFAULT 1
             CHECK(scan_handoff_batch_complete = 1)",
            [],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

#[cfg(test)]
mod tests;
