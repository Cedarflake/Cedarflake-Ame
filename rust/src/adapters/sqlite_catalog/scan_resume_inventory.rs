use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{AssetLocationView, ScanError};

use super::scan_staging::{StagingRetirement, discard_scan_staging};
use super::{SqliteCatalog, database_error, read_stored_asset, stored_asset_view};

pub(super) fn retain_locations(
    transaction: &Transaction<'_>,
    scan_id: &str,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO scan_resume_pending_locations(scan_id, location_id)
             SELECT scan_id, location_id FROM asset_locations WHERE scan_id = ?1",
            [scan_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn accept_location(
    transaction: &Transaction<'_>,
    scan_id: &str,
    location_id: &str,
) -> Result<(), ScanError> {
    transaction
        .prepare_cached(
            "DELETE FROM scan_resume_pending_locations WHERE scan_id = ?1 AND location_id = ?2",
        )
        .map_err(database_error)?
        .execute(params![scan_id, location_id])
        .map_err(database_error)?;
    Ok(())
}

impl SqliteCatalog {
    pub(crate) fn load_retained_import_location(
        &self,
        scan_id: &str,
        location_id: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.connection
            .prepare_cached(
                "SELECT location.asset_id, location.location_id, location.root_id, location.scan_id,
                        location.absolute_path, location.relative_path, location.preview_path,
                        location.file_size, location.created_unix_ms, location.modified_unix_ms,
                        location.width, location.height, location.preview_status,
                        location.preview_issue_code, location.preview_issue_message,
                        location.metadata_engine_id, location.metadata_engine_version,
                        location.capture_local_time, location.capture_offset_minutes,
                        location.capture_time_source, location.capture_raw_value,
                        location.file_identity_scheme, location.file_identity_value,
                        location.source_revision_token, location.source_generation
                 FROM scan_resume_pending_locations AS pending
                 JOIN asset_locations AS location
                   ON location.scan_id = pending.scan_id AND location.location_id = pending.location_id
                 JOIN scan_runs AS scan ON scan.id = pending.scan_id
                 JOIN library_roots AS root ON root.id = scan.root_id
                 JOIN library_change_root_state AS state ON state.root_id = root.id
                 WHERE pending.scan_id = ?1 AND pending.location_id = ?2
                   AND scan.status = 'running' AND scan.scan_owner = 'foreground'
                   AND root.active_scan_id IS NULL AND state.is_active = 1
                   AND state.generation = scan.root_generation_at_start",
            )
            .map_err(database_error)?
            .query_row(params![scan_id, location_id], read_stored_asset)
            .optional()
            .map_err(database_error)?
            .map(stored_asset_view)
            .transpose()
    }

    pub(crate) fn finish_retained_import_inventory(
        &mut self,
        scan_id: &str,
    ) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let pending: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM scan_resume_pending_locations WHERE scan_id = ?1)",
                [scan_id],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !pending {
            return Ok(());
        }
        let transaction = self.begin_write()?;
        let admitted: bool = transaction
            .query_row(
                "SELECT EXISTS (
                SELECT 1 FROM scan_runs AS scan
                JOIN library_roots AS root ON root.id = scan.root_id
                JOIN library_change_root_state AS state ON state.root_id = root.id
                WHERE scan.id = ?1 AND scan.status = 'running' AND scan.scan_owner = 'foreground'
                  AND root.active_scan_id IS NULL AND state.is_active = 1
                  AND state.generation = scan.root_generation_at_start
            )",
                [scan_id],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !admitted {
            return Err(ScanError::new(
                "catalog_scan_resume_retired",
                "Only the current unfinished first import can retire unobserved retained locations",
            ));
        }
        discard_scan_staging(
            &transaction,
            scan_id,
            StagingRetirement::UnobservedResumeLocations,
        )?;
        transaction.commit().map_err(database_error)
    }
}
