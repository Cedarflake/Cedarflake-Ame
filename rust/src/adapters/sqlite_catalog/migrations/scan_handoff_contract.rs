use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::current_schema::{ContractValidationDepth, record_current_schema_row_audit};
use super::database_error;
use super::schema_shape::{cascade_foreign_key_matches, named_index_matches, table_columns_match};

pub(super) fn validate_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    let marker_complete = connection
        .query_row(
            "SELECT scan_handoff_batch_complete = 1
             FROM library_change_queue_contract WHERE singleton = 1",
            [],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    let structure_matches = marker_complete
        && table_columns_match(
            connection,
            "library_change_scan_handoff_batches",
            &[
                ("id", "TEXT", true, 1),
                ("source_root_id", "TEXT", true, 0),
                ("updated_unix_ms", "INTEGER", true, 0),
            ],
        )?
        && table_columns_match(
            connection,
            "library_change_scan_handoff_lineage",
            &[
                ("batch_id", "TEXT", true, 1),
                ("catch_up_source", "TEXT", true, 2),
                ("catch_up_watermark", "TEXT", true, 3),
                ("enrolled_unix_ms", "INTEGER", true, 0),
            ],
        )?
        && table_columns_match(
            connection,
            "library_change_scan_handoff_items",
            &[
                ("batch_id", "TEXT", true, 1),
                ("file_identity_scheme", "TEXT", true, 2),
                ("file_identity_value", "TEXT", true, 3),
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
            ],
        )?
        && named_index_matches(
            connection,
            "library_change_scan_handoff_lineage",
            "library_change_scan_handoff_lineage_evidence",
            &["catch_up_source", "catch_up_watermark", "batch_id"],
        )?
        && named_index_matches(
            connection,
            "library_change_scan_handoff_items",
            "library_change_scan_handoff_items_identity",
            &["file_identity_scheme", "file_identity_value", "batch_id"],
        )?
        && named_index_matches(
            connection,
            "library_change_scan_handoff_items",
            "library_change_scan_handoff_items_asset",
            &["asset_id", "batch_id"],
        )?
        && named_index_matches(
            connection,
            "library_change_scan_handoff_items",
            "library_change_scan_handoff_items_preview",
            &["preview_path", "preview_status", "batch_id"],
        )?
        && cascade_foreign_key_matches(
            connection,
            "library_change_scan_handoff_lineage",
            "batch_id",
            "library_change_scan_handoff_batches",
            "id",
        )?
        && cascade_foreign_key_matches(
            connection,
            "library_change_scan_handoff_items",
            "batch_id",
            "library_change_scan_handoff_batches",
            "id",
        )?;
    if !structure_matches {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove its normalized scan handoff contract",
        ));
    }
    if !depth.includes_rows() {
        return Ok(());
    }
    record_current_schema_row_audit();

    let invalid_relations = connection
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
               SELECT 1 FROM library_change_scan_handoff_lineage AS handoffs
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
               SELECT 1 FROM library_change_scan_handoff_batches AS batches
               WHERE NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_lineage AS lineage
                 WHERE lineage.batch_id = batches.id
               ) OR NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS items
                 WHERE items.batch_id = batches.id
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
             ) OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
               'library_change_scan_handoff_lineage'
             )) OR EXISTS(SELECT 1 FROM pragma_foreign_key_check(
               'library_change_scan_handoff_items'
             ))",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog cannot prove the ownership of its retained handoff evidence",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
