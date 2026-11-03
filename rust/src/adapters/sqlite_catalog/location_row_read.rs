use rusqlite::Row;

use crate::domain::{AssetLocationView, CameraMetadata, ScanError};

use super::{
    parse_preview_status, sqlite_u32, sqlite_unsigned, stored_capture_time, stored_file_identity,
    stored_source_revision, user_visible_path,
};

pub(super) struct StoredAssetRow {
    asset_id: String,
    location_id: String,
    root_id: String,
    scan_id: String,
    absolute_path: String,
    relative_path: String,
    preview_path: String,
    file_size: i64,
    created_unix_ms: Option<i64>,
    modified_unix_ms: i64,
    width: i64,
    height: i64,
    preview_status: String,
    preview_issue_code: Option<String>,
    preview_issue_message: Option<String>,
    metadata_engine_id: String,
    metadata_engine_version: String,
    capture_local_time: Option<String>,
    capture_offset_minutes: Option<i64>,
    capture_time_source: Option<String>,
    capture_raw_value: Option<String>,
    file_identity_scheme: Option<String>,
    file_identity_value: Option<String>,
    source_revision_token: Option<String>,
    source_generation: Option<i64>,
    camera_make: Option<String>,
    camera_model: Option<String>,
}

pub(super) fn read_stored_asset(row: &Row<'_>) -> rusqlite::Result<StoredAssetRow> {
    Ok(StoredAssetRow {
        asset_id: row.get(0)?,
        location_id: row.get(1)?,
        root_id: row.get(2)?,
        scan_id: row.get(3)?,
        absolute_path: row.get(4)?,
        relative_path: row.get(5)?,
        preview_path: row.get(6)?,
        file_size: row.get(7)?,
        created_unix_ms: row.get(8)?,
        modified_unix_ms: row.get(9)?,
        width: row.get(10)?,
        height: row.get(11)?,
        preview_status: row.get(12)?,
        preview_issue_code: row.get(13)?,
        preview_issue_message: row.get(14)?,
        metadata_engine_id: row.get(15)?,
        metadata_engine_version: row.get(16)?,
        capture_local_time: row.get(17)?,
        capture_offset_minutes: row.get(18)?,
        capture_time_source: row.get(19)?,
        capture_raw_value: row.get(20)?,
        file_identity_scheme: row.get(21)?,
        file_identity_value: row.get(22)?,
        source_revision_token: row.get(23)?,
        source_generation: row.get(24)?,
        camera_make: row.get(25)?,
        camera_model: row.get(26)?,
    })
}

pub(super) fn stored_asset_view(stored: StoredAssetRow) -> Result<AssetLocationView, ScanError> {
    let camera = CameraMetadata {
        make: stored.camera_make,
        model: stored.camera_model,
    };
    camera.validate()?;
    let capture_time = stored_capture_time(
        stored.capture_local_time,
        stored.capture_offset_minutes,
        stored.capture_time_source,
        stored.capture_raw_value,
    )?;
    Ok(AssetLocationView {
        asset_id: stored.asset_id,
        location_id: stored.location_id,
        root_id: stored.root_id,
        scan_id: stored.scan_id,
        display_path: user_visible_path(&stored.absolute_path),
        absolute_path: stored.absolute_path,
        relative_path: stored.relative_path,
        preview_path: stored.preview_path,
        file_size: sqlite_unsigned(stored.file_size, "file size")?,
        created_unix_ms: stored.created_unix_ms,
        modified_unix_ms: stored.modified_unix_ms,
        file_identity: stored_file_identity(
            stored.file_identity_scheme,
            stored.file_identity_value,
        )?,
        source_revision: stored_source_revision(stored.source_revision_token)?,
        source_generation: sqlite_unsigned(
            stored.source_generation.ok_or_else(|| {
                ScanError::new(
                    "catalog_source_generation_missing",
                    "The catalog location has no source generation",
                )
            })?,
            "source generation",
        )?,
        width: sqlite_u32(stored.width, "image width")?,
        height: sqlite_u32(stored.height, "image height")?,
        preview_status: parse_preview_status(&stored.preview_status)?,
        preview_issue_code: stored.preview_issue_code,
        preview_issue_message: stored.preview_issue_message,
        metadata_engine_id: stored.metadata_engine_id,
        metadata_engine_version: stored.metadata_engine_version,
        camera,
        capture_time,
    })
}
