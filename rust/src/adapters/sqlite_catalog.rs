use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, params_from_iter};

use crate::domain::{
    AssetLocationView, CaptureTimeEvidence, CaptureTimeSource, CatalogCursor, CatalogSnapshot,
    ExpectedFileState, FileIdentityEvidence, GalleryLayoutDateGroup, GalleryLayoutManifestChunk,
    GalleryLayoutManifestCursor, GalleryQuery, GallerySortKey, GalleryTimeAnchor,
    GalleryTimeBucket, GalleryTimeline, LibraryFolderCursor, LibraryFolderPage,
    LibraryRootAvailability, LibraryRootView, PreviewArtifact, PreviewReclamationCandidate,
    PreviewStatus, RecoverableScan, ScanCheckpoint, ScanError, ScanIssue, ScanRequest,
};
use crate::ports::CatalogRepository;

use super::user_visible_path;

mod folders;
mod gallery;
mod migrations;

use change_queue::{activate_root_change_queue, retire_root_change_queue};
use gallery::{
    GalleryAssetAnchor, build_gallery_asset_query, build_gallery_count_query,
    build_gallery_layout_manifest_query, build_gallery_timeline_query, gallery_cursor_for_asset,
    resolve_gallery_anchor_cursor, resolve_gallery_asset_anchor, resolve_gallery_location_anchor,
    validate_gallery_query,
};
use migrations::migrate_schema;

mod catalog_delta;
mod catch_up;
mod change_queue;
const SCHEMA_VERSION: i64 = 19;
const SCAN_QUEUE_LEASE_MILLIS: i64 = 15 * 60 * 1_000;
const LOCATION_STAGE_BATCH: usize = 128;
const MAX_LAYOUT_MANIFEST_CHUNK_ITEMS: u32 = 4_096;
const MAX_CATALOG_PAGE_ITEMS: u32 = 4_096;
const LAYOUT_FLAG_DIMENSIONS_KNOWN: u8 = 1;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub struct SqliteCatalog {
    path: PathBuf,
    connection: Connection,
    pending_locations: Vec<PendingLocation>,
}

#[derive(Clone)]
struct PendingLocation {
    scan_id: String,
    root_id: String,
    location: AssetLocationView,
}

struct StoredLayoutManifestItem {
    location_id: String,
    root_id: String,
    width: u32,
    height: u32,
    date_key: Option<String>,
    primary_missing: bool,
    primary_text: String,
    primary_number: i64,
}

impl SqliteCatalog {
    pub fn open(path: PathBuf) -> Result<Self, ScanError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                ScanError::new(
                    "catalog_directory_unavailable",
                    format!("Could not create the catalog directory: {error}"),
                )
            })?;
        }
        let mut connection = Connection::open(&path).map_err(database_error)?;
        connection
            .busy_timeout(SQLITE_BUSY_TIMEOUT)
            .map_err(database_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(database_error)?;
        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            connection
                .execute_batch("PRAGMA journal_mode = WAL;")
                .map_err(database_error)?;
        }
        migrate_schema(&mut connection)?;

        Ok(Self {
            path,
            connection,
            pending_locations: Vec::with_capacity(LOCATION_STAGE_BATCH),
        })
    }

    fn flush_pending_locations(&mut self) -> Result<(), ScanError> {
        if self.pending_locations.is_empty() {
            return Ok(());
        }
        let pending = self.pending_locations.clone();
        let transaction = self.connection.transaction().map_err(database_error)?;
        for item in &pending {
            persist_location(&transaction, &item.scan_id, &item.root_id, &item.location)?;
        }
        transaction.commit().map_err(database_error)?;
        self.pending_locations.clear();
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanOwner {
    Foreground,
    AuthoritativeRecovery,
}

impl ScanOwner {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Foreground => "foreground",
            Self::AuthoritativeRecovery => "authoritative_recovery",
        }
    }
}

