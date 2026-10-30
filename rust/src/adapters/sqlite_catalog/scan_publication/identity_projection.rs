use rusqlite::{Transaction, params};

use crate::domain::{FileIdentityEvidence, ScanError};

use super::super::{StoredIdentityGroupState, allocate_source_generation, database_error};

struct StoredIdentityObservationPayload {
    created_unix_ms: Option<i64>,
    width: i64,
    height: i64,
    metadata_engine_id: String,
    metadata_engine_version: String,
    capture_local_time: Option<String>,
    capture_offset_minutes: Option<i64>,
    capture_time_source: Option<String>,
    capture_raw_value: Option<String>,
    camera_make: Option<String>,
    camera_model: Option<String>,
}

struct StoredActiveIdentityProjectionPayload {
    created_unix_ms: Option<i64>,
    width: i64,
    height: i64,
    metadata_engine_id: String,
    metadata_engine_version: String,
    capture_local_time: Option<String>,
    capture_offset_minutes: Option<i64>,
    capture_time_source: Option<String>,
    capture_raw_value: Option<String>,
    camera_make: Option<String>,
    camera_model: Option<String>,
    preview_status: String,
    preview_issue_code: Option<String>,
    preview_issue_message: Option<String>,
}

pub(super) fn mirror_newer_active_identity(
    transaction: &Transaction<'_>,
    scan_id: &str,
    identity: &FileIdentityEvidence,
    active: &StoredIdentityGroupState,
) -> Result<(), ScanError> {
    let payload = load_active_identity_projection_payload(transaction, identity)?;
    let preview_status = if payload.preview_status == "failed" {
        "failed"
    } else {
        "pending"
    };
    let preview_issue_code = (preview_status == "failed")
        .then_some(payload.preview_issue_code.as_deref())
        .flatten();
    let preview_issue_message = (preview_status == "failed")
        .then_some(payload.preview_issue_message.as_deref())
        .flatten();
    transaction
        .execute(
            "UPDATE asset_locations
             SET file_size = ?4, created_unix_ms = ?5, modified_unix_ms = ?6,
                 file_local_time = strftime(
                   '%Y-%m-%dT%H:%M:%f', COALESCE(?5, ?6) / 1000.0,
                   'unixepoch', 'localtime'
                 ),
                 width = ?7, height = ?8,
                 metadata_engine_id = ?9, metadata_engine_version = ?10,
                 capture_local_time = ?11, capture_offset_minutes = ?12,
                 capture_time_source = ?13, capture_raw_value = ?14,
                 source_revision_token = ?15, source_generation = ?16,
                 preview_path = '', preview_status = ?17,
                 preview_issue_code = ?18, preview_issue_message = ?19,
                 camera_make = ?20, camera_model = ?21
             WHERE scan_id = ?3
               AND file_identity_scheme = ?1 AND file_identity_value = ?2",
            params![
                identity.scheme,
                identity.value,
                scan_id,
                active.file_size,
                payload.created_unix_ms,
                active.modified_unix_ms,
                payload.width,
                payload.height,
                payload.metadata_engine_id,
                payload.metadata_engine_version,
                payload.capture_local_time,
                payload.capture_offset_minutes,
                payload.capture_time_source,
                payload.capture_raw_value,
                active.source_revision_token,
                active.generation,
                preview_status,
                preview_issue_code,
                preview_issue_message,
                payload.camera_make,
                payload.camera_model,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn supersede_active_identity(
    transaction: &Transaction<'_>,
    scan_id: &str,
    identity: &FileIdentityEvidence,
    staged: &StoredIdentityGroupState,
) -> Result<(), ScanError> {
    let payload = load_staged_identity_payload(transaction, scan_id, identity)?;
    let generation = allocate_source_generation(transaction)?;
    transaction
        .execute(
            "DELETE FROM preview_artifact_locations
             WHERE location_id IN (
               SELECT locations.location_id
               FROM asset_locations AS locations
               WHERE locations.file_identity_scheme = ?1
                 AND locations.file_identity_value = ?2
                 AND (
                   locations.scan_id = ?3 OR EXISTS (
                     SELECT 1 FROM library_roots AS roots
                     WHERE roots.id = locations.root_id
                       AND roots.active_scan_id = locations.scan_id
                   )
                 )
             )",
            params![identity.scheme, identity.value, scan_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_terminal_media_evidence
             WHERE file_identity_scheme = ?1 AND file_identity_value = ?2",
            params![identity.scheme, identity.value],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE asset_locations
             SET file_size = ?4, created_unix_ms = ?5, modified_unix_ms = ?6,
                 file_local_time = strftime(
                   '%Y-%m-%dT%H:%M:%f', COALESCE(?5, ?6) / 1000.0,
                   'unixepoch', 'localtime'
                 ),
                 width = ?7, height = ?8,
                 metadata_engine_id = ?9, metadata_engine_version = ?10,
                 capture_local_time = ?11, capture_offset_minutes = ?12,
                 capture_time_source = ?13, capture_raw_value = ?14,
                 source_revision_token = ?15, source_generation = ?16,
                 preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL,
                 camera_make = ?17, camera_model = ?18
             WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
               AND (
                 scan_id = ?3 OR EXISTS (
                   SELECT 1 FROM library_roots AS roots
                   WHERE roots.id = asset_locations.root_id
                     AND roots.active_scan_id = asset_locations.scan_id
                 )
               )",
            params![
                identity.scheme,
                identity.value,
                scan_id,
                staged.file_size,
                payload.created_unix_ms,
                staged.modified_unix_ms,
                payload.width,
                payload.height,
                payload.metadata_engine_id,
                payload.metadata_engine_version,
                payload.capture_local_time,
                payload.capture_offset_minutes,
                payload.capture_time_source,
                payload.capture_raw_value,
                staged.source_revision_token,
                generation,
                payload.camera_make,
                payload.camera_model,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn load_active_identity_projection_payload(
    transaction: &Transaction<'_>,
    identity: &FileIdentityEvidence,
) -> Result<StoredActiveIdentityProjectionPayload, ScanError> {
    transaction
        .query_row(
            "SELECT locations.created_unix_ms, locations.width, locations.height,
                    locations.metadata_engine_id, locations.metadata_engine_version,
                    locations.capture_local_time, locations.capture_offset_minutes,
                    locations.capture_time_source, locations.capture_raw_value,
                    locations.preview_status, locations.preview_issue_code,
                    locations.preview_issue_message, locations.camera_make, locations.camera_model
             FROM asset_locations AS locations
             JOIN library_roots AS roots
               ON roots.id = locations.root_id
              AND roots.active_scan_id = locations.scan_id
             WHERE locations.file_identity_scheme = ?1
               AND locations.file_identity_value = ?2
             ORDER BY locations.root_id, locations.location_id
             LIMIT 1",
            params![identity.scheme, identity.value],
            |row| {
                Ok(StoredActiveIdentityProjectionPayload {
                    created_unix_ms: row.get(0)?,
                    width: row.get(1)?,
                    height: row.get(2)?,
                    metadata_engine_id: row.get(3)?,
                    metadata_engine_version: row.get(4)?,
                    capture_local_time: row.get(5)?,
                    capture_offset_minutes: row.get(6)?,
                    capture_time_source: row.get(7)?,
                    capture_raw_value: row.get(8)?,
                    preview_status: row.get(9)?,
                    preview_issue_code: row.get(10)?,
                    preview_issue_message: row.get(11)?,
                    camera_make: row.get(12)?,
                    camera_model: row.get(13)?,
                })
            },
        )
        .map_err(database_error)
}

fn load_staged_identity_payload(
    transaction: &Transaction<'_>,
    scan_id: &str,
    identity: &FileIdentityEvidence,
) -> Result<StoredIdentityObservationPayload, ScanError> {
    transaction
        .query_row(
            "SELECT created_unix_ms, width, height, metadata_engine_id,
                    metadata_engine_version, capture_local_time,
                    capture_offset_minutes, capture_time_source, capture_raw_value,
                    camera_make, camera_model
             FROM asset_locations
             WHERE scan_id = ?1 AND file_identity_scheme = ?2
               AND file_identity_value = ?3
             ORDER BY location_id LIMIT 1",
            params![scan_id, identity.scheme, identity.value],
            |row| {
                Ok(StoredIdentityObservationPayload {
                    created_unix_ms: row.get(0)?,
                    width: row.get(1)?,
                    height: row.get(2)?,
                    metadata_engine_id: row.get(3)?,
                    metadata_engine_version: row.get(4)?,
                    capture_local_time: row.get(5)?,
                    capture_offset_minutes: row.get(6)?,
                    capture_time_source: row.get(7)?,
                    capture_raw_value: row.get(8)?,
                    camera_make: row.get(9)?,
                    camera_model: row.get(10)?,
                })
            },
        )
        .map_err(database_error)
}
