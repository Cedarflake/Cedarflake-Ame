use rusqlite::{Transaction, params};

use crate::domain::{AssetLocationView, ScanError};

use super::{
    capture_time_source_text, database_error, natural_name_key, parent_relative_path,
    preview_status_text, unix_time_ms,
};

pub(super) struct LocationRowWrite<'a> {
    pub(super) scan_id: &'a str,
    pub(super) root_id: &'a str,
    pub(super) location: &'a AssetLocationView,
    pub(super) file_size: i64,
    pub(super) source_revision_token: Option<&'a str>,
    pub(super) source_generation: i64,
}

impl LocationRowWrite<'_> {
    pub(super) fn persist(self, transaction: &Transaction<'_>) -> Result<(), ScanError> {
        let Self {
            scan_id,
            root_id,
            location,
            file_size,
            source_revision_token,
            source_generation,
        } = self;
        transaction
            .execute(
                "INSERT OR IGNORE INTO assets(id, created_unix_ms) VALUES (?1, ?2)",
                params![location.asset_id, unix_time_ms()],
            )
            .map_err(database_error)?;
        transaction
            .prepare_cached(
                "INSERT INTO asset_locations(
                   scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
                   preview_path, file_size, created_unix_ms, modified_unix_ms,
                   file_local_time, parent_relative_path, natural_name_key, width, height,
                   preview_status, preview_issue_code, preview_issue_message,
                   metadata_engine_id, metadata_engine_version, capture_local_time,
                   capture_offset_minutes, capture_time_source, capture_raw_value,
                   file_identity_scheme, file_identity_value, source_revision_token,
                   source_generation
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                    strftime(
                      '%Y-%m-%dT%H:%M:%f', COALESCE(?9, ?10) / 1000.0,
                      'unixepoch', 'localtime'
                    ),
                    ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                    ?21, ?22, ?23, ?24, ?25, ?26, ?27
                  )
                 ON CONFLICT(scan_id, location_id) DO UPDATE SET
                   asset_id = excluded.asset_id,
                   root_id = excluded.root_id,
                   absolute_path = excluded.absolute_path,
                   relative_path = excluded.relative_path,
                   preview_path = excluded.preview_path,
                   file_size = excluded.file_size,
                   created_unix_ms = excluded.created_unix_ms,
                   modified_unix_ms = excluded.modified_unix_ms,
                   file_local_time = excluded.file_local_time,
                   parent_relative_path = excluded.parent_relative_path,
                   natural_name_key = excluded.natural_name_key,
                   width = excluded.width,
                   height = excluded.height,
                   preview_status = excluded.preview_status,
                   preview_issue_code = excluded.preview_issue_code,
                   preview_issue_message = excluded.preview_issue_message,
                   metadata_engine_id = excluded.metadata_engine_id,
                   metadata_engine_version = excluded.metadata_engine_version,
                   capture_local_time = excluded.capture_local_time,
                   capture_offset_minutes = excluded.capture_offset_minutes,
                   capture_time_source = excluded.capture_time_source,
                   capture_raw_value = excluded.capture_raw_value,
                   file_identity_scheme = excluded.file_identity_scheme,
                   file_identity_value = excluded.file_identity_value,
                   source_revision_token = excluded.source_revision_token,
                   source_generation = excluded.source_generation",
            )
            .map_err(database_error)?
            .execute(params![
                scan_id,
                location.asset_id,
                location.location_id,
                root_id,
                location.absolute_path,
                location.relative_path,
                location.preview_path,
                file_size,
                location.created_unix_ms,
                location.modified_unix_ms,
                parent_relative_path(&location.relative_path),
                natural_name_key(&location.relative_path),
                i64::from(location.width),
                i64::from(location.height),
                preview_status_text(&location.preview_status),
                location.preview_issue_code,
                location.preview_issue_message,
                location.metadata_engine_id,
                location.metadata_engine_version,
                location
                    .capture_time
                    .as_ref()
                    .map(|evidence| &evidence.local_time),
                location
                    .capture_time
                    .as_ref()
                    .and_then(|evidence| evidence.offset_minutes)
                    .map(i64::from),
                location
                    .capture_time
                    .as_ref()
                    .map(|evidence| capture_time_source_text(&evidence.source)),
                location
                    .capture_time
                    .as_ref()
                    .map(|evidence| &evidence.raw_value),
                location
                    .file_identity
                    .as_ref()
                    .map(|identity| &identity.scheme),
                location
                    .file_identity
                    .as_ref()
                    .map(|identity| &identity.value),
                source_revision_token,
                source_generation,
            ])
            .map_err(database_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