impl SqliteCatalog {
    fn begin_scan_owned(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        owner: ScanOwner,
    ) -> Result<ScanCheckpoint, ScanError> {
        let now = unix_time_ms();
        let transaction = self.connection.transaction().map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET path = excluded.path",
                params![root_id, root_path, now],
            )
            .map_err(database_error)?;
        activate_root_change_queue(&transaction, root_id, now)?;
        let has_conflicting_scan = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE root_id = ?1 AND status IN ('running', 'paused') AND id <> ?2
                 )",
                params![root_id, request.scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if has_conflicting_scan {
            return Err(ScanError::new(
                "catalog_root_scan_in_progress",
                "Another authoritative scan already owns this library root",
            ));
        }
        let (root_generation_at_start, change_queue_high_watermark) = transaction
            .query_row(
                "SELECT state.generation,
                        MAX(CASE WHEN queue.status IN ('pending', 'leased', 'retry_wait')
                          THEN queue.id END)
                 FROM library_change_root_state AS state
                 LEFT JOIN library_change_queue AS queue
                   ON queue.root_id = state.root_id
                  AND queue.root_generation = state.generation
                 WHERE state.root_id = ?1 AND state.is_active = 1
                 GROUP BY state.generation",
                [root_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .map_err(database_error)?;
        let existing = transaction
            .query_row(
                "SELECT root_id, status, max_items, max_entries, preview_edge,
                        last_visited_relative_path, visited_entries, accepted_items, issue_count,
                        root_generation_at_start, change_queue_high_watermark,
                        requires_previous_snapshot, scan_owner
                 FROM scan_runs WHERE id = ?1",
                [&request.scan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, Option<i64>>(9)?,
                        row.get::<_, Option<i64>>(10)?,
                        row.get::<_, bool>(11)?,
                        row.get::<_, String>(12)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        let checkpoint = if let Some((
            stored_root_id,
            status,
            max_items,
            max_entries,
            preview_edge,
            last_visited_relative_path,
            visited_entries,
            accepted_items,
            issue_count,
            stored_root_generation,
            _stored_high_watermark,
            requires_previous_snapshot,
            stored_owner,
        )) = existing
        {
            let stored_max_items = optional_sqlite_u32(max_items, "item limit")?;
            let stored_max_entries = optional_sqlite_u32(max_entries, "entry limit")?;
            let stored_preview_edge = sqlite_u32(preview_edge, "preview edge")?;
            let is_paused = status == "paused";
            if status != "running" && !is_paused
                || stored_root_id != root_id
                || stored_max_items != request.max_items
                || stored_max_entries != request.max_entries
                || stored_preview_edge != request.preview_edge
                || stored_root_generation != Some(root_generation_at_start)
                || stored_owner != owner.as_str()
            {
                return Err(ScanError::new(
                    "catalog_scan_resume_mismatch",
                    "The stored scan cannot be resumed with different identity, ownership, or parameters",
                ));
            }
            if is_paused {
                transaction
                    .execute(
                        "UPDATE scan_runs SET status = 'running' WHERE id = ?1 AND status = 'paused'",
                        [&request.scan_id],
                    )
                    .map_err(database_error)?;
            }
            ScanCheckpoint {
                last_visited_relative_path,
                visited_entries: sqlite_unsigned(visited_entries, "visited entry count")?,
                accepted_items: sqlite_unsigned(accepted_items, "accepted item count")?,
                issue_count: sqlite_unsigned(issue_count, "issue count")?,
                requires_previous_snapshot,
            }
        } else {
            transaction
                .execute(
                    "INSERT INTO scan_runs(
                       id, root_id, status, started_unix_ms, max_items, max_entries, preview_edge,
                       root_generation_at_start, change_queue_high_watermark, scan_owner
                     ) VALUES (?1, ?2, 'running', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        request.scan_id,
                        root_id,
                        now,
                        request.max_items.map(i64::from),
                        request.max_entries.map(i64::from),
                        i64::from(request.preview_edge),
                        root_generation_at_start,
                        change_queue_high_watermark,
                        owner.as_str(),
                    ],
                )
                .map_err(database_error)?;
            if let Some(high_watermark) = change_queue_high_watermark {
                transaction
                    .execute(
                        "UPDATE library_change_queue
                         SET status = 'leased', next_retry_unix_ms = NULL,
                             lease_generation = lease_generation + 1,
                             lease_expires_unix_ms = ?1, updated_unix_ms = ?2,
                             authoritative_scan_id = ?3
                         WHERE root_id = ?4 AND root_generation = ?5 AND id <= ?6
                           AND status IN ('pending', 'retry_wait')",
                        params![
                            now.saturating_add(SCAN_QUEUE_LEASE_MILLIS),
                            now,
                            request.scan_id,
                            root_id,
                            root_generation_at_start,
                            high_watermark,
                        ],
                    )
                    .map_err(database_error)?;
            }
            transaction
                .execute(
                    "INSERT INTO scan_directory_frontier(scan_id, relative_path) VALUES (?1, '')",
                    [&request.scan_id],
                )
                .map_err(database_error)?;
            ScanCheckpoint::default()
        };
        transaction.commit().map_err(database_error)?;
        Ok(checkpoint)
    }

    pub(crate) fn has_active_scan_for_root(&self, root_id: &str) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE root_id = ?1 AND status IN ('running', 'paused')
                 )",
                [root_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }
}

impl CatalogRepository for SqliteCatalog {
    fn catalog_path(&self) -> &Path {
        &self.path
    }

    fn begin_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_scan_owned(request, root_id, root_path, ScanOwner::Foreground)
    }

    fn begin_authoritative_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_scan_owned(
            request,
            root_id,
            root_path,
            ScanOwner::AuthoritativeRecovery,
        )
    }

    fn has_active_locations(&self) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_roots AS roots
                   JOIN asset_locations AS locations
                     ON locations.scan_id = roots.active_scan_id
                   LIMIT 1
                 )",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    fn load_active_location_by_file_identity(
        &self,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.connection
            .query_row(
                "SELECT locations.asset_id, locations.location_id, locations.root_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size,
                        locations.created_unix_ms, locations.modified_unix_ms,
                        locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, locations.file_identity_scheme,
                        locations.file_identity_value
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 WHERE locations.file_identity_scheme = ?1
                   AND locations.file_identity_value = ?2
                 ORDER BY locations.location_id
                 LIMIT 1",
                params![identity.scheme, identity.value],
                read_stored_asset,
            )
            .optional()
            .map_err(database_error)?
            .map(stored_asset_view)
            .transpose()
    }

    fn load_active_location(
        &self,
        location_id: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.connection
            .query_row(
                "SELECT locations.asset_id, locations.location_id, locations.root_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size,
                        locations.created_unix_ms, locations.modified_unix_ms,
                        locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, locations.file_identity_scheme,
                        locations.file_identity_value
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 WHERE locations.location_id = ?1
                 LIMIT 1",
                [location_id],
                read_stored_asset,
            )
            .optional()
            .map_err(database_error)?
            .map(stored_asset_view)
            .transpose()
    }

    fn load_active_location_by_asset_id(
        &self,
        asset_id: &str,
        preferred_location_id: Option<&str>,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.connection
            .query_row(
                "SELECT locations.asset_id, locations.location_id, locations.root_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size,
                        locations.created_unix_ms, locations.modified_unix_ms,
                        locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, locations.file_identity_scheme,
                        locations.file_identity_value
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 WHERE locations.asset_id = ?1
                 ORDER BY CASE WHEN locations.location_id = ?2 THEN 0 ELSE 1 END,
                          locations.root_id, locations.location_id
                 LIMIT 1",
                params![asset_id, preferred_location_id.unwrap_or_default()],
                read_stored_asset,
            )
            .optional()
            .map_err(database_error)?
            .map(stored_asset_view)
            .transpose()
    }

    fn stage_location(
        &mut self,
        scan_id: &str,
        root_id: &str,
        location: &AssetLocationView,
    ) -> Result<(), ScanError> {
        if location.root_id != root_id {
            return Err(ScanError::new(
                "catalog_root_mismatch",
                "The staged location does not belong to the scan root",
            ));
        }
        self.pending_locations.push(PendingLocation {
            scan_id: scan_id.to_owned(),
            root_id: root_id.to_owned(),
            location: location.clone(),
        });
        if self.pending_locations.len() >= LOCATION_STAGE_BATCH {
            self.flush_pending_locations()?;
        }
        Ok(())
    }

    fn update_active_preview(
        &mut self,
        location: &AssetLocationView,
        artifact: Option<&PreviewArtifact>,
    ) -> Result<(), ScanError> {
        let file_size = sqlite_integer(location.file_size, "file size")?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        if let Some(artifact) = artifact {
            let artifact_bytes = sqlite_integer(artifact.byte_size, "preview artifact size")?;
            transaction
                .execute(
                    "DELETE FROM preview_artifact_locations
                     WHERE location_id = ?1
                       AND artifact_key IN (
                         SELECT artifact_key FROM preview_artifacts
                         WHERE algorithm_id = ?2
                           AND orientation_contract = ?3
                           AND size_bucket = ?4
                           AND artifact_key <> ?5
                       )",
                    params![
                        location.location_id,
                        artifact.algorithm_id,
                        artifact.orientation_contract,
                        i64::from(artifact.size_bucket),
                        artifact.artifact_key,
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE preview_artifacts
                     SET lifecycle_state = 'stale'
                     WHERE algorithm_id = ?1
                       AND orientation_contract = ?3
                       AND size_bucket = ?2
                       AND artifact_key <> ?4
                       AND lifecycle_state = 'ready'
                       AND NOT EXISTS (
                         SELECT 1 FROM preview_artifact_locations AS owners
                         WHERE owners.artifact_key = preview_artifacts.artifact_key
                       )",
                    params![
                        artifact.algorithm_id,
                        i64::from(artifact.size_bucket),
                        artifact.orientation_contract,
                        artifact.artifact_key,
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO preview_artifacts(
                       artifact_key, source_file_size, source_modified_unix_ms,
                       source_identity_scheme, source_identity_value, algorithm_id,
                       algorithm_version, orientation_contract, size_bucket, encoded_width,
                       encoded_height, artifact_path, byte_size, lifecycle_state,
                       created_unix_ms, last_used_unix_ms
                     ) VALUES (
                       ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                       'ready', ?14, ?14
                     )
                     ON CONFLICT(artifact_key) DO UPDATE SET
                       source_file_size = excluded.source_file_size,
                       source_modified_unix_ms = excluded.source_modified_unix_ms,
                       source_identity_scheme = excluded.source_identity_scheme,
                       source_identity_value = excluded.source_identity_value,
                       algorithm_id = excluded.algorithm_id,
                       algorithm_version = excluded.algorithm_version,
                       orientation_contract = excluded.orientation_contract,
                       size_bucket = excluded.size_bucket,
                       encoded_width = excluded.encoded_width,
                       encoded_height = excluded.encoded_height,
                       artifact_path = excluded.artifact_path,
                       byte_size = excluded.byte_size,
                       lifecycle_state = 'ready',
                       last_used_unix_ms = excluded.last_used_unix_ms",
                    params![
                        artifact.artifact_key,
                        file_size,
                        location.modified_unix_ms,
                        location
                            .file_identity
                            .as_ref()
                            .map(|identity| &identity.scheme),
                        location
                            .file_identity
                            .as_ref()
                            .map(|identity| &identity.value),
                        artifact.algorithm_id,
                        i64::from(artifact.algorithm_version),
                        artifact.orientation_contract,
                        i64::from(artifact.size_bucket),
                        i64::from(artifact.encoded_width),
                        i64::from(artifact.encoded_height),
                        artifact.path,
                        artifact_bytes,
                        unix_time_ms(),
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO preview_artifact_locations(
                       artifact_key, location_id
                     ) VALUES (?1, ?2)",
                    params![artifact.artifact_key, location.location_id],
                )
                .map_err(database_error)?;
        } else if !matches!(location.preview_status, PreviewStatus::Ready) {
            transaction
                .execute(
                    "DELETE FROM preview_artifact_locations WHERE location_id = ?1",
                    [&location.location_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE preview_artifacts
                     SET lifecycle_state = 'stale'
                     WHERE lifecycle_state = 'ready'
                       AND NOT EXISTS (
                         SELECT 1 FROM preview_artifact_locations AS owners
                         WHERE owners.artifact_key = preview_artifacts.artifact_key
                       )",
                    [],
                )
                .map_err(database_error)?;
        }
        let updated = transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = ?2, width = ?3, height = ?4,
                     preview_status = ?5, preview_issue_code = ?6,
                     preview_issue_message = ?7
                 WHERE location_id = ?1 AND file_size = ?8 AND modified_unix_ms = ?9
                   AND root_id = ?10 AND absolute_path = ?11
                   AND file_identity_scheme IS ?12 AND file_identity_value IS ?13
                   AND scan_id = (
                     SELECT active_scan_id FROM library_roots WHERE id = ?10
                   )",
                params![
                    location.location_id,
                    location.preview_path,
                    i64::from(location.width),
                    i64::from(location.height),
                    preview_status_text(&location.preview_status),
                    location.preview_issue_code,
                    location.preview_issue_message,
                    file_size,
                    location.modified_unix_ms,
                    location.root_id,
                    location.absolute_path,
                    location
                        .file_identity
                        .as_ref()
                        .map(|identity| &identity.scheme),
                    location
                        .file_identity
                        .as_ref()
                        .map(|identity| &identity.value),
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "active_preview_location_stale",
                "The active catalog location changed before its preview was updated",
            ));
        }
        transaction.commit().map_err(database_error)
    }

    fn reset_all_previews_for_cleanup(&mut self) -> Result<u64, ScanError> {
        self.flush_pending_locations()?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let updated = transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path <> '' OR preview_status <> 'pending'
                   OR preview_issue_code IS NOT NULL OR preview_issue_message IS NOT NULL",
                [],
            )
            .map_err(database_error)?;
        transaction
            .execute("DELETE FROM preview_artifacts", [])
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        u64::try_from(updated).map_err(|_| {
            ScanError::new(
                "preview_cleanup_count_invalid",
                "The preview cleanup update count exceeds the supported range",
            )
        })
    }

    fn reset_previews_outside_root(&mut self, preview_root_prefix: &str) -> Result<u64, ScanError> {
        if preview_root_prefix.is_empty() {
            return Err(ScanError::new(
                "preview_root_prefix_empty",
                "The active preview root prefix is required",
            ));
        }
        self.flush_pending_locations()?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let updated = transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path <> ''
                   AND lower(substr(preview_path, 1, length(?1))) <> lower(?1)",
                [preview_root_prefix],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM preview_artifacts
                 WHERE lower(substr(artifact_path, 1, length(?1))) <> lower(?1)",
                [preview_root_prefix],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        u64::try_from(updated).map_err(|_| {
            ScanError::new(
                "preview_root_reset_count_invalid",
                "The preview-root reset count exceeds the supported range",
            )
        })
    }

    fn is_preview_artifact_path_indexed(
        &self,
        path: &str,
        artifact_key: Option<&str>,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM preview_artifacts
                   WHERE lower(artifact_path) = lower(?1)
                      OR (?2 IS NOT NULL AND artifact_key = ?2)
                 )",
                params![path, artifact_key],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    fn load_preview_recovery_artifacts(
        &self,
        preview_root_prefix: &str,
        after_artifact_key: Option<&str>,
        limit: u32,
    ) -> Result<Vec<PreviewReclamationCandidate>, ScanError> {
        if preview_root_prefix.is_empty() || limit == 0 || limit > 4_096 {
            return Err(ScanError::new(
                "preview_recovery_query_invalid",
                "Preview recovery requires a root prefix and a limit between 1 and 4096",
            ));
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT artifact_key, artifact_path
                 FROM preview_artifacts
                 WHERE lower(substr(artifact_path, 1, length(?1))) = lower(?1)
                   AND (?2 IS NULL OR artifact_key > ?2)
                 ORDER BY artifact_key
                 LIMIT ?3",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![preview_root_prefix, after_artifact_key, i64::from(limit)],
                |row| {
                    Ok(PreviewReclamationCandidate {
                        artifact_key: row.get(0)?,
                        path: row.get(1)?,
                    })
                },
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok(rows)
    }

    fn reconcile_preview_artifact_bytes(
        &mut self,
        candidate: &PreviewReclamationCandidate,
        actual_bytes: u64,
    ) -> Result<bool, ScanError> {
        let actual_bytes = i64::try_from(actual_bytes).map_err(|_| {
            ScanError::new(
                "preview_artifact_size_invalid",
                "The preview artifact size exceeds the catalog range",
            )
        })?;
        let updated = self
            .connection
            .execute(
                "UPDATE preview_artifacts
                 SET byte_size = ?3
                 WHERE artifact_key = ?1 AND artifact_path = ?2
                   AND byte_size <> ?3",
                params![candidate.artifact_key, candidate.path, actual_bytes,],
            )
            .map_err(database_error)?;
        Ok(updated != 0)
    }

    fn touch_preview_artifacts(
        &mut self,
        artifacts: &[(String, String)],
    ) -> Result<u64, ScanError> {
        if artifacts.len() > 4_096 {
            return Err(ScanError::new(
                "preview_touch_batch_too_large",
                "Preview usage updates are limited to 4096 artifacts per batch",
            ));
        }
        if artifacts.is_empty() {
            return Ok(0);
        }
        let now = unix_time_ms();
        let oldest_retained = now.saturating_sub(60_000);
        let transaction = self.connection.transaction().map_err(database_error)?;
        let mut statement = transaction
            .prepare_cached(
                "UPDATE preview_artifacts
                 SET last_used_unix_ms = ?3
                 WHERE artifact_key IN (
                     SELECT artifact_key FROM preview_artifact_locations
                     WHERE location_id = ?1
                 )
                   AND artifact_path = ?2 AND last_used_unix_ms < ?4",
            )
            .map_err(database_error)?;
        let mut updated = 0_usize;
        for (location_id, artifact_path) in artifacts {
            updated = updated.saturating_add(
                statement
                    .execute(params![location_id, artifact_path, now, oldest_retained])
                    .map_err(database_error)?,
            );
        }
        drop(statement);
        transaction.commit().map_err(database_error)?;
        u64::try_from(updated).map_err(|_| {
            ScanError::new(
                "preview_touch_count_invalid",
                "The preview usage update count exceeds the supported range",
            )
        })
    }

    fn load_preview_reclamation_candidates(
        &self,
        protected_location_ids: &[String],
        current_algorithm_id: &str,
        current_algorithm_version: u32,
        current_orientation_contract: &str,
        current_preview_root_prefix: &str,
        limit: u32,
    ) -> Result<Vec<PreviewReclamationCandidate>, ScanError> {
        if limit == 0 || limit > 4_096 {
            return Err(ScanError::new(
                "preview_reclamation_limit_invalid",
                "Preview reclamation candidate limits must be between 1 and 4096",
            ));
        }
        let protected = protected_location_ids
            .iter()
            .filter(|location_id| !location_id.is_empty())
            .collect::<HashSet<_>>();
        let mut values = vec![
            Value::Text(current_algorithm_id.to_owned()),
            Value::Integer(i64::from(current_algorithm_version)),
            Value::Text(current_orientation_contract.to_owned()),
            Value::Text(current_preview_root_prefix.to_owned()),
        ];
        let protected_clause = if protected.is_empty() {
            String::new()
        } else {
            let placeholders = protected
                .iter()
                .map(|location_id| {
                    values.push(Value::Text((*location_id).clone()));
                    format!("?{}", values.len())
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "AND artifact_key NOT IN (
                   SELECT artifact_key FROM preview_artifact_locations
                   WHERE location_id IN ({placeholders})
                 )"
            )
        };
        values.push(Value::Integer(i64::from(limit)));
        let limit_parameter = values.len();
        let query = format!(
            "SELECT artifact_key, artifact_path
             FROM preview_artifacts
             WHERE lower(substr(artifact_path, 1, length(?4))) = lower(?4)
             {protected_clause}
             ORDER BY
               CASE
                 WHEN lifecycle_state <> 'ready' THEN 0
                 WHEN algorithm_id <> ?1 OR algorithm_version <> ?2
                   OR orientation_contract <> ?3 THEN 1
                 ELSE 2
               END,
               last_used_unix_ms, artifact_key
             LIMIT ?{limit_parameter}"
        );
        let mut statement = self.connection.prepare(&query).map_err(database_error)?;
        let rows = statement
            .query_map(params_from_iter(values), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        let mut candidates = Vec::new();
        for row in rows {
            let (artifact_key, path) = row.map_err(database_error)?;
            candidates.push(PreviewReclamationCandidate { artifact_key, path });
        }
        Ok(candidates)
    }

    fn remove_reclaimed_preview(
        &mut self,
        candidate: &PreviewReclamationCandidate,
    ) -> Result<bool, ScanError> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path = ?1
                   AND location_id IN (
                     SELECT location_id FROM preview_artifact_locations
                     WHERE artifact_key = ?2
                   )",
                params![candidate.path, candidate.artifact_key],
            )
            .map_err(database_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM preview_artifacts
                 WHERE artifact_key = ?1 AND artifact_path = ?2",
                params![candidate.artifact_key, candidate.path],
            )
            .map_err(database_error)?;
        if deleted != 1 {
            return Ok(false);
        }
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }

    fn record_issue(&mut self, scan_id: &str, issue: &ScanIssue) -> Result<(), ScanError> {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO scan_issues(scan_id, path, code, message)
                 VALUES (?1, ?2, ?3, ?4)",
                params![scan_id, issue.path, issue.code, issue.message],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn checkpoint_scan(
        &mut self,
        scan_id: &str,
        checkpoint: &ScanCheckpoint,
    ) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let visited_entries = sqlite_integer(checkpoint.visited_entries, "visited entry count")?;
        let accepted_items = sqlite_integer(checkpoint.accepted_items, "accepted item count")?;
        let issue_count = sqlite_integer(checkpoint.issue_count, "issue count")?;
        let updated = self
            .connection
            .execute(
                "UPDATE scan_runs
                 SET last_visited_relative_path = ?2, visited_entries = ?3,
                     accepted_items = ?4, issue_count = ?5,
                     requires_previous_snapshot = ?6
                 WHERE id = ?1 AND status = 'running'",
                params![
                    scan_id,
                    checkpoint.last_visited_relative_path,
                    visited_entries,
                    accepted_items,
                    issue_count,
                    checkpoint.requires_previous_snapshot,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_not_checkpointable",
                "The scan is no longer in a checkpointable running state",
            ));
        }
        Ok(())
    }

    fn load_recoverable_scan(&self) -> Result<Option<RecoverableScan>, ScanError> {
        load_scan_with_status(&self.connection, "running", ScanOwner::Foreground)
    }

    fn load_paused_scan(&self) -> Result<Option<RecoverableScan>, ScanError> {
        load_scan_with_status(&self.connection, "paused", ScanOwner::Foreground)
    }

    fn load_authoritative_recoverable_scan_after(
        &self,
        after_scan_id: Option<&str>,
    ) -> Result<Option<RecoverableScan>, ScanError> {
        load_scans_with_status(
            &self.connection,
            "running",
            ScanOwner::AuthoritativeRecovery,
            after_scan_id,
            1,
        )
        .map(|mut scans| scans.pop())
    }

    fn claim_next_directory(&mut self, scan_id: &str) -> Result<Option<String>, ScanError> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        let current = transaction
            .query_row(
                "SELECT current_directory_relative_path
                 FROM scan_runs WHERE id = ?1 AND status = 'running'",
                [scan_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_not_traversable",
                    "The scan is no longer in a traversable running state",
                )
            })?;
        if current.is_some() {
            transaction.commit().map_err(database_error)?;
            return Ok(current);
        }
        let next = transaction
            .query_row(
                "SELECT id, relative_path FROM scan_directory_frontier
                 WHERE scan_id = ?1 ORDER BY id LIMIT 1",
                [scan_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(database_error)?;
        let Some((frontier_id, relative_path)) = next else {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        };
        transaction
            .execute(
                "DELETE FROM scan_directory_frontier WHERE id = ?1 AND scan_id = ?2",
                params![frontier_id, scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE scan_runs
                 SET current_directory_relative_path = ?2,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE id = ?1 AND status = 'running'",
                params![scan_id, relative_path],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(Some(relative_path))
    }

    fn is_current_directory_enumerated(
        &self,
        scan_id: &str,
        relative_path: &str,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT current_directory_enumerated
                 FROM scan_runs
                 WHERE id = ?1 AND status = 'running'
                   AND current_directory_relative_path = ?2",
                params![scan_id, relative_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_directory_mismatch",
                    "The requested directory is not the current running directory",
                )
            })
    }

    fn stage_directory_entries(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
        relative_paths: &[String],
    ) -> Result<(), ScanError> {
        if relative_paths.is_empty() {
            return Ok(());
        }
        let transaction = self.connection.transaction().map_err(database_error)?;
        let is_enumerating: bool = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE id = ?1 AND status = 'running'
                     AND current_directory_relative_path = ?2
                     AND current_directory_enumerated = 0
                 )",
                params![scan_id, relative_directory],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !is_enumerating {
            return Err(ScanError::new(
                "catalog_scan_directory_not_enumerable",
                "Directory entries cannot be staged outside the active enumeration",
            ));
        }
        {
            let mut statement = transaction
                .prepare_cached(
                    "INSERT OR IGNORE INTO scan_directory_entries(
                       scan_id, directory_relative_path, relative_path
                     ) VALUES (?1, ?2, ?3)",
                )
                .map_err(database_error)?;
            for relative_path in relative_paths {
                statement
                    .execute(params![scan_id, relative_directory, relative_path])
                    .map_err(database_error)?;
            }
        }
        transaction.commit().map_err(database_error)
    }

    fn complete_directory_enumeration(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
    ) -> Result<(), ScanError> {
        let updated = self
            .connection
            .execute(
                "UPDATE scan_runs SET current_directory_enumerated = 1
                 WHERE id = ?1 AND status = 'running'
                   AND current_directory_relative_path = ?2
                   AND current_directory_enumerated = 0",
                params![scan_id, relative_directory],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_directory_not_enumerable",
                "The current directory enumeration could not be completed",
            ));
        }
        Ok(())
    }

    fn has_directory_entry(
        &self,
        scan_id: &str,
        relative_directory: &str,
        relative_path: &str,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_directory_entries
                   WHERE scan_id = ?1 AND directory_relative_path = ?2
                     AND relative_path = ?3
                 )",
                params![scan_id, relative_directory, relative_path],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    fn load_directory_entry_window(
        &self,
        scan_id: &str,
        relative_directory: &str,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, ScanError> {
        if limit == 0 {
            return Err(ScanError::new(
                "directory_entry_window_invalid",
                "The directory entry window must contain at least one item",
            ));
        }
        let is_ready: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE id = ?1 AND status = 'running'
                     AND current_directory_relative_path = ?2
                     AND current_directory_enumerated = 1
                 )",
                params![scan_id, relative_directory],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !is_ready {
            return Err(ScanError::new(
                "catalog_scan_directory_not_ready",
                "Directory entries cannot be loaded before enumeration completes",
            ));
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT relative_path FROM scan_directory_entries
                 WHERE scan_id = ?1 AND directory_relative_path = ?2
                   AND (?3 IS NULL OR relative_path > ?3)
                 ORDER BY relative_path
                 LIMIT ?4",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![scan_id, relative_directory, after, i64::from(limit)],
                |row| row.get::<_, String>(0),
            )
            .map_err(database_error)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(database_error)?);
        }
        Ok(entries)
    }

    fn enqueue_directory(&mut self, scan_id: &str, relative_path: &str) -> Result<(), ScanError> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        let is_running: bool = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs WHERE id = ?1 AND status = 'running'
                 )",
                [scan_id],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !is_running {
            return Err(ScanError::new(
                "catalog_scan_not_traversable",
                "The scan is no longer in a traversable running state",
            ));
        }
        transaction
            .execute(
                "INSERT OR IGNORE INTO scan_directory_frontier(scan_id, relative_path)
                 VALUES (?1, ?2)",
                params![scan_id, relative_path],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    fn complete_directory(
        &mut self,
        scan_id: &str,
        checkpoint: &ScanCheckpoint,
    ) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let visited_entries = sqlite_integer(checkpoint.visited_entries, "visited entry count")?;
        let accepted_items = sqlite_integer(checkpoint.accepted_items, "accepted item count")?;
        let issue_count = sqlite_integer(checkpoint.issue_count, "issue count")?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let current_directory = transaction
            .query_row(
                "SELECT current_directory_relative_path FROM scan_runs
                 WHERE id = ?1 AND status = 'running'",
                [scan_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(database_error)?
            .flatten()
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_directory_not_completable",
                    "The current scan directory is missing",
                )
            })?;
        transaction
            .execute(
                "DELETE FROM scan_directory_entries
                 WHERE scan_id = ?1 AND directory_relative_path = ?2",
                params![scan_id, current_directory],
            )
            .map_err(database_error)?;
        let updated = transaction
            .execute(
                "UPDATE scan_runs
                 SET current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL,
                     visited_entries = ?2, accepted_items = ?3, issue_count = ?4,
                     requires_previous_snapshot = ?5
                 WHERE id = ?1 AND status = 'running'",
                params![
                    scan_id,
                    visited_entries,
                    accepted_items,
                    issue_count,
                    checkpoint.requires_previous_snapshot,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_directory_not_completable",
                "The current scan directory could not be completed",
            ));
        }
        transaction.commit().map_err(database_error)
    }

    fn pause_scan(&mut self, scan_id: &str, checkpoint: &ScanCheckpoint) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let visited_entries = sqlite_integer(checkpoint.visited_entries, "visited entry count")?;
        let accepted_items = sqlite_integer(checkpoint.accepted_items, "accepted item count")?;
        let issue_count = sqlite_integer(checkpoint.issue_count, "issue count")?;
        let updated = self
            .connection
            .execute(
                "UPDATE scan_runs
                 SET status = 'paused', last_visited_relative_path = ?2,
                     visited_entries = ?3, accepted_items = ?4, issue_count = ?5,
                     requires_previous_snapshot = ?6
                 WHERE id = ?1 AND status = 'running'",
                params![
                    scan_id,
                    checkpoint.last_visited_relative_path,
                    visited_entries,
                    accepted_items,
                    issue_count,
                    checkpoint.requires_previous_snapshot,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_not_pausable",
                "The scan is no longer in a pausable running state",
            ));
        }
        Ok(())
    }

    fn count_staged_file_states(&mut self, scan_id: &str) -> Result<u64, ScanError> {
        self.flush_pending_locations()?;
        let count = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1",
                [scan_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        sqlite_unsigned(count, "staged file state count")
    }

    fn load_staged_file_state_window(
        &self,
        scan_id: &str,
        after_location_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, ExpectedFileState)>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT location_id, absolute_path, file_size, modified_unix_ms,
                        file_identity_scheme, file_identity_value
                 FROM asset_locations
                 WHERE scan_id = ?1 AND (?2 IS NULL OR location_id > ?2)
                 ORDER BY location_id LIMIT ?3",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![scan_id, after_location_id, i64::from(limit)],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .map_err(database_error)?;
        let mut states = Vec::new();
        for row in rows {
            let (
                location_id,
                absolute_path,
                file_size,
                modified_unix_ms,
                identity_scheme,
                identity_value,
            ) = row.map_err(database_error)?;
            let file_size = u64::try_from(file_size).map_err(|_| {
                ScanError::new(
                    "catalog_integer_invalid",
                    "A staged file size is outside the supported range",
                )
            })?;
            states.push((
                location_id,
                ExpectedFileState {
                    absolute_path,
                    file_size,
                    modified_unix_ms,
                    file_identity: stored_file_identity(identity_scheme, identity_value)?,
                },
            ));
        }
        Ok(states)
    }

    fn publish_scan(
        &mut self,
        scan_id: &str,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
    ) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let asset_count = sqlite_integer(asset_count, "asset count")?;
        let issue_count = sqlite_integer(issue_count, "issue count")?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let (
            previous_active_scan,
            root_generation_at_start,
            change_queue_high_watermark,
            requires_previous_snapshot,
        ) = transaction
            .query_row(
                "SELECT roots.active_scan_id, scans.root_generation_at_start,
                        scans.change_queue_high_watermark,
                        scans.requires_previous_snapshot
                 FROM library_roots AS roots
                 JOIN scan_runs AS scans ON scans.id = ?1 AND scans.root_id = roots.id
                 WHERE roots.id = ?2",
                params![scan_id, root_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                },
            )
            .map_err(database_error)?;
        if requires_previous_snapshot {
            return Err(ScanError::new(
                "catalog_scan_requires_previous_snapshot",
                "The scan encountered evidence that requires retaining the previous catalog snapshot",
            ));
        }
        let root_generation_at_start = root_generation_at_start.ok_or_else(|| {
            ScanError::new(
                "catalog_scan_generation_unverifiable",
                "The scan cannot prove the root generation captured at start",
            )
        })?;
        let root_generation_is_current = transaction
            .query_row(
                "SELECT generation = ?2 AND is_active = 1
                 FROM library_change_root_state WHERE root_id = ?1",
                params![root_id, root_generation_at_start],
                |row| row.get::<_, bool>(0),
            )
            .optional()
            .map_err(database_error)?
            .unwrap_or(false);
        if !root_generation_is_current {
            return Err(ScanError::new(
                "catalog_scan_root_generation_changed",
                "The library root changed while the authoritative scan was running",
            ));
        }
        let updated = transaction
            .execute(
                "UPDATE scan_runs
                 SET status = 'completed', completed_unix_ms = ?2,
                     asset_count = ?3, issue_count = ?4,
                     current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE id = ?1 AND status = 'running'",
                params![scan_id, unix_time_ms(), asset_count, issue_count],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_not_publishable",
                "The scan is no longer in a publishable running state",
            ));
        }
        transaction
            .execute(
                "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_roots SET active_scan_id = ?2 WHERE id = ?1",
                params![root_id, scan_id],
            )
            .map_err(database_error)?;
        detach_preview_references_for_root_locations(&transaction, root_id, Some(scan_id))?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO preview_artifact_locations(artifact_key, location_id)
                 SELECT artifacts.artifact_key, locations.location_id
                 FROM asset_locations AS locations
                 JOIN preview_artifacts AS artifacts
                   ON artifacts.artifact_path = locations.preview_path
                 WHERE locations.root_id = ?1 AND locations.scan_id = ?2
                   AND locations.preview_status = 'ready'",
                params![root_id, scan_id],
            )
            .map_err(database_error)?;
        mark_unreferenced_preview_artifacts_stale(&transaction)?;
        if let Some(previous_active_scan) = previous_active_scan
            && previous_active_scan != scan_id
        {
            transaction
                .execute(
                    "DELETE FROM asset_locations WHERE scan_id = ?1",
                    [previous_active_scan],
                )
                .map_err(database_error)?;
        }
        delete_orphan_assets(&transaction)?;
        let revision_updated = transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        if revision_updated != 1 {
            return Err(ScanError::new(
                "catalog_revision_unavailable",
                "The catalog revision state is missing or invalid",
            ));
        }
        let published_revision = load_catalog_revision(&transaction)?;
        let completed_unix_ms = unix_time_ms();
        if let Some(high_watermark) = change_queue_high_watermark {
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'completed', next_retry_unix_ms = NULL,
                         lease_expires_unix_ms = NULL,
                         catalog_revision_at_success = ?1, updated_unix_ms = ?2,
                         authoritative_scan_id = NULL
                     WHERE root_id = ?3 AND root_generation = ?4 AND id <= ?5
                       AND status IN ('pending', 'leased', 'retry_wait')",
                    params![
                        sqlite_integer(published_revision, "catalog revision")?,
                        completed_unix_ms,
                        root_id,
                        root_generation_at_start,
                        high_watermark,
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET authoritative_scan_id = NULL
                     WHERE authoritative_scan_id = ?1",
                    [scan_id],
                )
                .map_err(database_error)?;
        }
        transaction
            .execute(
                "UPDATE library_change_root_state
                 SET last_consistency_audit_unix_ms = ?2, updated_unix_ms = ?2
                 WHERE root_id = ?1 AND generation = ?3 AND is_active = 1",
                params![root_id, completed_unix_ms, root_generation_at_start],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    fn abandon_scan(
        &mut self,
        scan_id: &str,
        status: &str,
        issue_count: u64,
    ) -> Result<(), ScanError> {
        self.pending_locations
            .retain(|pending| pending.scan_id != scan_id);
        let issue_count = sqlite_integer(issue_count, "issue count")?;
        let now = unix_time_ms();
        let transaction = self.connection.transaction().map_err(database_error)?;
        let abandoned = transaction
            .execute(
                "UPDATE scan_runs
                 SET status = ?2, completed_unix_ms = ?3, issue_count = ?4,
                     current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE id = ?1 AND status = 'running'",
                params![scan_id, status, now, issue_count],
            )
            .map_err(database_error)?;
        if abandoned == 1 {
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'pending', ready_unix_ms = ?2,
                         next_retry_unix_ms = NULL, lease_expires_unix_ms = NULL,
                         authoritative_scan_id = NULL, updated_unix_ms = ?2
                     WHERE authoritative_scan_id = ?1 AND status = 'leased'",
                    params![scan_id, now],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET authoritative_scan_id = NULL
                     WHERE authoritative_scan_id = ?1",
                    [scan_id],
                )
                .map_err(database_error)?;
        }
        transaction
            .execute(
                "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute("DELETE FROM asset_locations WHERE scan_id = ?1", [scan_id])
            .map_err(database_error)?;
        delete_orphan_assets(&transaction)?;
        transaction.commit().map_err(database_error)
    }

    fn load_snapshot(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&CatalogCursor>,
        before: Option<&CatalogCursor>,
        anchor: Option<&GalleryTimeAnchor>,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                format!(
                    "The catalog page limit must be between 1 and {MAX_CATALOG_PAGE_ITEMS} items"
                ),
            ));
        }
        validate_gallery_query(query)?;
        if usize::from(after.is_some())
            + usize::from(before.is_some())
            + usize::from(anchor.is_some())
            > 1
        {
            return Err(ScanError::new(
                "catalog_query_invalid",
                "A gallery request accepts only one page cursor or anchor",
            ));
        }

        let catalog_path = self.path.to_string_lossy().into_owned();
        let transaction = self.connection.transaction().map_err(database_error)?;
        let revision = load_catalog_revision(&transaction)?;
        if after.is_some_and(|cursor| cursor.revision != revision || cursor.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this page cursor was created",
            ));
        }
        if before.is_some_and(|cursor| cursor.revision != revision || cursor.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this page cursor was created",
            ));
        }
        if anchor.is_some_and(|value| value.revision != revision || value.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this time anchor was created",
            ));
        }

        let roots = load_root_views(&transaction)?;

        let requested = usize::try_from(max_items).map_err(|_| {
            ScanError::new(
                "catalog_page_limit_invalid",
                "The catalog page limit is outside the supported range",
            )
        })?;
        let sql_limit = i64::from(max_items).saturating_add(1);
        let resolved_anchor_cursor = anchor
            .filter(|value| value.item_offset > 0)
            .map(|value| {
                resolve_gallery_anchor_cursor(&transaction, revision, query, query_id, value)
            })
            .transpose()?;
        let effective_after = after.or(resolved_anchor_cursor.as_ref());
        let effective_anchor = if resolved_anchor_cursor.is_some() {
            None
        } else {
            anchor
        };
        let built =
            build_gallery_asset_query(query, effective_after, before, effective_anchor, sql_limit)?;
        let mut asset_statement = transaction.prepare(&built.sql).map_err(database_error)?;
        let mut asset_rows = asset_statement
            .query(params_from_iter(built.parameters.iter()))
            .map_err(database_error)?;
        let mut stored_assets = Vec::new();
        while let Some(row) = asset_rows.next().map_err(database_error)? {
            let stored = read_stored_asset(row).map_err(database_error)?;
            stored_assets.push(stored_asset_view(stored)?);
        }
        drop(asset_rows);
        drop(asset_statement);

        let has_more = stored_assets.len() > requested;
        stored_assets.truncate(requested);
        if before.is_some() {
            stored_assets.reverse();
        }
        let previous_cursor = if before.is_some() && has_more {
            stored_assets
                .first()
                .map(|asset| {
                    gallery_cursor_for_asset(&transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else if effective_after.is_some() || anchor.is_some() {
            stored_assets
                .first()
                .map(|asset| {
                    gallery_cursor_for_asset(&transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else {
            None
        };
        let next_cursor = if before.is_some() {
            stored_assets
                .last()
                .map(|asset| {
                    gallery_cursor_for_asset(&transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else if has_more {
            stored_assets
                .last()
                .map(|asset| {
                    gallery_cursor_for_asset(&transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else {
            None
        };
        let assets = stored_assets;

        transaction.commit().map_err(database_error)?;

        Ok(CatalogSnapshot {
            catalog_path,
            revision,
            query_id: query_id.to_owned(),
            roots,
            assets,
            previous_cursor,
            next_cursor,
            query_anchor_resolution: None,
        })
    }

    fn load_snapshot_around_location(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor_location_id: &str,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                format!(
                    "The catalog page limit must be between 1 and {MAX_CATALOG_PAGE_ITEMS} items"
                ),
            ));
        }
        validate_gallery_query(query)?;
        for _ in 0..3 {
            let transaction = self.connection.transaction().map_err(database_error)?;
            let revision = load_catalog_revision(&transaction)?;
            let (resolution, predecessor) = resolve_gallery_location_anchor(
                &transaction,
                revision,
                query,
                query_id,
                anchor_location_id,
                max_items,
            )?;
            transaction.commit().map_err(database_error)?;
            match self.load_snapshot(max_items, query, query_id, predecessor.as_ref(), None, None) {
                Ok(mut snapshot) if snapshot.revision == revision => {
                    snapshot.query_anchor_resolution = Some(resolution);
                    return Ok(snapshot);
                }
                Ok(_) => continue,
                Err(error) if error.code == "catalog_cursor_stale" => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ScanError::new(
            "catalog_cursor_stale",
            "The catalog kept changing while the gallery location anchor was resolved",
        ))
    }

    fn load_snapshot_around_asset(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        requested_location_id: &str,
        anchor_asset_id: &str,
        fallback_ordinal: u64,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                format!(
                    "The catalog page limit must be between 1 and {MAX_CATALOG_PAGE_ITEMS} items"
                ),
            ));
        }
        validate_gallery_query(query)?;
        for _ in 0..3 {
            let transaction = self.connection.transaction().map_err(database_error)?;
            let revision = load_catalog_revision(&transaction)?;
            let (resolution, predecessor) = resolve_gallery_asset_anchor(
                &transaction,
                revision,
                query,
                query_id,
                max_items,
                GalleryAssetAnchor {
                    requested_location_id,
                    asset_id: anchor_asset_id,
                    fallback_ordinal,
                },
            )?;
            transaction.commit().map_err(database_error)?;
            match self.load_snapshot(max_items, query, query_id, predecessor.as_ref(), None, None) {
                Ok(mut snapshot) if snapshot.revision == revision => {
                    snapshot.query_anchor_resolution = Some(resolution);
                    return Ok(snapshot);
                }
                Ok(_) => continue,
                Err(error) if error.code == "catalog_cursor_stale" => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ScanError::new(
            "catalog_cursor_stale",
            "The catalog kept changing while the gallery asset anchor was resolved",
        ))
    }

    fn load_gallery_timeline(
        &mut self,
        query: &GalleryQuery,
        query_id: &str,
    ) -> Result<GalleryTimeline, ScanError> {
        validate_gallery_query(query)?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let revision = load_catalog_revision(&transaction)?;
        let built = build_gallery_timeline_query(query);
        let mut statement = transaction.prepare(&built.sql).map_err(database_error)?;
        let rows = statement
            .query_map(params_from_iter(built.parameters.iter()), |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(database_error)?;
        let mut total_items = 0_u64;
        let mut buckets = Vec::new();
        for row in rows {
            let (month_key, item_count, aspect_ratio_milli_sum) = row.map_err(database_error)?;
            let item_count = sqlite_unsigned(item_count, "timeline bucket item count")?;
            let aspect_ratio_milli_sum =
                sqlite_unsigned(aspect_ratio_milli_sum, "timeline bucket aspect ratio sum")?;
            total_items = total_items.checked_add(item_count).ok_or_else(|| {
                ScanError::new(
                    "catalog_timeline_count_invalid",
                    "The gallery timeline item count exceeds the supported range",
                )
            })?;
            if !matches!(query.sort_key, GallerySortKey::FileName) {
                buckets.push(GalleryTimeBucket {
                    month_key,
                    item_count,
                    aspect_ratio_milli_sum,
                });
            }
        }
        drop(statement);
        transaction.commit().map_err(database_error)?;

        Ok(GalleryTimeline {
            revision,
            query_id: query_id.to_owned(),
            total_items,
            buckets,
        })
    }

    fn load_gallery_layout_manifest_chunk(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&GalleryLayoutManifestCursor>,
    ) -> Result<GalleryLayoutManifestChunk, ScanError> {
        if max_items == 0 || max_items > MAX_LAYOUT_MANIFEST_CHUNK_ITEMS {
            return Err(ScanError::new(
                "catalog_layout_chunk_limit_invalid",
                format!(
                    "The gallery layout chunk limit must be between 1 and \
                     {MAX_LAYOUT_MANIFEST_CHUNK_ITEMS} items"
                ),
            ));
        }
        validate_gallery_query(query)?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let revision = load_catalog_revision(&transaction)?;
        if after.is_some_and(|cursor| {
            cursor.revision != revision
                || cursor.query_id != query_id
                || cursor.after.revision != revision
                || cursor.after.query_id != query_id
                || cursor.next_ordinal > cursor.total_items
        }) {
            return Err(ScanError::new(
                "catalog_layout_cursor_stale",
                "The catalog or gallery query changed after this layout cursor was created",
            ));
        }

        let total_items = if let Some(cursor) = after {
            cursor.total_items
        } else {
            let count_query = build_gallery_count_query(query);
            let count = transaction
                .query_row(
                    &count_query.sql,
                    params_from_iter(count_query.parameters.iter()),
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            sqlite_unsigned(count, "gallery layout item count")?
        };
        let start_ordinal = after.map_or(0, |cursor| cursor.next_ordinal);
        let requested = usize::try_from(max_items).map_err(|_| {
            ScanError::new(
                "catalog_layout_chunk_limit_invalid",
                "The gallery layout chunk limit is outside the supported range",
            )
        })?;
        let built = build_gallery_layout_manifest_query(
            query,
            after.map(|cursor| &cursor.after),
            i64::from(max_items).saturating_add(1),
        )?;
        let mut statement = transaction.prepare(&built.sql).map_err(database_error)?;
        let mut rows = statement
            .query(params_from_iter(built.parameters.iter()))
            .map_err(database_error)?;
        let mut items = Vec::with_capacity(requested.saturating_add(1));
        while let Some(row) = rows.next().map_err(database_error)? {
            items.push(StoredLayoutManifestItem {
                location_id: row.get(0).map_err(database_error)?,
                root_id: row.get(1).map_err(database_error)?,
                width: sqlite_u32(row.get(2).map_err(database_error)?, "layout width")?,
                height: sqlite_u32(row.get(3).map_err(database_error)?, "layout height")?,
                date_key: row.get(4).map_err(database_error)?,
                primary_missing: row.get::<_, i64>(5).map_err(database_error)? != 0,
                primary_text: row.get(6).map_err(database_error)?,
                primary_number: row.get(7).map_err(database_error)?,
            });
        }
        drop(rows);
        drop(statement);

        let has_more = items.len() > requested;
        items.truncate(requested);
        let loaded_items = u64::try_from(items.len()).map_err(|_| {
            ScanError::new(
                "catalog_layout_count_invalid",
                "The gallery layout chunk exceeds the supported range",
            )
        })?;
        let next_ordinal = start_ordinal.checked_add(loaded_items).ok_or_else(|| {
            ScanError::new(
                "catalog_layout_count_invalid",
                "The gallery layout ordinal exceeds the supported range",
            )
        })?;
        if next_ordinal > total_items || (has_more && next_ordinal >= total_items) {
            return Err(ScanError::new(
                "catalog_layout_count_invalid",
                "The gallery layout cursor does not match the complete query count",
            ));
        }

        let next_cursor = if has_more {
            items.last().map(|item| GalleryLayoutManifestCursor {
                revision,
                query_id: query_id.to_owned(),
                total_items,
                next_ordinal,
                after: CatalogCursor {
                    revision,
                    query_id: query_id.to_owned(),
                    primary_missing: item.primary_missing,
                    primary_text: item.primary_text.clone(),
                    primary_number: item.primary_number,
                    root_id: item.root_id.clone(),
                    location_id: item.location_id.clone(),
                },
            })
        } else {
            None
        };

        let mut location_ids = Vec::with_capacity(items.len());
        let mut aspect_ratio_milli = Vec::with_capacity(items.len());
        let mut date_group_indices = Vec::with_capacity(items.len());
        let mut date_groups = Vec::new();
        let mut date_group_lookup = HashMap::new();
        let mut flags = Vec::with_capacity(items.len());
        for item in items {
            let has_dimensions = item.width > 0 && item.height > 0;
            let ratio = if has_dimensions {
                let scaled = u64::from(item.width)
                    .saturating_mul(1_000)
                    .checked_div(u64::from(item.height))
                    .unwrap_or(1_000)
                    .clamp(200, 5_000);
                u16::try_from(scaled).map_err(|_| {
                    ScanError::new(
                        "catalog_layout_ratio_invalid",
                        "The gallery layout aspect ratio exceeds the supported range",
                    )
                })?
            } else {
                1_000
            };
            let date_group_index = if let Some(index) = date_group_lookup.get(&item.date_key) {
                *index
            } else {
                let index = u16::try_from(date_groups.len()).map_err(|_| {
                    ScanError::new(
                        "catalog_layout_date_groups_invalid",
                        "The gallery layout chunk contains too many date groups",
                    )
                })?;
                date_groups.push(GalleryLayoutDateGroup {
                    date_key: item.date_key.clone(),
                });
                date_group_lookup.insert(item.date_key, index);
                index
            };
            location_ids.push(item.location_id);
            aspect_ratio_milli.push(ratio);
            date_group_indices.push(date_group_index);
            flags.push(if has_dimensions {
                LAYOUT_FLAG_DIMENSIONS_KNOWN
            } else {
                0
            });
        }

        transaction.commit().map_err(database_error)?;
        Ok(GalleryLayoutManifestChunk {
            revision,
            query_id: query_id.to_owned(),
            total_items,
            start_ordinal,
            location_ids,
            aspect_ratio_milli,
            date_group_indices,
            date_groups,
            flags,
            next_cursor,
        })
    }

    fn load_folder_page(
        &mut self,
        root_id: &str,
        parent_relative_path: &str,
        max_items: u32,
        after: Option<&LibraryFolderCursor>,
    ) -> Result<LibraryFolderPage, ScanError> {
        folders::load_folder_page(
            &mut self.connection,
            root_id,
            parent_relative_path,
            max_items,
            after,
        )
    }

    fn unregister_root(&mut self, root_id: &str) -> Result<bool, ScanError> {
        self.flush_pending_locations()?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let root_exists = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?1)",
                [root_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !root_exists {
            transaction.commit().map_err(database_error)?;
            return Ok(false);
        }
        retire_root_change_queue(&transaction, root_id, unix_time_ms())?;
        for table in [
            "scan_directory_frontier",
            "scan_directory_entries",
            "scan_issues",
        ] {
            transaction
                .execute(
                    &format!(
                        "DELETE FROM {table}
                         WHERE scan_id IN (
                           SELECT id FROM scan_runs WHERE root_id = ?1
                         )"
                    ),
                    [root_id],
                )
                .map_err(database_error)?;
        }
        detach_preview_references_for_root_locations(&transaction, root_id, None)?;
        mark_unreferenced_preview_artifacts_stale(&transaction)?;
        transaction
            .execute("DELETE FROM asset_locations WHERE root_id = ?1", [root_id])
            .map_err(database_error)?;
        transaction
            .execute("DELETE FROM scan_runs WHERE root_id = ?1", [root_id])
            .map_err(database_error)?;
        let removed = transaction
            .execute("DELETE FROM library_roots WHERE id = ?1", [root_id])
            .map_err(database_error)?;
        if removed != 1 {
            return Err(ScanError::new(
                "catalog_root_unregister_failed",
                "The registered library root could not be removed",
            ));
        }
        delete_orphan_assets(&transaction)?;
        let revision_updated = transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        if revision_updated != 1 {
            return Err(ScanError::new(
                "catalog_revision_unavailable",
                "The catalog revision state is missing or invalid",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }
}

fn load_catalog_revision(transaction: &Transaction<'_>) -> Result<u64, ScanError> {
    let revision = transaction
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(database_error)?;
    sqlite_unsigned(revision, "catalog revision")
}

fn load_root_views(transaction: &Transaction<'_>) -> Result<Vec<LibraryRootView>, ScanError> {
    let mut statement = transaction
        .prepare(
            "SELECT roots.id, roots.path, roots.active_scan_id, roots.created_unix_ms,
                    COALESCE(scans.asset_count, 0), COALESCE(scans.issue_count, 0)
             FROM library_roots AS roots
             LEFT JOIN scan_runs AS scans ON scans.id = roots.active_scan_id
             ORDER BY roots.created_unix_ms, roots.id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(database_error)?;
    let mut roots = Vec::new();
    for row in rows {
        let (root_id, path, active_scan_id, created_unix_ms, asset_count, issue_count) =
            row.map_err(database_error)?;
        roots.push(LibraryRootView {
            root_id,
            display_path: user_visible_path(&path),
            path,
            active_scan_id,
            created_unix_ms,
            asset_count: sqlite_unsigned(asset_count, "asset count")?,
            issue_count: sqlite_unsigned(issue_count, "issue count")?,
            availability: LibraryRootAvailability::Unknown,
            availability_message: None,
        });
    }
    Ok(roots)
}

fn normalize_relative_folder(value: &str) -> String {
    value.replace('\\', "/").trim_matches('/').to_owned()
}

fn parent_relative_path(relative_path: &str) -> String {
    let normalized = relative_path.replace('\\', "/");
    normalized
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_owned())
        .unwrap_or_default()
}

fn natural_name_key(relative_path: &str) -> String {
    let normalized = relative_path.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);
    let mut output = String::with_capacity(file_name.len());
    let mut characters = file_name.chars().peekable();
    while let Some(character) = characters.next() {
        if !character.is_ascii_digit() {
            output.extend(character.to_lowercase());
            continue;
        }
        let mut digits = String::from(character);
        while characters.peek().is_some_and(char::is_ascii_digit) {
            if let Some(digit) = characters.next() {
                digits.push(digit);
            }
        }
        let significant = digits.trim_start_matches('0');
        let significant = if significant.is_empty() {
            "0"
        } else {
            significant
        };
        output.push('\u{1e}');
        output.push_str(&format!(
            "{:04x}:{significant}:{:04x}",
            significant.len(),
            digits.len()
        ));
        output.push('\u{1f}');
    }
    output
}

fn persist_location(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    location: &AssetLocationView,
) -> Result<(), ScanError> {
    let file_size = sqlite_integer(location.file_size, "file size")?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO assets(id, created_unix_ms) VALUES (?1, ?2)",
            params![location.asset_id, unix_time_ms()],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               file_local_time, parent_relative_path, natural_name_key, width, height,
               preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               file_identity_scheme, file_identity_value
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                strftime(
                  '%Y-%m-%dT%H:%M:%f', COALESCE(?9, ?10) / 1000.0,
                  'unixepoch', 'localtime'
                ),
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25
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
               file_identity_value = excluded.file_identity_value",
            params![
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
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

struct StoredAssetRow {
    asset_id: String,
    location_id: String,
    root_id: String,
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
}

fn read_stored_asset(row: &Row<'_>) -> rusqlite::Result<StoredAssetRow> {
    Ok(StoredAssetRow {
        asset_id: row.get(0)?,
        location_id: row.get(1)?,
        root_id: row.get(2)?,
        absolute_path: row.get(3)?,
        relative_path: row.get(4)?,
        preview_path: row.get(5)?,
        file_size: row.get(6)?,
        created_unix_ms: row.get(7)?,
        modified_unix_ms: row.get(8)?,
        width: row.get(9)?,
        height: row.get(10)?,
        preview_status: row.get(11)?,
        preview_issue_code: row.get(12)?,
        preview_issue_message: row.get(13)?,
        metadata_engine_id: row.get(14)?,
        metadata_engine_version: row.get(15)?,
        capture_local_time: row.get(16)?,
        capture_offset_minutes: row.get(17)?,
        capture_time_source: row.get(18)?,
        capture_raw_value: row.get(19)?,
        file_identity_scheme: row.get(20)?,
        file_identity_value: row.get(21)?,
    })
}

fn stored_asset_view(stored: StoredAssetRow) -> Result<AssetLocationView, ScanError> {
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
        width: sqlite_u32(stored.width, "image width")?,
        height: sqlite_u32(stored.height, "image height")?,
        preview_status: parse_preview_status(&stored.preview_status)?,
        preview_issue_code: stored.preview_issue_code,
        preview_issue_message: stored.preview_issue_message,
        metadata_engine_id: stored.metadata_engine_id,
        metadata_engine_version: stored.metadata_engine_version,
        capture_time,
    })
}

fn stored_capture_time(
    local_time: Option<String>,
    offset_minutes: Option<i64>,
    source: Option<String>,
    raw_value: Option<String>,
) -> Result<Option<CaptureTimeEvidence>, ScanError> {
    match (local_time, source, raw_value) {
        (None, None, None) if offset_minutes.is_none() => Ok(None),
        (Some(local_time), Some(source), Some(raw_value)) => Ok(Some(CaptureTimeEvidence {
            local_time,
            offset_minutes: offset_minutes
                .map(|value| sqlite_i16(value, "capture time offset"))
                .transpose()?,
            source: parse_capture_time_source(&source)?,
            raw_value,
        })),
        _ => Err(ScanError::new(
            "catalog_capture_time_incomplete",
            "The stored capture-time evidence is incomplete",
        )),
    }
}

fn stored_file_identity(
    scheme: Option<String>,
    value: Option<String>,
) -> Result<Option<FileIdentityEvidence>, ScanError> {
    match (scheme, value) {
        (None, None) => Ok(None),
        (Some(scheme), Some(value)) => Ok(Some(FileIdentityEvidence { scheme, value })),
        _ => Err(ScanError::new(
            "catalog_file_identity_incomplete",
            "The stored file-identity evidence is incomplete",
        )),
    }
}

fn preview_status_text(status: &PreviewStatus) -> &'static str {
    match status {
        PreviewStatus::Pending => "pending",
        PreviewStatus::Ready => "ready",
        PreviewStatus::Failed => "failed",
    }
}

fn parse_preview_status(value: &str) -> Result<PreviewStatus, ScanError> {
    match value {
        "pending" => Ok(PreviewStatus::Pending),
        "ready" => Ok(PreviewStatus::Ready),
        "failed" => Ok(PreviewStatus::Failed),
        _ => Err(ScanError::new(
            "catalog_preview_status_invalid",
            format!("Unknown preview status {value}"),
        )),
    }
}

fn capture_time_source_text(source: &CaptureTimeSource) -> &'static str {
    match source {
        CaptureTimeSource::Original => "exif_original",
        CaptureTimeSource::Digitized => "exif_digitized",
        CaptureTimeSource::Image => "exif_datetime",
    }
}

fn parse_capture_time_source(value: &str) -> Result<CaptureTimeSource, ScanError> {
    match value {
        "exif_original" => Ok(CaptureTimeSource::Original),
        "exif_digitized" => Ok(CaptureTimeSource::Digitized),
        "exif_datetime" => Ok(CaptureTimeSource::Image),
        _ => Err(ScanError::new(
            "catalog_capture_time_source_invalid",
            format!("Unknown capture-time source {value}"),
        )),
    }
}

fn load_scan_with_status(
    connection: &Connection,
    status: &str,
    owner: ScanOwner,
) -> Result<Option<RecoverableScan>, ScanError> {
    let stored = connection
        .query_row(
            "SELECT scans.id, roots.path, scans.max_items, scans.max_entries,
                    scans.preview_edge, scans.visited_entries, scans.accepted_items,
                    scans.issue_count
             FROM scan_runs AS scans
             JOIN library_roots AS roots ON roots.id = scans.root_id
             WHERE scans.status = ?1 AND scans.scan_owner = ?2
             ORDER BY scans.started_unix_ms DESC, scans.id DESC
             LIMIT 1",
            params![status, owner.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    stored
        .map(
            |(
                scan_id,
                root_path,
                max_items,
                max_entries,
                preview_edge,
                visited_entries,
                accepted_items,
                issue_count,
            )| {
                Ok(RecoverableScan {
                    scan_id,
                    display_root_path: user_visible_path(&root_path),
                    root_path,
                    max_items: optional_sqlite_u32(max_items, "item limit")?,
                    max_entries: optional_sqlite_u32(max_entries, "entry limit")?,
                    preview_edge: sqlite_u32(preview_edge, "preview edge")?,
                    visited_entries: sqlite_unsigned(visited_entries, "visited entry count")?,
                    accepted_items: sqlite_unsigned(accepted_items, "accepted item count")?,
                    issue_count: sqlite_unsigned(issue_count, "issue count")?,
                })
            },
        )
        .transpose()
}

fn load_scans_with_status(
    connection: &Connection,
    status: &str,
    owner: ScanOwner,
    after_scan_id: Option<&str>,
    limit: u32,
) -> Result<Vec<RecoverableScan>, ScanError> {
    if !(1..=64).contains(&limit) {
        return Err(ScanError::new(
            "catalog_recoverable_scan_limit_invalid",
            "Recoverable scan queries require a limit between 1 and 64",
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT scans.id, roots.path, scans.max_items, scans.max_entries,
                    scans.preview_edge, scans.visited_entries, scans.accepted_items,
                    scans.issue_count
             FROM scan_runs AS scans
             JOIN library_roots AS roots ON roots.id = scans.root_id
             WHERE scans.status = ?1 AND scans.scan_owner = ?2
               AND (?3 IS NULL OR scans.id > ?3)
             ORDER BY scans.id
             LIMIT ?4",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            params![status, owner.as_str(), after_scan_id, i64::from(limit)],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )
        .map_err(database_error)?;
    let mut recoverable = Vec::new();
    for row in rows {
        let (
            scan_id,
            root_path,
            max_items,
            max_entries,
            preview_edge,
            visited_entries,
            accepted_items,
            issue_count,
        ) = row.map_err(database_error)?;
        recoverable.push(RecoverableScan {
            scan_id,
            display_root_path: user_visible_path(&root_path),
            root_path,
            max_items: optional_sqlite_u32(max_items, "item limit")?,
            max_entries: optional_sqlite_u32(max_entries, "entry limit")?,
            preview_edge: sqlite_u32(preview_edge, "preview edge")?,
            visited_entries: sqlite_unsigned(visited_entries, "visited entry count")?,
            accepted_items: sqlite_unsigned(accepted_items, "accepted item count")?,
            issue_count: sqlite_unsigned(issue_count, "issue count")?,
        });
    }
    Ok(recoverable)
}

fn database_error(error: rusqlite::Error) -> ScanError {
    if let rusqlite::Error::SqliteFailure(failure, _) = &error {
        match failure.code {
            rusqlite::ErrorCode::DatabaseBusy => {
                return ScanError::new(
                    "catalog_database_busy",
                    format!("The catalog database remained busy after waiting: {error}"),
                );
            }
            rusqlite::ErrorCode::DatabaseLocked => {
                return ScanError::new(
                    "catalog_database_locked",
                    format!("The catalog database is locked: {error}"),
                );
            }
            _ => {}
        }
    }
    ScanError::new("catalog_database_error", error.to_string())
}

fn delete_orphan_assets(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM assets
             WHERE NOT EXISTS (
               SELECT 1 FROM asset_locations WHERE asset_locations.asset_id = assets.id
             )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn detach_preview_references_for_root_locations(
    transaction: &Transaction<'_>,
    root_id: &str,
    retained_scan_id: Option<&str>,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM preview_artifact_locations
             WHERE location_id IN (
               SELECT locations.location_id
               FROM asset_locations AS locations
               WHERE locations.root_id = ?1
                 AND (?2 IS NULL OR locations.scan_id <> ?2)
             )",
            params![root_id, retained_scan_id],
        )
        .map_err(database_error)?;
    Ok(())
}

fn mark_unreferenced_preview_artifacts_stale(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE preview_artifacts
             SET lifecycle_state = 'stale'
             WHERE lifecycle_state = 'ready'
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifact_locations AS owners
                 WHERE owners.artifact_key = preview_artifacts.artifact_key
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn sqlite_integer(value: u64, field: &str) -> Result<i64, ScanError> {
    i64::try_from(value).map_err(|_| {
        ScanError::new(
            "catalog_integer_overflow",
            format!("The {field} exceeds the SQLite integer range"),
        )
    })
}

fn sqlite_unsigned(value: i64, field: &str) -> Result<u64, ScanError> {
    u64::try_from(value).map_err(|_| {
        ScanError::new(
            "catalog_integer_invalid",
            format!("The stored {field} is outside the supported range"),
        )
    })
}

fn sqlite_u32(value: i64, field: &str) -> Result<u32, ScanError> {
    u32::try_from(value).map_err(|_| {
        ScanError::new(
            "catalog_integer_invalid",
            format!("The stored {field} is outside the supported range"),
        )
    })
}

fn sqlite_i16(value: i64, field: &str) -> Result<i16, ScanError> {
    i16::try_from(value).map_err(|_| {
        ScanError::new(
            "catalog_integer_invalid",
            format!("The stored {field} is outside the supported range"),
        )
    })
}

fn optional_sqlite_u32(value: Option<i64>, field: &str) -> Result<Option<u32>, ScanError> {
    value.map(|value| sqlite_u32(value, field)).transpose()
}

fn unix_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
